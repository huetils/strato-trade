use crate::ta::rma::rma;
use crate::vars::ohlc::Ohlc;

/// https://www.tradingview.com/pine-script-reference/v5/#fun_ta.atr
pub fn true_range(candles: &[Ohlc]) -> Vec<f64> {
    let mut tr = vec![0.0; candles.len()];

    for i in 1..candles.len() {
        let high_low = candles[i].high - candles[i].low;
        let high_close = (candles[i].high - candles[i - 1].close).abs();
        let low_close = (candles[i].low - candles[i - 1].close).abs();
        tr[i] = high_low.max(high_close).max(low_close);
    }

    tr
}

pub fn atr(candles: &[Ohlc], length: usize) -> Vec<f64> {
    let mut tr = vec![0.0; candles.len()];

    for i in 1..candles.len() {
        let high_low = candles[i].high - candles[i].low;
        let high_close = (candles[i].high - candles[i - 1].close).abs();
        let low_close = (candles[i].low - candles[i - 1].close).abs();
        tr[i] = high_low.max(high_close).max(low_close);
    }

    rma(&tr, length)
}
