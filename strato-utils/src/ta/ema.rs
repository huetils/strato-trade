pub fn ema(src: Vec<f64>, length: usize) -> Vec<f64> {
    let alpha = 2.0 / (length as f64 + 1.0);
    let mut ema = vec![0.0; src.len()];

    for i in 0..src.len() {
        if i == 0 {
            ema[i] = src[i]; // Start with the first value
        } else {
            ema[i] = alpha * src[i] + (1.0 - alpha) * ema[i - 1];
        }
    }

    ema
}
