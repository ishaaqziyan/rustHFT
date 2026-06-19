use hft_core::{ExecutionReport, MarketEvent, Order, OrderBookSnapshot, StrategyCommand, Tick};
use strategy::{StatArbStrategy, StatSummary};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct DashboardUpdate {
    pub tick: Option<Tick>,
    pub orderbook: Option<OrderBookSnapshot>,
    pub stat: Option<StatSummary>,
    pub pnl: f64,
    pub position: i32,
    pub new_order: Option<Order>,
}

pub struct Engine {
    ui_sender: mpsc::Sender<DashboardUpdate>,
    command_rx: mpsc::Receiver<StrategyCommand>,
}

impl Engine {
    pub fn new(
        ui_sender: mpsc::Sender<DashboardUpdate>,
        command_rx: mpsc::Receiver<StrategyCommand>,
    ) -> Self {
        Self {
            ui_sender,
            command_rx,
        }
    }

    pub async fn run(mut self) {
        let (event_tx, mut event_rx) = mpsc::channel(1024);

        tokio::spawn(async move {
            exchange::webhook::WebhookServer::start().await;
        });

        let mut strategy = StatArbStrategy::new(100, "BTCUSDT".to_string(), "ETHUSDT".to_string());

        let mut poll_interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
        let yahoo = exchange::yahoo::YahooClient::new();

        let mut ws_task: Option<tokio::task::JoinHandle<()>> = None;
        let mut api_key_id = std::env::var("APCA_API_KEY_ID").unwrap_or_default();
        let mut api_secret = std::env::var("APCA_API_SECRET_KEY").unwrap_or_default();
        let binance_key = std::env::var("BINANCE_API_KEY").unwrap_or_default();
        let binance_secret = std::env::var("BINANCE_API_SECRET").unwrap_or_default();

        // Start initial WS if crypto
        if strategy.symbol_a.ends_with("USDT") {
            let tx = event_tx.clone();
            let syms = vec![strategy.symbol_a.clone(), strategy.symbol_b.clone()];
            ws_task = Some(tokio::spawn(async move {
                exchange::binance_ws::start_binance_ws(syms, tx).await;
            }));
        }

        // Local state to simulate orderbook for the UI since we only fetch top-of-book prices
        let mut last_price_a = 0.0;

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    let sym_a = strategy.symbol_a.clone();
                    let sym_b = strategy.symbol_b.clone();
                    let tx = event_tx.clone();

                    let is_crypto = sym_a.ends_with("USDT");

                    if !is_crypto && api_key_id.is_empty() {
                        // Fallback to Yahoo if Alpaca keys are not provided
                        if let Ok(ticks) = yahoo.fetch_latest_quotes(&[&sym_a, &sym_b]).await {
                            for tick in ticks {
                                if tick.symbol == sym_a { last_price_a = tick.price; }
                                let _ = tx.send(MarketEvent::Tick(tick)).await;
                            }
                        }

                        // Simulate OrderBook for Yahoo polling
                        if last_price_a > 0.0 {
                            use hft_core::{OrderBookSnapshot, OrderBookLevel};
                            let ob = OrderBookSnapshot {
                                symbol: sym_a.clone(),
                                bids: vec![
                                    OrderBookLevel { price: last_price_a - 0.01, qty: 150.0 },
                                    OrderBookLevel { price: last_price_a - 0.05, qty: 300.0 },
                                ],
                                asks: vec![
                                    OrderBookLevel { price: last_price_a + 0.01, qty: 120.0 },
                                    OrderBookLevel { price: last_price_a + 0.05, qty: 250.0 },
                                ],
                            };
                            let _ = tx.send(MarketEvent::OrderBook(ob)).await;
                        }
                    }
                }
                // Handle UI commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        StrategyCommand::KillSwitch => {
                            strategy.is_active = !strategy.is_active;
                        }
                        StrategyCommand::UpdateApiKeys(key, secret) => {
                            api_key_id = key.clone();
                            api_secret = secret.clone();

                            // If not crypto, restart WS for Alpaca
                            if !strategy.symbol_a.ends_with("USDT") && !key.is_empty() && !secret.is_empty() {
                                if let Some(task) = ws_task.take() {
                                    task.abort();
                                }
                                let tx = event_tx.clone();
                                let syms = vec![strategy.symbol_a.clone(), strategy.symbol_b.clone()];
                                ws_task = Some(tokio::spawn(async move {
                                    exchange::alpaca_ws::start_alpaca_ws(key, secret, syms, tx).await;
                                }));
                            }
                        }
                        StrategyCommand::ChangePair(a, b) => {
                            strategy.symbol_a = a.clone();
                            strategy.symbol_b = b.clone();
                            strategy.position = 0; // Reset position on pair change
                            strategy.pnl = 0.0;
                            last_price_a = 0.0;

                            // Manage WebSocket task
                            if let Some(task) = ws_task.take() {
                                task.abort();
                            }

                            if a.ends_with("USDT") {
                                let tx = event_tx.clone();
                                let syms = vec![a, b];
                                ws_task = Some(tokio::spawn(async move {
                                    exchange::binance_ws::start_binance_ws(syms, tx).await;
                                }));
                            } else if !api_key_id.is_empty() && !api_secret.is_empty() {
                                let tx = event_tx.clone();
                                let syms = vec![a, b];
                                let key = api_key_id.clone();
                                let sec = api_secret.clone();
                                ws_task = Some(tokio::spawn(async move {
                                    exchange::alpaca_ws::start_alpaca_ws(key, sec, syms, tx).await;
                                }));
                            }
                        }
                        _ => {}
                    }
                }

                // Handle market events
                Some(event) = event_rx.recv() => {
                    let mut tick = None;
                    let mut orderbook = None;
                    let mut stat = None;
                    let mut commands = vec![];

                    match &event {
                        MarketEvent::Tick(t) => {
                            let (s, cmds) = strategy.on_tick(t);
                            stat = s;
                            commands = cmds;
                            tick = Some(t.clone());
                        }
                        MarketEvent::OrderBook(ob) => {
                            orderbook = Some(ob.clone());
                        }
                    }

                    // Process strategy commands
                    let mut sent_order = None;
                    for cmd in commands {
                        if let StrategyCommand::SubmitOrder(order) = cmd {
                            sent_order = Some(order.clone());

                            // Forward to Alpaca Paper API if keys exist and not crypto
                            if !api_key_id.is_empty() && !api_secret.is_empty() && !order.symbol.ends_with("USDT") {
                                let alpaca = exchange::alpaca::AlpacaClient::new(api_key_id.clone(), api_secret.clone());
                                let side_str = if order.side == hft_core::Side::Buy { "buy" } else { "sell" };
                                let symbol = order.symbol.clone();
                                let qty = order.qty;

                                tokio::spawn(async move {
                                    let _ = alpaca.place_order(&symbol, qty, side_str).await;
                                });
                            }

                            // Forward to Binance Testnet API if keys exist and IS crypto
                            if !binance_key.is_empty() && !binance_secret.is_empty() && order.symbol.ends_with("USDT") {
                                let binance = exchange::binance::BinanceClient::new(binance_key.clone(), binance_secret.clone());
                                let side_str = if order.side == hft_core::Side::Buy { "buy" } else { "sell" };
                                let symbol = order.symbol.clone();
                                let qty = order.qty;

                                tokio::spawn(async move {
                                    let _ = binance.place_order(&symbol, qty, side_str).await;
                                });
                            }

                            // Simulate local execution for PnL
                            let exec = ExecutionReport {
                                order_id: order.id,
                                symbol: order.symbol,
                                side: order.side,
                                executed_qty: order.qty,
                                executed_price: order.price,
                            };
                            strategy.on_execution(&exec);
                        }
                    }

                    let update = DashboardUpdate {
                        tick,
                        orderbook,
                        stat,
                        pnl: strategy.pnl,
                        position: strategy.position,
                        new_order: sent_order,
                    };

                    // Send to UI without blocking
                    let _ = self.ui_sender.try_send(update);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_update_creation() {
        let update = DashboardUpdate {
            tick: None,
            orderbook: None,
            stat: None,
            pnl: 100.0,
            position: 1,
            new_order: None,
        };
        assert_eq!(update.pnl, 100.0);
        assert_eq!(update.position, 1);
        assert!(update.tick.is_none());
    }

    #[test]
    fn test_engine_creation() {
        let (ui_tx, _ui_rx) = mpsc::channel(10);
        let (_cmd_tx, cmd_rx) = mpsc::channel(10);
        let _engine = Engine::new(ui_tx, cmd_rx);

        // Just verify it doesn't crash on instantiation
        // We cannot easily test run() without a full tokio setup and mocking
        // the external connections like Binance and Alpaca.
    }
}
