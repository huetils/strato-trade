use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variables;
use good_lp::Expression;
use good_lp::Solution;
use good_lp::SolverModel;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;

// Define option data structure
#[derive(Clone, Debug, Default)]
pub struct OptionData {
    pub name: String,
    pub s: f64,     // Underlying asset price
    pub k: f64,     // Strike price
    pub t: f64,     // Time to maturity (in years)
    pub r: f64,     // Risk-free rate
    pub sigma: f64, // Volatility
    pub option_type: String,
    pub market_price: f64, // Current market price
}

// Define portfolio structure
pub struct Portfolio {
    pub holdings: Vec<(String, f64)>, // (Option name, Position size)
}

// Black-Scholes option pricing function
fn black_scholes_price(s: f64, k: f64, t: f64, r: f64, sigma: f64, option_type: &str) -> f64 {
    let norm = Normal::new(0.0, 1.0).unwrap(); // No need for Univariate trait
    let d1 = (f64::ln(s / k) + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();

    match option_type {
        "call" => s * norm.cdf(d1) - k * (f64::exp(-r * t)) * norm.cdf(d2),
        "put" => k * (f64::exp(-r * t)) * norm.cdf(-d2) - s * norm.cdf(-d1),
        _ => panic!("Invalid option type. Must be 'call' or 'put'"),
    }
}

// Linear programming model for arbitrage detection
fn find_arbitrage(
    prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
) -> Vec<f64> {
    let num_assets = prices.len();

    // Define the decision variables (weights for each option)
    variables! {
        vars:
        0 <= weights[num_assets] <= capital;
    }

    // Set up the linear programming problem
    let objective: Expression = (0..num_assets)
        .map(|i| -(prices[i] - transaction_costs[i]) * weights[i])
        .sum();

    // Create the problem with individual liquidity constraints
    let mut problem = vars.maximise(objective).using(default_solver);

    // Add capital constraint
    problem = problem.with(constraint!(weights.iter().sum::<Expression>() <= capital));

    // Add individual liquidity constraints
    for i in 0..num_assets {
        problem = problem.with(constraint!(weights[i] <= liquidity[i]));
    }

    let solution = problem.solve().unwrap();

    // Retrieve optimal weights (positions)
    weights.iter().map(|&var| solution.value(var)).collect()
}

// Portfolio construction function
pub fn construct_portfolio(option_data: Vec<OptionData>, capital: f64) -> Portfolio {
    let mut prices: Vec<f64> = Vec::new();
    let transaction_costs: Vec<f64> = vec![0.01; option_data.len()]; // Simulate transaction costs
    let liquidity: Vec<f64> = vec![100.0; option_data.len()]; // Simulate liquidity limits

    // Calculate theoretical prices and mispricing for each option
    for option in &option_data {
        let theoretical_price = black_scholes_price(
            option.s,
            option.k,
            option.t,
            option.r,
            option.sigma,
            &option.option_type,
        );
        let mispricing = theoretical_price - option.market_price;
        prices.push(mispricing);
    }

    // Find optimal portfolio weights via linear programming
    let portfolio_weights = find_arbitrage(prices, transaction_costs, capital, liquidity);

    // Create portfolio holdings
    let holdings = option_data
        .iter()
        .zip(portfolio_weights.iter())
        .map(|(option, &weight)| (option.name.clone(), weight))
        .collect();

    Portfolio { holdings }
}
