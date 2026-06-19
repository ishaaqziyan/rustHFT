#![allow(non_snake_case)]
use dioxus::prelude::*;
use engine::{Engine, DashboardUpdate};
use hft_core::{OrderBookSnapshot, StrategyCommand, Side};
use futures_util::StreamExt;
use tokio::sync::mpsc;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut ticks = use_signal(|| Vec::<String>::new());
    let mut rog_ob = use_signal(|| None::<OrderBookSnapshot>);
    let mut orders = use_signal(|| Vec::<String>::new());
    let mut pnl = use_signal(|| 0.0);
    let mut position = use_signal(|| 0);
    let mut z_score = use_signal(|| 0.0);
    let mut beta = use_signal(|| 2.8);
    let mut is_active = use_signal(|| true);

    // Settings
    let mut symbol_a = use_signal(|| "BTCUSDT".to_string());
    let mut symbol_b = use_signal(|| "ETHUSDT".to_string());

    let ui_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<StrategyCommand>| async move {
        let (update_tx, mut update_rx) = mpsc::channel::<DashboardUpdate>(1024);
        let (cmd_tx, cmd_rx) = mpsc::channel::<StrategyCommand>(100);
        
        let engine = Engine::new(update_tx, cmd_rx);
        
        tokio::spawn(async move {
            engine.run().await;
        });

        loop {
            tokio::select! {
                Some(cmd) = rx.next() => {
                    let _ = cmd_tx.send(cmd).await;
                }
                Some(update) = update_rx.recv() => {
                    if let Some(tick) = update.tick {
                        let tick_str = format!("{} | {} | Price: {:.2} | Vol: {:.2}", 
                            tick.timestamp.format("%H:%M:%S%.3f"),
                            tick.symbol,
                            tick.price,
                            tick.volume
                        );
                        ticks.with_mut(|t| {
                            t.push(tick_str);
                            if t.len() > 10 { t.remove(0); }
                        });
                    }

                    if let Some(ob) = update.orderbook {
                        // Display the orderbook for symbol_a
                        if ob.symbol == symbol_a() {
                            rog_ob.set(Some(ob));
                        }
                    }

                    if let Some(order) = update.new_order {
                        let side_str = if order.side == Side::Buy { "BUY " } else { "SELL" };
                        let order_str = format!("Order #{} | {} {} | Qty: {:.2} @ {:.2}", 
                            order.id, side_str, order.symbol, order.qty, order.price
                        );
                        orders.with_mut(|o| {
                            o.push(order_str);
                            if o.len() > 10 { o.remove(0); }
                        });
                    }

                    pnl.set(update.pnl);
                    position.set(update.position);
                    if let Some(stat) = update.stat {
                        z_score.set(stat.z_score);
                        beta.set(stat.beta);
                    }
                }
            }
        }
    });

    rsx! {
        div {
            style: "padding: 20px; font-family: 'Inter', sans-serif; background-color: #0d1117; color: #e6edf3; height: 100vh; overflow: hidden; display: flex; flex-direction: column;",
            
            // Header
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0;",
                h1 { style: "color: #58a6ff; margin: 0;", "Universal StatArb Engine (Live)" }
                
                div {
                    style: "display: flex; gap: 10px; align-items: center; background: #161b22; padding: 10px; border-radius: 8px; border: 1px solid #30363d;",
                    span { style: "color: #3fb950; font-size: 14px; font-weight: bold;", "🔒 Secrets Managed by Doppler" }
                }

                button {
                    style: format!("padding: 10px 20px; font-weight: bold; font-size: 16px; border-radius: 6px; border: none; cursor: pointer; color: white; background-color: {};", if is_active() { "#da3633" } else { "#238636" }),
                    onclick: move |_| {
                        is_active.set(!is_active());
                        ui_coroutine.send(StrategyCommand::KillSwitch);
                    },
                    if is_active() { "KILL SWITCH (HALT)" } else { "SYSTEM HALTED (RESUME)" }
                }
            }
            
            div {
                style: "display: flex; gap: 20px; flex: 1; min-height: 0;",
                
                // Left Panel: Ticks & Executions
                div {
                    style: "flex: 1; display: flex; flex-direction: column; gap: 20px;",
                    div {
                        style: "flex: 1; background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px; display: flex; flex-direction: column;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "Market Data Feed" }
                        div {
                            style: "font-family: monospace; font-size: 13px; overflow-y: auto; flex: 1;",
                            for tick in ticks.read().iter().rev() {
                                div { style: "padding: 4px; border-bottom: 1px solid #21262d;", "{tick}" }
                            }
                        }
                    }
                    div {
                        style: "flex: 1; background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px; display: flex; flex-direction: column;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "Execution Log" }
                        div {
                            style: "font-family: monospace; font-size: 13px; overflow-y: auto; flex: 1;",
                            for order in orders.read().iter().rev() {
                                div { 
                                    style: format!("padding: 4px; border-bottom: 1px solid #21262d; color: {};", if order.contains("BUY") { "#3fb950" } else { "#f85149" }), 
                                    "{order}" 
                                }
                            }
                        }
                    }
                }

                // Middle Panel: Order Book
                div {
                    style: "flex: 1.5; display: flex; flex-direction: column; gap: 20px;",
                    div {
                        style: "flex: 1; background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px; display: flex; flex-direction: column;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "Limit Order Book ({symbol_a})" }
                        div {
                            style: "font-family: monospace; font-size: 13px; display: flex; flex-direction: column; flex: 1;",
                            // Asks (Sells) - Red
                            div {
                                style: "display: flex; flex-direction: column-reverse; flex: 1; justify-content: flex-end; border-bottom: 1px solid #444; padding-bottom: 5px; margin-bottom: 5px;",
                                if let Some(ob) = rog_ob.read().as_ref() {
                                    for ask in ob.asks.iter().rev() {
                                        div {
                                            style: "display: flex; justify-content: space-between; color: #f85149; padding: 2px 0;",
                                            span { "{ask.price:.2}" }
                                            span { "{ask.qty:.0}" }
                                        }
                                    }
                                }
                            }
                            // Bids (Buys) - Green
                            div {
                                style: "display: flex; flex-direction: column; flex: 1;",
                                if let Some(ob) = rog_ob.read().as_ref() {
                                    for bid in ob.bids.iter() {
                                        div {
                                            style: "display: flex; justify-content: space-between; color: #3fb950; padding: 2px 0;",
                                            span { "{bid.price:.2}" }
                                            span { "{bid.qty:.0}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Panel: Strategy Stats & Config
                div {
                    style: "flex: 1; display: flex; flex-direction: column; gap: 20px;",
                    
                    div {
                        style: "background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "Pair Configuration" }
                        div {
                            style: "display: flex; flex-direction: column; gap: 10px;",
                            select {
                                style: "width: 100%; padding: 8px; background: #0d1117; color: white; border: 1px solid #30363d; border-radius: 4px;",
                                onchange: move |e| {
                                    let selected = e.value();
                                    let parts: Vec<&str> = selected.split('-').collect();
                                    if parts.len() == 2 {
                                        symbol_a.set(parts[0].to_string());
                                        symbol_b.set(parts[1].to_string());
                                        rog_ob.set(None);
                                        ui_coroutine.send(StrategyCommand::ChangePair(parts[0].to_string(), parts[1].to_string()));
                                    }
                                },
                                option { value: "BTCUSDT-ETHUSDT", "Crypto: Bitcoin (BTC) vs Ethereum (ETH)" }
                                option { value: "SOLUSDT-ADAUSDT", "Crypto: Solana (SOL) vs Cardano (ADA)" }
                                option { value: "AAPL-MSFT", "US Stocks: Apple (AAPL) vs Microsoft (MSFT)" }
                                option { value: "GOOGL-META", "US Stocks: Alphabet (GOOGL) vs Meta (META)" }
                                option { value: "ROG.SW-NESN.SW", "Swiss Stocks: Roche (ROG) vs Nestle (NESN)" }
                            }
                            span { style: "font-size: 12px; color: #8b949e;", "Active Pair: {symbol_a} vs {symbol_b}" }
                        }
                    }

                    div {
                        style: "background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "Live PnL & Exposure" }
                        h1 { 
                            style: if *pnl.read() >= 0.0 { "color: #3fb950; margin: 10px 0; font-size: 42px;" } else { "color: #f85149; margin: 10px 0; font-size: 42px;" },
                            "CHF {pnl.read():.2}" 
                        }
                        p { style: "font-size: 18px;", "Net Position (Spread Lots): {position.read()}" }
                    }

                    div {
                        style: "background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 15px;",
                        h3 { style: "color: #8b949e; margin-top: 0;", "OLS Co-integration Model" }
                        div {
                            style: "margin-top: 10px;",
                            p { style: "font-family: monospace; font-size: 14px;", "Model: {symbol_a} = β * {symbol_b} + ε" }
                            p { style: "font-family: monospace; font-size: 14px;", "Dynamic Beta (β): {beta():.4}" }
                            div {
                                style: "display: flex; align-items: center; gap: 10px; margin-top: 20px;",
                                span { "Z-Score:" }
                                div {
                                    style: "flex: 1; height: 10px; background: #30363d; border-radius: 5px; position: relative;",
                                    div {
                                        style: "position: absolute; left: 50%; height: 10px; width: 2px; background: #8b949e;",
                                    }
                                    div {
                                        style: format!("position: absolute; top: -3px; width: 16px; height: 16px; border-radius: 50%; background: {}; left: calc(50% + ({} * 10%)); transition: left 0.1s;", 
                                            if z_score() > 2.0 { "#f85149" } else if z_score() < -2.0 { "#3fb950" } else { "#58a6ff" },
                                            (z_score() as f64).clamp(-5.0, 5.0)
                                        ),
                                    }
                                }
                                span { style: "font-family: monospace; width: 50px; text-align: right;", "{z_score():.2}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
