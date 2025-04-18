use std::fmt::Debug;

use chrono::Utc;
use hftbacktest::prelude::*;
use tracing::debug;
use tracing::error;

/// The number of historical values (window size) to consider in the model. This
/// parameter determines the depth of the historical data used to calculate the
/// weighted sum of VOI, OIR, and MPB. According to the study, a window size of
/// 5 provides a balance between responsiveness and stability in trading
/// signals.
pub const DEFAULT_K: usize = 5;

/// The threshold value for decision making in the model. This parameter sets
/// the sensitivity of the trading signals by defining the boundary for buy and
/// sell decisions. The study suggests a threshold value of 0.15, which ensures
/// that trading signals are generated only when there is a significant combined
/// effect of VOI, OIR, and MPB.
pub const DEFAULT_Q: f64 = 0.15;

/// Future implementation for live trading
// fn exec_live_trading() {}

pub fn exec_backtest_hft_oir<MD, I, R>(
    hbt: &mut I,
    recorder: &mut R,
    order_qty: f64,
) -> anyhow::Result<(), anyhow::Error>
where
    MD: L2MarketDepth + MarketDepth,
    I: Bot<MD>,
    <I as Bot<MD>>::Error: Debug,
    R: Recorder,
    <R as Recorder>::Error: Debug,
{
    let mut int = 0;
    let mut trading_state = TradingState::new();

    // 100ms
    while hbt.elapse(100_000_000).unwrap() {
        int += 1;
        if int % 10 == 0 {
            // Records every 1-sec
            recorder.record(hbt).unwrap();
        }

        // --- Generate signal from trading strategy ---
        let depth = hbt.depth(0);

        let last_price = 0.0; // Get from market feed or historical data
        let mid_price = (depth.best_bid() + depth.best_ask()) / 2.0;

        let bid_volume = 0.0;
        let ask_volume = 0.0;

        let current_voi = TradingState::calculate_voi(bid_volume, ask_volume);
        let current_oir = TradingState::calculate_oir(bid_volume, ask_volume);
        let current_mpb = TradingState::calculate_mpb(last_price, mid_price);

        let signal = trading_state.parametrized_linear_model(
            current_voi,
            current_oir,
            current_mpb,
            Some(DEFAULT_K),
            Some(DEFAULT_Q),
        );
        // ---

        let asset_no = 0;
        let order_id = 0;
        let price = last_price;
        let time_in_force = TimeInForce::FOK; // Could prevent any order from being executed
        let order_type = OrdType::Market;
        let wait = true;
        let mut result = false;

        // Use the signal to open a position. We might have to close any current
        // position before opening a new one that is if the current position is
        // the opposite of the signal
        if signal == 1.0 {
            result = hbt
                .submit_buy_order(
                    asset_no,
                    order_id,
                    price,
                    order_qty,
                    time_in_force,
                    order_type,
                    wait,
                )
                .expect("Failed to submit buy order");
        } else if signal == -1.0 {
            result = hbt
                .submit_sell_order(
                    asset_no,
                    order_id,
                    price,
                    order_qty,
                    time_in_force,
                    order_type,
                    wait,
                )
                .expect("Failed to submit sell order");
        }

        if !result {
            error!("Failed to submit order");
        }
    }

    Ok(())
}

pub enum Side {
    Buy,
    Sell,
}

// Struct to hold the trading state
#[derive(Debug, Default)]
pub struct TradingState {
    pub positions: Vec<f64>,
    pub voi_history: Vec<f64>,
    pub oir_history: Vec<f64>,
    pub mpb_history: Vec<f64>,
}

impl TradingState {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            voi_history: Vec::new(),
            oir_history: Vec::new(),
            mpb_history: Vec::new(),
        }
    }

    /// Calculates a smoothed price using a weighted average of the last traded
    /// price and the mid-price.
    ///
    /// This method combines the latest trade data with the mid-price for a more
    /// stable price indicator.
    ///
    /// # Arguments
    ///
    /// * `last_price` - Last traded price.
    /// * `mid_price` - Mid-price of the bid-ask spread.
    /// * `weight_last_price` - Weight for the last traded price (0.0 to 1.0).
    ///
    /// # Returns
    ///
    /// * `smoothed_price` - Smoothed price.
    pub fn calculate_smoothed_price(
        last_price: f64,
        mid_price: f64,
        weight_last_price: f64,
    ) -> f64 {
        let weight_mid_price = 1.0 - weight_last_price;
        weight_last_price * last_price + weight_mid_price * mid_price
    }

    /// Calculates an exponentially weighted moving average (EWMA) of the price.
    ///
    /// This method gives more weight to the most recent price while
    /// incorporating the previous smoothed price.
    ///
    /// # Arguments
    ///
    /// * `last_price` - Last traded price.
    /// * `prev_ewma_price` - Previous EWMA price.
    /// * `alpha` - Smoothing factor (0.0 to 1.0).
    ///
    /// # Returns
    ///
    /// * `ewma_price` - EWMA price.
    pub fn calculate_ewma_price(last_price: f64, prev_ewma_price: f64, alpha: f64) -> f64 {
        alpha * last_price + (1.0 - alpha) * prev_ewma_price
    }

    /// Calculates the Volume Order Imbalance (VOI).
    ///
    /// VOI is calculated as the difference between the bid volume and the ask
    /// volume. This metric helps in understanding the imbalance between buy
    /// and sell orders in the order book.
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
    /// OIR is calculated as the normalized difference between the bid volume
    /// and the ask volume. This ratio is used to quantify the relative
    /// imbalance between buy and sell orders.
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
    /// MPB is calculated as the difference between the last traded price and
    /// the mid-price. This metric indicates the deviation of the last trade
    /// price from the mid-price.
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
    /// This metric helps in understanding the relative size of the spread
    /// compared to the bid price.
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

    /// Calculates the mid-price as the average of the bid and ask prices.
    ///
    /// This metric provides a reference price point that is used to calculate
    /// the Mid-Price Basis (MPB).
    ///
    /// # Arguments
    ///
    /// * `bid` - Current bid price.
    /// * `ask` - Current ask price.
    ///
    /// # Returns
    ///
    /// * `mid_price` - Mid-price of the bid-ask spread.
    pub fn calculate_mid_price(bid: f64, ask: f64) -> f64 {
        (bid + ask) / 2.0
    }

    /// Implements the Parametrized Linear Model for trading decisions.
    ///
    /// This model uses a weighted sum of the historical values of VOI, OIR, and
    /// MPB to make trading decisions. A buy signal is generated if the
    /// weighted sum exceeds the positive threshold `q`. A sell signal is
    /// generated if the weighted sum falls below the negative threshold `-q`.
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
        k: Option<usize>,
        q: Option<f64>,
    ) -> f64 {
        let k = k.unwrap_or(DEFAULT_K);
        let q = q.unwrap_or(DEFAULT_Q);

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

    /// Ensure the spread is within the acceptable threshold
    ///
    /// A wide spread may indicate lower liquidity or higher uncertainty in the
    /// market, leading to more expensive trades and potentially lower
    /// profits.
    ///
    /// # Arguments
    ///
    /// * `spread` - Current bid-ask spread percentage.
    /// * `spread_threshold` - Maximum acceptable spread percentage.
    ///
    /// # Returns
    ///
    /// * `is_threshold_constrained` - Boolean indicating if the spread is
    ///   within the threshold.
    pub fn is_threshold_constrained(spread: f64, spread_threshold: f64) -> bool {
        spread <= spread_threshold
    }

    /// Including the `voi.abs() > 0.0` check ensures that trades are only
    /// considered when there is a significant volume order imbalance (VOI).
    ///
    /// **Usage**
    ///
    /// - VOI represents the difference between bid and ask volumes. A non-zero
    ///   VOI indicates a discrepancy between supply and demand, which can
    ///   signal potential price movements.
    /// - If `voi.abs()` is greater than 0, it implies that there is a
    ///   meaningful imbalance in the order book, making it a more reliable
    ///   signal for trading decisions.
    ///
    /// **Avoiding Noise**
    ///
    /// - By ensuring `voi.abs() > 0.0`, the strategy avoids acting on
    ///   insignificant imbalances that may not lead to meaningful price
    ///   movements. This helps in filtering out noise and focusing on more
    ///   substantial trading opportunities.
    ///
    /// **Reinforcing Trading Signals**
    ///
    /// - Combining the spread threshold check with the VOI condition reinforces
    ///   the reliability of the trading signals. It ensures that trades are
    ///   only executed when both the market spread is favorable, and there is a
    ///   significant order imbalance, thereby improving the accuracy of the
    ///   strategy.
    ///
    /// # Arguments
    ///
    /// * `voi` - Volume Order Imbalance.
    ///
    /// # Returns
    ///
    /// * `is_voi_detected` - Boolean indicating if a significant VOI is
    ///   detected.
    pub fn is_voi_detected(voi: f64) -> bool {
        voi.abs() > 0.0
    }

    /// Executes a trade based on the provided price and side.
    ///
    /// This function updates the cash balance and position size based on the
    /// trade details.
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
                debug!(
                    "Buying {} at {} (cost: {}) at {}",
                    trade_size,
                    price,
                    transaction_cost,
                    Utc::now()
                );
            }
            Side::Sell => {
                if let Some(_position) = self.positions.pop() {
                    debug!(
                        "Sell {} at {} (cost: {}) at {}",
                        trade_size,
                        price,
                        transaction_cost,
                        Utc::now()
                    );
                }
            }
        }
    }
}
