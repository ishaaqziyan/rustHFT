use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Tick {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub qty: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Tick(Tick),
    OrderBook(OrderBookSnapshot),
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub symbol: String,
    pub side: Side,
    pub price: f64,
    pub qty: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub order_id: u64,
    pub symbol: String,
    pub side: Side,
    pub executed_qty: f64,
    pub executed_price: f64,
}

#[derive(Debug, Clone)]
pub enum StrategyCommand {
    SubmitOrder(Order),
    KillSwitch,
    UpdateApiKeys(String, String), // key, secret
    ChangePair(String, String),    // symbol_a, symbol_b
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_tick_creation() {
        let now = Utc::now();
        let tick = Tick {
            symbol: "BTCUSDT".to_string(),
            price: 50000.0,
            volume: 1.5,
            timestamp: now,
        };
        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.price, 50000.0);
        assert_eq!(tick.volume, 1.5);
    }

    #[test]
    fn test_order_creation() {
        let now = Utc::now();
        let order = Order {
            id: 1,
            symbol: "ETHUSDT".to_string(),
            side: Side::Buy,
            price: 3000.0,
            qty: 2.0,
            timestamp: now,
        };
        assert_eq!(order.id, 1);
        assert_eq!(order.side, Side::Buy);
    }

    #[test]
    fn test_market_event_enum() {
        let now = Utc::now();
        let tick = Tick {
            symbol: "SOLUSDT".to_string(),
            price: 150.0,
            volume: 10.0,
            timestamp: now,
        };
        let event = MarketEvent::Tick(tick);
        match event {
            MarketEvent::Tick(t) => assert_eq!(t.symbol, "SOLUSDT"),
            _ => panic!("Expected Tick event"),
        }
    }
}
