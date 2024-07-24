/*!
This module provides functionality for generating grid levels for a trading strategy
based on the Rolling Moving Average (RMA) and Average True Range (ATR) of the market data. The grid levels
are used to define premium and discount levels for making trading decisions.

The module relies on utility functions from the `strato_utils` crate for calculating the
RMA (Rolling Moving Average) and ATR (Average True Range).
*/

use strato_utils::ta::atr::atr;
use strato_utils::ta::rma::rma;
use strato_utils::ta::sma::sma;
use strato_utils::vars::ohlc::Ohlc;

const DEFAULT_MA_LEN: usize = 100;
const DEFAULT_ATR_LEN: usize = 14;
const DEFAULT_BAND_MULT: f64 = 2.5;

pub enum MaType {
    Rma,
    Sma,
}

pub enum GridLogic {
    Atr,
    Percent,
}

pub struct TradingState {
    pub balance: f64,
    pub position: f64,
}

/// Parameters for configuring the grid trading strategy.
pub struct GridParams {
    /// Length of the Rolling Moving Average (RMA) period.
    pub ma_len: usize,
    /// Type of Moving Average (e.g., RMA, SMA, etc.)
    pub ma_type: MaType,
    /// Grid Logic (e.g., ATR, Percent)
    pub grid_logic: GridLogic,
    /// Multiplier for the ATR to determine grid levels.
    pub band_mult: f64,
    /// Length of the Average True Range (ATR) period.
    pub atr_len: usize,
}

impl Default for GridParams {
    fn default() -> Self {
        GridParams {
            ma_len: DEFAULT_MA_LEN,
            ma_type: MaType::Rma,
            grid_logic: GridLogic::Atr,
            band_mult: DEFAULT_BAND_MULT,
            atr_len: DEFAULT_ATR_LEN,
        }
    }
}

/// Generates the premium and discount grid levels based on the provided ohlc and parameters.
///
/// This function calculates the Rolling Moving Average (RMA) and Average True Range (ATR)
/// of the market data, and then uses these values to generate the grid levels.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
/// * `params` - A reference to `GridParams` struct containing the parameters for the grid.
///
/// # Returns
///
/// A tuple containing two vectors:
/// - `premium_levels`: The calculated premium levels.
/// - `discount_levels`: The calculated discount levels.
pub fn generate_grid_levels(ohlc: &[Ohlc], params: &GridParams) -> (Vec<f64>, Vec<f64>) {
    let src = calculate_src(&ohlc);
    let ma_values = match params.ma_type {
        MaType::Sma => sma(&src, params.ma_len),
        MaType::Rma => rma(&src, params.ma_len),
    };
    let atr_values = atr(&ohlc, params.atr_len);
    calculate_grid_levels(&ma_values, &atr_values, params.band_mult)
}

/// Calculates the source prices from the provided ohlc.
///
/// The source price is calculated as the average of the open, high, low, and close prices.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
///
/// # Returns
///
/// A vector of source prices.
pub fn calculate_src(ohlc: &[Ohlc]) -> Vec<f64> {
    ohlc.iter()
        .map(|c| (c.open + c.high + c.low + c.close) / 4.0)
        .collect()
}

/// Calculates the premium and discount grid levels based on RMA and ATR values.
///
/// This function uses the Rolling Moving Average (RMA) and Average True Range (ATR)
/// to determine the grid levels for trading. The premium levels are calculated by adding
/// multiples of the ATR to the RMA, and the discount levels are calculated by subtracting
/// multiples of the ATR from the RMA.
///
/// # Arguments
///
/// * `rma` - A slice of RMA values.
/// * `atr` - A slice of ATR values.
/// * `band_mult` - The multiplier for the ATR to determine grid levels.
/// * `grid_level_index` - The number of grid levels.
///
/// # Returns
///
/// A tuple containing two vectors:
/// - `premium_levels`: The calculated premium levels.
/// - `discount_levels`: The calculated discount levels.
pub fn calculate_grid_levels(rma: &[f64], atr: &[f64], band_mult: f64) -> (Vec<f64>, Vec<f64>) {
    let mut premium_levels = vec![0.0; rma.len()];
    let mut discount_levels = vec![0.0; rma.len()];

    for i in 0..rma.len() {
        premium_levels[i] = rma[i] + atr[i] * band_mult;
        discount_levels[i] = rma[i] - atr[i] * band_mult;
    }

    (premium_levels, discount_levels)
}

/// Checks entry conditions based on the discount levels.
///
/// The entry condition is met when the low price of the ohlc is below the discount level.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
/// * `discount_levels` - A slice of discount levels.
///
/// # Returns
///
/// A vector of boolean values indicating whether the entry condition is met for each ohlc.
pub fn check_entry_conditions(ohlc: &[Ohlc], discount_levels: &[f64]) -> Vec<bool> {
    ohlc.iter()
        .zip(discount_levels.iter())
        .map(|(c, &d)| c.low < d)
        .collect()
}

/// Checks exit conditions based on the premium levels.
///
/// The exit condition is met when the high price of the ohlc is above the premium level.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
/// * `premium_levels` - A slice of premium levels.
///
/// # Returns
///
/// A vector of boolean values indicating whether the exit condition is met for each ohlc.
pub fn check_exit_conditions(ohlc: &[Ohlc], premium_levels: &[f64]) -> Vec<bool> {
    ohlc.iter()
        .zip(premium_levels.iter())
        .map(|(c, &p)| c.high > p)
        .collect()
}

/// Manages the grids based on the calculated grid levels and entry/exit conditions.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
/// * `params` - A reference to `GridParams` struct containing the parameters for the grid.
///
/// # Returns
///
/// A tuple containing vectors of boolean values indicating whether the entry or exit condition is met for each ohlc.
pub fn manage_grids(ohlc: &[Ohlc], params: &GridParams) -> (Vec<bool>, Vec<bool>) {
    let (premium_levels, discount_levels) = generate_grid_levels(ohlc, params);
    let entry_conditions = check_entry_conditions(ohlc, &discount_levels);
    let exit_conditions = check_exit_conditions(ohlc, &premium_levels);

    (entry_conditions, exit_conditions)
}

/// Executes trades based on the entry and exit conditions.
///
/// # Arguments
///
/// * `ohlc` - A slice of `Ohlc` structs representing market data.
/// * `entry_conditions` - A vector of boolean values indicating whether the entry condition is met for each ohlc.
/// * `exit_conditions` - A vector of boolean values indicating whether the exit condition is met for each ohlc.
/// * `initial_balance` - The initial balance for the trading account.
///
/// # Returns
///
/// The final balance after executing the trades.
pub fn execute_trades(
    ohlc: &[Ohlc],
    entry_conditions: &[bool],
    exit_conditions: &[bool],
    initial_balance: f64,
) -> f64 {
    let mut state = TradingState {
        balance: initial_balance,
        position: 0.0,
    };

    for i in 0..ohlc.len() {
        if entry_conditions[i] {
            handle_entry(&mut state, ohlc[i].close);
        } else if exit_conditions[i] {
            handle_exit(&mut state, ohlc[i].close);
        }
    }

    finalize_balance(&mut state, ohlc.last().unwrap().close);

    state.balance
}

/// Handles trade entry.
///
/// # Arguments
///
/// * `state` - The current trading state.
/// * `price` - The current price of the asset.
pub fn handle_entry(state: &mut TradingState, price: f64) {
    if state.position == 0.0 {
        state.position = state.balance / price;
        state.balance = 0.0;
    }
}

/// Handles trade exit.
///
/// # Arguments
///
/// * `state` - The current trading state.
/// * `price` - The current price of the asset.
pub fn handle_exit(state: &mut TradingState, price: f64) {
    if state.position > 0.0 {
        state.balance = state.position * price;
        state.position = 0.0;
    }
}

/// Finalizes the balance at the end of the trading period.
///
/// # Arguments
///
/// * `state` - The current trading state.
/// * `price` - The final price of the asset.
pub fn finalize_balance(state: &mut TradingState, price: f64) {
    if state.position > 0.0 {
        state.balance = state.position * price;
        state.position = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strato_utils::vars::ohlc::Ohlc;

    #[test]
    fn test_calculate_src() {
        let ohlc = vec![
            Ohlc {
                open: 100.0,
                high: 110.0,
                low: 90.0,
                close: 105.0,
                ..Default::default()
            },
            Ohlc {
                open: 105.0,
                high: 115.0,
                low: 95.0,
                close: 100.0,
                ..Default::default()
            },
        ];

        let expected_src = vec![101.25, 103.75];
        let src = calculate_src(&ohlc);
        assert_eq!(src, expected_src);
    }

    #[test]
    fn test_calculate_grid_levels() {
        let rma = vec![100.0, 105.0];
        let atr = vec![5.0, 10.0];
        let band_mult = 2.5;

        let (premium_levels, discount_levels) = calculate_grid_levels(&rma, &atr, band_mult);

        let expected_premium_levels = vec![112.5, 130.0];
        let expected_discount_levels = vec![87.5, 80.0];

        assert_eq!(premium_levels, expected_premium_levels);
        assert_eq!(discount_levels, expected_discount_levels);
    }

    #[test]
    fn test_generate_grid_levels() {
        let ohlc = vec![
            Ohlc {
                open: 100.0,
                high: 110.0,
                low: 90.0,
                close: 105.0,
                ..Default::default()
            },
            Ohlc {
                open: 105.0,
                high: 115.0,
                low: 95.0,
                close: 100.0,
                ..Default::default()
            },
        ];

        let params = GridParams::default();

        let (premium_levels, discount_levels) = generate_grid_levels(&ohlc, &params);

        assert_eq!(premium_levels.len(), ohlc.len());
        assert_eq!(discount_levels.len(), ohlc.len());
    }
}
