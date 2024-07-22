pub fn sma(src: &[f64], length: usize) -> Vec<f64> {
    let mut sma_values = Vec::with_capacity(src.len());

    for i in 0..src.len() {
        if i < length - 1 {
            sma_values.push(0.0);
        } else {
            let sum: f64 = src[i + 1 - length..=i].iter().sum();
            sma_values.push(sum / length as f64);
        }
    }

    sma_values
}
