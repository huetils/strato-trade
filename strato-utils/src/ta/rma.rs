/// https://www.tradingview.com/pine-script-reference/v5/#fun_ta.rma
pub fn rma(src: &[f64], length: usize) -> Vec<f64> {
    let alpha = 1.0 / length as f64;
    let mut rma_values = Vec::with_capacity(src.len());

    if src.len() >= length {
        let initial_sma: f64 = src.iter().take(length).sum::<f64>() / length as f64;
        rma_values.push(initial_sma);
    } else {
        rma_values.push(src.iter().sum::<f64>() / src.len() as f64);
    }

    for i in 1..src.len() {
        let prev_rma = rma_values[i - 1];
        let new_rma = alpha * src[i] + (1.0 - alpha) * prev_rma;
        rma_values.push(new_rma);
    }

    rma_values
}
