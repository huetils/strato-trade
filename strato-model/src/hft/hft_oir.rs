/// Implementation of an order imbalance based strategy by Darryl Shen
use barter_data::subscription::book::OrderBook;
use chrono::Utc;
use tracing::info;

pub const TRADE_SIZE: f64 = 0.001;
pub const SPREAD_THRESHOLD: f64 = 0.05;
pub const TRANSACTION_COST: f64 = 0.005; // 0.5%
pub const BUY_OIR_THRESHOLD: f64 = 0.1;
pub const SELL_OIR_THRESHOLD: f64 = -0.1;
pub const BUY_MPB_THRESHOLD: f64 = 0.1;
pub const SELL_MPB_THRESHOLD: f64 = -0.1;

#[derive(Debug)]
pub struct TradingState {
    pub cash: f64,
    pub positions: Vec<f64>,
    pub symbol: &'static str,
}

impl TradingState {
    pub fn new(cash: f64, symbol: &'static str) -> Self {
        Self {
            cash,
            positions: Vec::new(),
            symbol,
        }
    }

    pub fn calculate_voi(order_book: &OrderBook) -> (f64, f64, f64) {
        let bid_volume: f64 = order_book.bids.levels.iter().map(|bid| bid.amount).sum();
        let ask_volume: f64 = order_book.asks.levels.iter().map(|ask| ask.amount).sum();
        let voi: f64 = bid_volume - ask_volume;
        (voi, bid_volume, ask_volume)
    }

    pub fn calculate_oir(bid_volume: f64, ask_volume: f64) -> f64 {
        (bid_volume - ask_volume) / (bid_volume + ask_volume)
    }

    pub fn calculate_mpb(last_price: f64, mid_price: f64) -> f64 {
        last_price - mid_price
    }

    pub fn calculate_spread(bid: f64, ask: f64) -> f64 {
        (ask - bid) / bid * 100.0
    }

    pub fn should_trade(spread: f64, voi: f64, spread_threshold: f64) -> bool {
        spread <= spread_threshold && voi.abs() > 0.0
    }

    pub fn execute_trade(&mut self, price: f64, side: &str, trade_size: f64, fee: f64) {
        let transaction_cost = trade_size * price * fee;
        if side == "buy" {
            self.positions.push(price);
            self.cash -= price * trade_size + transaction_cost;
            info!(
                "Buying {} {} at {} (cost: {}) at {}",
                trade_size,
                self.symbol,
                price,
                transaction_cost,
                Utc::now()
            );
        } else if side == "sell" {
            if let Some(_position) = self.positions.pop() {
                self.cash += price * trade_size - transaction_cost;
                info!(
                    "Selling {} {} at {} (cost: {}) at {}",
                    trade_size,
                    self.symbol,
                    price,
                    transaction_cost,
                    Utc::now()
                );
            }
        }
    }

    pub fn check_sell_conditions(&mut self, ask: f64, voi: f64, oir: f64, mpb: f64) {
        if voi < 0.0
            && oir < SELL_OIR_THRESHOLD
            && mpb < SELL_MPB_THRESHOLD
            && !self.positions.is_empty()
        {
            info!(
                "Selling triggered by conditions: VOI: {}, OIR: {}, MPB: {} at ask price: {}",
                voi, oir, mpb, ask
            );
            self.execute_trade(ask, "sell", TRADE_SIZE, TRANSACTION_COST);
        }
    }

    pub fn check_buy_conditions(&mut self, bid: f64, voi: f64, oir: f64, mpb: f64) {
        if voi > 0.0 && oir > BUY_OIR_THRESHOLD && mpb > BUY_MPB_THRESHOLD {
            info!(
                "Buying triggered by conditions: VOI: {}, OIR: {}, MPB: {} at bid price: {}",
                voi, oir, mpb, bid
            );
            self.execute_trade(bid, "buy", TRADE_SIZE, TRANSACTION_COST);
        }
    }

    pub fn calculate_portfolio_value(&self, bid: f64) -> f64 {
        let position_value: f64 = self.positions.len() as f64 * TRADE_SIZE * bid;
        self.cash + position_value
    }
}

#[cfg(test)]
mod tests {
    use barter_data::subscription::book::Level;
    use barter_data::subscription::book::OrderBookSide;
    use barter_integration::model::Side;
    use chrono::DateTime;

    use super::*;

    // Constants for tests
    const TEST_TRADE_SIZE: f64 = 0.001;
    const TEST_TRANSACTION_COST: f64 = 0.005;
    const TEST_SPREAD_THRESHOLD: f64 = 0.05;

    #[test]
    fn test_calculate_voi() {
        let order_book = OrderBook {
            last_update_time: DateTime::from_timestamp_millis(0).unwrap(),
            bids: OrderBookSide::new(
                Side::Buy,
                vec![Level {
                    price: 100.0,
                    amount: 1.0,
                }],
            ),
            asks: OrderBookSide::new(
                Side::Sell,
                vec![Level {
                    price: 101.0,
                    amount: 1.0,
                }],
            ),
        };

        let (voi, bid_volume, ask_volume) = TradingState::calculate_voi(&order_book);
        assert_eq!(voi, 0.0);
        assert_eq!(bid_volume, 1.0);
        assert_eq!(ask_volume, 1.0);
    }

    #[test]
    fn test_calculate_oir() {
        let oir = TradingState::calculate_oir(1.0, 1.0);
        assert_eq!(oir, 0.0);
    }

    #[test]
    fn test_calculate_mpb() {
        let mpb = TradingState::calculate_mpb(100.0, 100.0);
        assert_eq!(mpb, 0.0);
    }

    #[test]
    fn test_calculate_spread() {
        let spread = TradingState::calculate_spread(100.0, 101.0);
        assert_eq!(spread, 1.0);
    }

    #[test]
    fn test_should_trade() {
        let valid_voi = 1.0;
        assert!(TradingState::should_trade(
            TEST_SPREAD_THRESHOLD,
            valid_voi,
            TEST_SPREAD_THRESHOLD
        ));

        let invalid_spread = 0.06;
        let valid_voi = 1.0;
        assert!(!TradingState::should_trade(
            invalid_spread,
            valid_voi,
            TEST_SPREAD_THRESHOLD
        ));

        let invalid_voi = 0.0;
        assert!(!TradingState::should_trade(
            TEST_SPREAD_THRESHOLD,
            invalid_voi,
            TEST_SPREAD_THRESHOLD
        ));
    }

    #[test]
    fn test_execute_trade() {
        let mut state = TradingState::new(1000.0, "BTC/USDT");
        state.execute_trade(100.0, "buy", TEST_TRADE_SIZE, TEST_TRANSACTION_COST);
        let expected_cash_after_buy =
            1000.0 - (100.0 * TEST_TRADE_SIZE) - (100.0 * TEST_TRADE_SIZE * TEST_TRANSACTION_COST);
        assert_eq!(state.cash, expected_cash_after_buy);
        assert_eq!(state.positions.len(), 1);

        state.execute_trade(100.0, "sell", TEST_TRADE_SIZE, TEST_TRANSACTION_COST);
        let expected_cash_after_sell = expected_cash_after_buy + (100.0 * TEST_TRADE_SIZE)
            - (100.0 * TEST_TRADE_SIZE * TEST_TRANSACTION_COST);
        assert_eq!(state.cash, expected_cash_after_sell);
        assert_eq!(state.positions.len(), 0);
    }

    #[test]
    fn test_calculate_portfolio_value() {
        let mut state = TradingState::new(1000.0, "BTC/USDT");
        state.positions.push(100.0);
        let portfolio_value = state.calculate_portfolio_value(101.0);
        let expected_portfolio_value = 1000.0 + (101.0 * TEST_TRADE_SIZE);
        assert_eq!(portfolio_value, expected_portfolio_value);
    }
}
