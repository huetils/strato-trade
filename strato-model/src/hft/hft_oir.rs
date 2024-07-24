use chrono::Utc;
use tracing::debug;

pub const DEFAULT_K: usize = 5;
pub const DEFAULT_Q: f64 = 0.15;
pub const TRADE_SIZE: f64 = 0.001;
pub const DEFAULT_SPREAD_THRESHOLD: f64 = 0.05; // Adjust based on backtesting and performance analysis
pub const DEFAULT_TRANSACTION_COST: f64 = 0.005; // 0.5%

pub enum Side {
    Buy,
    Sell,
}

// Struct to hold the trading state
#[derive(Debug)]
pub struct TradingState {
    pub cash: f64,
    pub positions: Vec<f64>,
    pub symbol: &'static str,
    pub voi_history: Vec<f64>,
    pub oir_history: Vec<f64>,
    pub mpb_history: Vec<f64>,
}

impl TradingState {
    pub fn new(cash: f64, symbol: &'static str) -> Self {
        Self {
            cash,
            positions: Vec::new(),
            symbol,
            voi_history: Vec::new(),
            oir_history: Vec::new(),
            mpb_history: Vec::new(),
        }
    }

    /// Calculates the Volume Order Imbalance (VOI).
    ///
    /// VOI is calculated as the difference between the bid volume and the ask volume.
    /// This metric helps in understanding the imbalance between buy and sell orders in the order book.
    ///
    /// # Arguments
    ///
    /// * `bid_volume` - Total volume of buy orders.
    /// * `ask_volume` - Total volume of sell orders.
    ///
    /// # Returns
    ///
    /// * `voi` - Volume Order Imbalance.
    pub fn calculate_voi(bid_volume: f64, ask_volume: f64) -> f64 {
        bid_volume - ask_volume
    }

    /// Calculates the Order Imbalance Ratio (OIR).
    ///
    /// OIR is calculated as the normalized difference between the bid volume and the ask volume.
    /// This ratio is used to quantify the relative imbalance between buy and sell orders.
    ///
    /// # Arguments
    ///
    /// * `bid_volume` - Total volume of buy orders.
    /// * `ask_volume` - Total volume of sell orders.
    ///
    /// # Returns
    ///
    /// * `oir` - Order Imbalance Ratio.
    pub fn calculate_oir(bid_volume: f64, ask_volume: f64) -> f64 {
        (bid_volume - ask_volume) / (bid_volume + ask_volume)
    }

    /// Calculates the Mid-Price Basis (MPB).
    ///
    /// MPB is calculated as the difference between the last traded price and the mid-price.
    /// This metric indicates the deviation of the last trade price from the mid-price.
    ///
    /// # Arguments
    ///
    /// * `last_price` - Last traded price.
    /// * `mid_price` - Mid-price of the current bid-ask spread.
    ///
    /// # Returns
    ///
    /// * `mpb` - Mid-Price Basis.
    pub fn calculate_mpb(last_price: f64, mid_price: f64) -> f64 {
        last_price - mid_price
    }

    /// Calculates the bid-ask spread as a percentage of the bid price.
    ///
    /// This metric helps in understanding the relative size of the spread compared to the bid price.
    ///
    /// # Arguments
    ///
    /// * `bid` - Current bid price.
    /// * `ask` - Current ask price.
    ///
    /// # Returns
    ///
    /// * `spread` - Bid-ask spread percentage.
    pub fn calculate_spread(bid: f64, ask: f64) -> f64 {
        (ask - bid) / bid * 100.0
    }

    /// Implements the Parametrized Linear Model for trading decisions.
    ///
    /// This model uses a weighted sum of the historical values of VOI, OIR, and MPB to make trading decisions.
    /// A buy signal is generated if the weighted sum exceeds the positive threshold `q`.
    /// A sell signal is generated if the weighted sum falls below the negative threshold `-q`.
    ///
    /// # Arguments
    ///
    /// * `current_voi` - Current VOI value.
    /// * `current_oir` - Current OIR value.
    /// * `current_mpb` - Current MPB value.
    /// * `k` - Number of historical values to consider (window size).
    /// * `q` - Threshold for decision making.
    ///
    /// # Returns
    ///
    /// * `signal` - Trading signal (1.0 for buy, -1.0 for sell, 0.0 for hold).
    pub fn parametrized_linear_model(
        &mut self,
        current_voi: f64,
        current_oir: f64,
        current_mpb: f64,
        k: usize,
        q: f64,
    ) -> f64 {
        // Update history
        self.voi_history.push(current_voi);
        self.oir_history.push(current_oir);
        self.mpb_history.push(current_mpb);

        // Keep history size to k
        if self.voi_history.len() > k {
            self.voi_history.remove(0);
        }
        if self.oir_history.len() > k {
            self.oir_history.remove(0);
        }
        if self.mpb_history.len() > k {
            self.mpb_history.remove(0);
        }

        // Calculate the weighted sum of VOI, OIR, and MPB
        let weighted_sum: f64 = self.voi_history.iter().sum::<f64>()
            + self.oir_history.iter().sum::<f64>()
            + self.mpb_history.iter().sum::<f64>();

        // Decision based on weighted sum and threshold q
        if weighted_sum > q {
            // Buy signal
            return 1.0;
        } else if weighted_sum < -q {
            // Sell signal
            return -1.0;
        }

        // No trade signal
        0.0
    }

    /// Executes a trade based on the provided price and side.
    ///
    /// This function updates the cash balance and position size based on the trade details.
    ///
    /// # Arguments
    ///
    /// * `price` - Trade price.
    /// * `side` - Trade side (Buy or Sell).
    /// * `trade_size` - Size of the trade.
    /// * `fee` - Transaction fee percentage.
    pub fn execute_trade(&mut self, price: f64, side: Side, trade_size: f64, fee: f64) {
        let transaction_cost = trade_size * price * fee;
        match side {
            Side::Buy => {
                self.positions.push(price);
                self.cash -= price * trade_size + transaction_cost;
                debug!(
                    "Buying {} {} at {} (cost: {}) at {}",
                    trade_size,
                    self.symbol,
                    price,
                    transaction_cost,
                    Utc::now()
                );
            }
            Side::Sell => {
                if let Some(_position) = self.positions.pop() {
                    self.cash += price * trade_size - transaction_cost;
                    debug!(
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
    }
}
