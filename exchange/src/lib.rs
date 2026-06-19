pub mod webhook;
pub mod yahoo;
pub mod binance;
pub mod binance_ws;
pub mod alpaca_ws;
pub mod alpaca;

use chrono::Utc;
use hft_core::{MarketEvent, OrderBookLevel, OrderBookSnapshot, Tick};
use rand_distr::{Distribution, Normal};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

pub struct DummyExchange {
    event_sender: mpsc::Sender<MarketEvent>,
}

impl DummyExchange {
    pub fn new(event_sender: mpsc::Sender<MarketEvent>) -> Self {
        Self { event_sender }
    }

    fn generate_orderbook(symbol: &str, mid_price: f64) -> OrderBookSnapshot {
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        let spread = mid_price * 0.0005; // 0.05% spread

        for i in 1..=5 {
            let offset = spread * (i as f64);
            bids.push(OrderBookLevel {
                price: mid_price - offset,
                qty: 100.0 * (i as f64) + {
                    let mut rng = rand::thread_rng();
                    Normal::<f64>::new(0.0, 10.0)
                        .unwrap()
                        .sample(&mut rng)
                        .abs()
                },
            });
            asks.push(OrderBookLevel {
                price: mid_price + offset,
                qty: 100.0 * (i as f64) + {
                    let mut rng = rand::thread_rng();
                    Normal::<f64>::new(0.0, 10.0)
                        .unwrap()
                        .sample(&mut rng)
                        .abs()
                },
            });
        }

        OrderBookSnapshot {
            symbol: symbol.to_string(),
            bids,
            asks,
        }
    }

    pub async fn start(&self) {
        let normal_dist = Normal::new(0.0, 0.1).unwrap();

        // Starting prices for Swiss dummy stocks
        let mut nesn_price = 100.0; // Nestle
        let mut rog_price = 280.0; // Roche
        let mut novn_price = 90.0; // Novartis

        loop {
            let market_move = {
                let mut rng = rand::thread_rng();
                normal_dist.sample(&mut rng)
            };

            // Individual noise
            nesn_price += market_move + {
                let mut rng = rand::thread_rng();
                normal_dist.sample(&mut rng) * 0.5
            };
            rog_price += market_move * 2.8 + {
                let mut rng = rand::thread_rng();
                normal_dist.sample(&mut rng) * 1.0
            };
            novn_price += {
                let mut rng = rand::thread_rng();
                normal_dist.sample(&mut rng)
            };

            // Create ticks
            let t1 = Tick {
                symbol: "NESN".to_string(),
                price: nesn_price,
                volume: 100.0,
                timestamp: Utc::now(),
            };
            let t2 = Tick {
                symbol: "ROG".to_string(),
                price: rog_price,
                volume: 50.0,
                timestamp: Utc::now(),
            };
            let t3 = Tick {
                symbol: "NOVN".to_string(),
                price: novn_price,
                volume: 150.0,
                timestamp: Utc::now(),
            };

            if self.event_sender.send(MarketEvent::Tick(t1)).await.is_err() {
                break;
            }
            if self.event_sender.send(MarketEvent::Tick(t2)).await.is_err() {
                break;
            }
            if self.event_sender.send(MarketEvent::Tick(t3)).await.is_err() {
                break;
            }

            // Create orderbooks
            let ob1 = Self::generate_orderbook("NESN", nesn_price);
            let ob2 = Self::generate_orderbook("ROG", rog_price);
            let ob3 = Self::generate_orderbook("NOVN", novn_price);

            if self
                .event_sender
                .send(MarketEvent::OrderBook(ob1))
                .await
                .is_err()
            {
                break;
            }
            if self
                .event_sender
                .send(MarketEvent::OrderBook(ob2))
                .await
                .is_err()
            {
                break;
            }
            if self
                .event_sender
                .send(MarketEvent::OrderBook(ob3))
                .await
                .is_err()
            {
                break;
            }

            // Sleep to simulate high frequency updates (e.g. 50ms)
            sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_orderbook() {
        let ob = DummyExchange::generate_orderbook("TEST", 100.0);
        assert_eq!(ob.symbol, "TEST");
        assert_eq!(ob.bids.len(), 5);
        assert_eq!(ob.asks.len(), 5);
        
        // Check prices
        assert!(ob.bids[0].price < 100.0);
        assert!(ob.asks[0].price > 100.0);
    }

    #[tokio::test]
    async fn test_dummy_exchange_creation() {
        let (tx, mut rx) = mpsc::channel(10);
        let exchange = DummyExchange::new(tx);
        
        // Start in background and let it run for a short bit
        let handle = tokio::spawn(async move {
            exchange.start().await;
        });
        
        // Wait for first event
        let event = rx.recv().await;
        assert!(event.is_some());
        
        // Abort task
        handle.abort();
    }
}
