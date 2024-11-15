use statrs::distribution::Continuous;
use statrs::distribution::{ContinuousCDF, Normal};

#[allow(unused_variables)]
pub fn calculate_futures_to_hedge(
    option_type: &str,
    model_type: &str,
    num_contracts: usize,
    s: f64,       // Underlying price
    k: f64,       // Strike price
    t: f64,       // Time to maturity
    r: f64,       // Risk-free rate
    sigma: f64,   // Volatility
    steps: usize, // Steps for binomial model if applicable
) -> f64 {
    let delta = if model_type == "european" {
        // black_scholes_delta(s, k, t, r, sigma, option_type)
        0.0
    } else {
        // american_option_binomial_delta(s, k, t, r, sigma, steps, option_type)
        0.0
    };

    let total_delta = num_contracts as f64 * delta;
    let futures_needed = -total_delta; // Assume futures delta = 1

    futures_needed
}

// Function to calculate d1 using the Black-Scholes formula
pub fn calculate_d1(
    underlying_price: f64,
    strike_price: f64,
    time_to_expiration: f64,
    risk_free_rate: f64,
    volatility: f64,
) -> f64 {
    let d1 = (underlying_price / strike_price).ln()
        + (risk_free_rate + 0.5 * volatility.powi(2)) * time_to_expiration;
    d1 / (volatility * time_to_expiration.sqrt())
}

// Use d1 to calculate delta and gamma
pub fn calculate_greeks_from_d1(
    d1: f64,
    underlying_price: f64,
    time_to_expiration: f64,
    volatility: f64,
) -> (f64, f64, f64) {
    let normal = Normal::new(0.0, 1.0).unwrap();

    // Calculate delta for call and put options
    let delta_call = normal.cdf(d1);
    let delta_put = delta_call - 1.0;

    // Calculate gamma
    let gamma = normal.pdf(d1) / (underlying_price * volatility * time_to_expiration.sqrt());

    (delta_call, delta_put, gamma)
}
