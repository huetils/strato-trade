use statrs::function::erf::erf;

/// Cumulative distribution function for the standard normal distribution
fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / f64::sqrt(2.0)))
}

/// Black-Scholes formula for European call options
pub fn black_scholes_call(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t == 0.0 {
        // Option has expired; return intrinsic value
        return (s - k).max(0.0);
    }

    let d1 = (f64::ln(s / k) + (r + 0.5 * sigma.powi(2)) * t) / (sigma * f64::sqrt(t));
    let d2 = d1 - sigma * f64::sqrt(t);
    s * norm_cdf(d1) - k * f64::exp(-r * t) * norm_cdf(d2)
}

/// Black-Scholes formula for European put options
pub fn black_scholes_put(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> f64 {
    if t == 0.0 {
        // Option has expired; return intrinsic value
        return (k - s).max(0.0);
    }

    let d1 = (f64::ln(s / k) + (r + 0.5 * sigma.powi(2)) * t) / (sigma * f64::sqrt(t));
    let d2 = d1 - sigma * f64::sqrt(t);
    k * f64::exp(-r * t) * norm_cdf(-d2) - s * norm_cdf(-d1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_scholes_call_price() {
        let s = 100.0; // Spot price of the underlying asset
        let k = 100.0; // Strike price
        let t = 1.0; // Time to maturity (1 year)
        let r = 0.05; // Risk-free interest rate (5%)
        let sigma = 0.2; // Volatility (20%)

        // Call price using Black-Scholes formula
        let call_price = black_scholes_call(s, k, t, r, sigma);

        // Expected price calculated from a standard Black-Scholes calculator
        let expected_call_price = 10.45058;

        // Set an acceptable margin of error (epsilon) for floating-point comparison
        let epsilon = 1e-5;
        assert!(
            (call_price - expected_call_price).abs() < epsilon,
            "Call price incorrect. Expected: {}, Got: {}",
            expected_call_price,
            call_price
        );
    }

    #[test]
    fn test_black_scholes_put_price() {
        let s = 100.0; // Spot price of the underlying asset
        let k = 100.0; // Strike price
        let t = 1.0; // Time to maturity (1 year)
        let r = 0.05; // Risk-free interest rate (5%)
        let sigma = 0.2; // Volatility (20%)

        // Put price using Black-Scholes formula
        let put_price = black_scholes_put(s, k, t, r, sigma);

        // Expected price calculated from a standard Black-Scholes calculator
        let expected_put_price = 5.57352;

        // Set an acceptable margin of error (epsilon) for floating-point comparison
        let epsilon = 1e-5;
        assert!(
            (put_price - expected_put_price).abs() < epsilon,
            "Put price incorrect. Expected: {}, Got: {}",
            expected_put_price,
            put_price
        );
    }

    #[test]
    fn test_black_scholes_zero_volatility() {
        let s = 100.0; // Spot price of the underlying asset
        let k = 100.0; // Strike price
        let t = 1.0; // Time to maturity (1 year)
        let r = 0.05; // Risk-free interest rate (5%)
        let sigma = 0.0; // Volatility (0%)

        // Call price with zero volatility should equal max(S - K, 0) discounted at the
        // risk-free rate
        let call_price = black_scholes_call(s, k, t, r, sigma);
        let expected_call_price = (s - k * f64::exp(-r * t)).max(0.0);

        let epsilon = 1e-5;
        assert!(
            (call_price - expected_call_price).abs() < epsilon,
            "Call price with zero volatility incorrect. Expected: {}, Got: {}",
            expected_call_price,
            call_price
        );

        // Put price with zero volatility should equal max(K - S, 0) discounted at the
        // risk-free rate
        let put_price = black_scholes_put(s, k, t, r, sigma);
        let expected_put_price = (k * f64::exp(-r * t) - s).max(0.0);

        assert!(
            (put_price - expected_put_price).abs() < epsilon,
            "Put price with zero volatility incorrect. Expected: {}, Got: {}",
            expected_put_price,
            put_price
        );
    }

    #[test]
    fn test_black_scholes_zero_time_to_maturity() {
        let s = 100.0; // Spot price of the underlying asset
        let k = 100.0; // Strike price
        let t = 0.0; // Time to maturity (0 years)
        let r = 0.05; // Risk-free interest rate (5%)
        let sigma = 0.2; // Volatility (20%)

        // Call price with zero time to maturity should equal max(S - K, 0)
        let call_price = black_scholes_call(s, k, t, r, sigma);
        let expected_call_price = (s - k).max(0.0);

        let epsilon = 1e-5;

        assert!(
            (call_price - expected_call_price).abs() < epsilon,
            "Call price with zero time to maturity incorrect. Expected: {}, Got: {}",
            expected_call_price,
            call_price
        );

        // Put price with zero time to maturity should equal max(K - S, 0)
        let put_price = black_scholes_put(s, k, t, r, sigma);
        let expected_put_price = (k - s).max(0.0);

        assert!(
            (put_price - expected_put_price).abs() < epsilon,
            "Put price with zero time to maturity incorrect. Expected: {}, Got: {}",
            expected_put_price,
            put_price
        );
    }

    #[test]
    fn test_black_scholes_deep_in_the_money_call() {
        let s = 150.0; // Spot price of the underlying asset (deep ITM)
        let k = 100.0; // Strike price
        let t = 1.0; // Time to maturity (1 year)
        let r = 0.05; // Risk-free interest rate (5%)
        let sigma = 0.2; // Volatility (20%)

        // Call price using Black-Scholes formula
        let call_price = black_scholes_call(s, k, t, r, sigma);
        let expected_call_price = 54.970140138;

        let epsilon = 1e-5; // Tolerance for floating-point comparison
        assert!(
            (call_price - expected_call_price).abs() < epsilon,
            "Call price deep in-the-money incorrect. Expected: {}, Got: {}",
            expected_call_price,
            call_price
        );
    }
}
