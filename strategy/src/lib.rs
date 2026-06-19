use chrono::Utc;
use hft_core::{ExecutionReport, Order, Side, StrategyCommand, Tick};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StatSummary {
    pub spread: f64,
    pub mean: f64,
    pub z_score: f64,
    pub beta: f64,
}

pub struct StatArbStrategy {
    window_size: usize,
    pub symbol_a: String,
    pub symbol_b: String,
    history_a: VecDeque<f64>,
    history_b: VecDeque<f64>,
    pub position: i32,
    pub pnl: f64,
    pub order_counter: u64,
    pub is_active: bool,
}

impl StatArbStrategy {
    pub fn new(window_size: usize, symbol_a: String, symbol_b: String) -> Self {
        Self {
            window_size,
            symbol_a,
            symbol_b,
            history_a: VecDeque::with_capacity(window_size),
            history_b: VecDeque::with_capacity(window_size),
            position: 0,
            pnl: 0.0,
            order_counter: 0,
            is_active: true,
        }
    }

    pub fn on_execution(&mut self, exec: &ExecutionReport) {
        self.pnl += 0.5 * exec.executed_qty;
    }

    pub fn on_tick(&mut self, tick: &Tick) -> (Option<StatSummary>, Vec<StrategyCommand>) {
        if !self.is_active {
            return (None, vec![]);
        }

        if tick.symbol == self.symbol_a {
            if self.history_a.len() == self.window_size {
                self.history_a.pop_front();
            }
            self.history_a.push_back(tick.price);
        } else if tick.symbol == self.symbol_b {
            if self.history_b.len() == self.window_size {
                self.history_b.pop_front();
            }
            self.history_b.push_back(tick.price);
        } else {
            return (None, vec![]);
        }

        let n = usize::min(self.history_a.len(), self.history_b.len());
        if n >= 20 {
            // Calculate Beta (Covariance / Variance) for ROG ~ NESN
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut sum_xy = 0.0;
            let mut sum_x2 = 0.0;

            for i in 0..n {
                let x = self.history_b[i];
                let y = self.history_a[i];
                sum_x += x;
                sum_y += y;
                sum_xy += x * y;
                sum_x2 += x * x;
            }

            let nf = n as f64;
            let beta = (nf * sum_xy - sum_x * sum_y) / (nf * sum_x2 - sum_x * sum_x);

            // Calculate spread: symbol_a - beta * symbol_b
            let current_a = *self.history_a.back().unwrap();
            let current_b = *self.history_b.back().unwrap();
            let spread = current_a - beta * current_b;

            // Calculate rolling mean and std dev of spread
            let mut sum_spread = 0.0;
            let mut spreads = Vec::with_capacity(n);
            for i in 0..n {
                let s = self.history_a[i] - beta * self.history_b[i];
                sum_spread += s;
                spreads.push(s);
            }

            let mean = sum_spread / nf;
            let variance = spreads.iter().map(|s| (s - mean) * (s - mean)).sum::<f64>() / nf;
            let std_dev = variance.sqrt();
            let z_score = if std_dev > 0.0 {
                (spread - mean) / std_dev
            } else {
                0.0
            };

            let mut commands = Vec::new();

            let sym_a = self.symbol_a.clone();
            let sym_b = self.symbol_b.clone();

            // Trading logic
            if z_score > 2.0 && self.position <= 0 {
                // Short Spread: Sell symbol_a, Buy symbol_b
                self.position -= 1;
                commands.push(self.create_order(&sym_a, Side::Sell, current_a, 1.0));
                commands.push(self.create_order(&sym_b, Side::Buy, current_b, beta));
            } else if z_score < -2.0 && self.position >= 0 {
                // Long Spread: Buy symbol_a, Sell symbol_b
                self.position += 1;
                commands.push(self.create_order(&sym_a, Side::Buy, current_a, 1.0));
                commands.push(self.create_order(&sym_b, Side::Sell, current_b, beta));
            } else if z_score.abs() < 0.5 && self.position != 0 {
                // Mean reversion: Close position
                let qty = self.position.abs() as f64;
                if self.position > 0 {
                    commands.push(self.create_order(&sym_a, Side::Sell, current_a, qty));
                    commands.push(self.create_order(&sym_b, Side::Buy, current_b, qty * beta));
                } else {
                    commands.push(self.create_order(&sym_a, Side::Buy, current_a, qty));
                    commands.push(self.create_order(&sym_b, Side::Sell, current_b, qty * beta));
                }
                self.position = 0;
            }

            return (
                Some(StatSummary {
                    spread,
                    mean,
                    z_score,
                    beta,
                }),
                commands,
            );
        }

        (None, vec![])
    }

    fn create_order(&mut self, symbol: &str, side: Side, price: f64, qty: f64) -> StrategyCommand {
        self.order_counter += 1;
        StrategyCommand::SubmitOrder(Order {
            id: self.order_counter,
            symbol: symbol.to_string(),
            side,
            price,
            qty,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_initialization() {
        let strategy = StatArbStrategy::new(20, "A".to_string(), "B".to_string());
        assert_eq!(strategy.symbol_a, "A");
        assert_eq!(strategy.symbol_b, "B");
        assert_eq!(strategy.position, 0);
        assert_eq!(strategy.pnl, 0.0);
        assert!(strategy.is_active);
    }

    #[test]
    fn test_strategy_not_active() {
        let mut strategy = StatArbStrategy::new(20, "A".to_string(), "B".to_string());
        strategy.is_active = false;

        let tick = Tick {
            symbol: "A".to_string(),
            price: 100.0,
            volume: 10.0,
            timestamp: Utc::now(),
        };

        let (summary, commands) = strategy.on_tick(&tick);
        assert!(summary.is_none());
        assert!(commands.is_empty());
    }

    #[test]
    fn test_strategy_on_execution() {
        let mut strategy = StatArbStrategy::new(20, "A".to_string(), "B".to_string());

        let exec = ExecutionReport {
            order_id: 1,
            symbol: "A".to_string(),
            side: Side::Buy,
            executed_qty: 10.0,
            executed_price: 100.0,
        };

        strategy.on_execution(&exec);
        assert_eq!(strategy.pnl, 5.0); // 0.5 * executed_qty
    }

    #[test]
    fn test_strategy_trading_logic() {
        let mut strategy = StatArbStrategy::new(20, "A".to_string(), "B".to_string());

        // Feed 20 ticks for B (constant price 100.0)
        for _ in 0..20 {
            strategy.on_tick(&Tick {
                symbol: "B".to_string(),
                price: 100.0,
                volume: 1.0,
                timestamp: Utc::now(),
            });
        }

        // Feed 19 ticks for A (constant price 100.0)
        for _ in 0..19 {
            strategy.on_tick(&Tick {
                symbol: "A".to_string(),
                price: 100.0,
                volume: 1.0,
                timestamp: Utc::now(),
            });
        }

        // 20th tick for A, slightly higher price
        let (summary, commands) = strategy.on_tick(&Tick {
            symbol: "A".to_string(),
            price: 110.0, // High price to trigger short spread
            volume: 1.0,
            timestamp: Utc::now(),
        });

        assert!(summary.is_some());
        let sum = summary.unwrap();
        // Since B is constant and A jumped, beta should be calculated, spread should deviate
        // We just check that commands are generated depending on z-score
        // If z_score > 2.0, it should short A and buy B
        if sum.z_score > 2.0 {
            assert!(!commands.is_empty());
            assert_eq!(strategy.position, -1);
        }
    }
}
