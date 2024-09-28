use std::time::Instant;

use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variable;
use good_lp::Expression;
use good_lp::ProblemVariables;
use good_lp::Solution;
use good_lp::SolverModel;
use good_lp::Variable;

/// Define option data structure
#[derive(Clone, Debug, Default)]
pub struct OptionData {
    pub name: String,
    /// Underlying asset price
    pub s: f64,
    /// Strike price
    pub k: f64,
    /// Time to maturity (in years)
    pub t: f64,
    /// Risk-free rate
    pub r: f64,
    /// Volatility of the underlying asset
    pub sigma: f64,
    /// Market price of the option
    pub market_price: f64,
    /// Option type ("call" or "put")
    pub option_type: String,
}

/// Struct for managing the portfolio's holdings
#[derive(Debug)]
pub struct Portfolio {
    /// Portfolio holdings (option name, position size)
    pub holdings: Vec<(String, f64)>,
}

/// Function to build a binomial tree and estimate probabilities
pub fn estimate_probabilities(
    s0: f64,
    r: f64,
    sigma: f64,
    t: f64,
    steps: usize,
) -> (Vec<f64>, Vec<f64>) {
    let dt = t / steps as f64;
    let u = f64::exp(sigma * dt.sqrt());
    let d = 1.0 / u;
    let p = (f64::exp(r * dt) - d) / (u - d);

    // Adjust p to be between 0 and 1
    let p = p.max(0.0).min(1.0);

    let mut asset_prices = Vec::new();
    let mut probabilities = Vec::new();

    for i in 0..=steps {
        let price = s0 * u.powi((steps - i) as i32) * d.powi(i as i32);
        asset_prices.push(price);

        let prob =
            binomial_coefficient(steps, i) * p.powi(i as i32) * (1.0 - p).powi((steps - i) as i32);
        probabilities.push(prob);
    }

    // Verify that probabilities sum to 1
    let total_probability: f64 = probabilities.iter().sum();
    println!("Total probability: {}", total_probability);

    (asset_prices, probabilities)
}

/// Helper function to calculate binomial coefficients
fn binomial_coefficient(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    if k == 0 || k == n {
        return 1.0;
    }
    let k = std::cmp::min(k, n - k); // Take advantage of symmetry
    let mut result = 1.0;
    for i in 1..=k {
        result *= (n - k + i) as f64 / i as f64;
    }
    result
}

/// Function to find arbitrage opportunities using linear programming
pub fn find_arbitrage(
    market_prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
    asset_prices: Vec<f64>,
    option_data: &Vec<OptionData>,
) -> Result<Vec<f64>, String> {
    let start_time = Instant::now();
    let num_assets = market_prices.len();

    let mut vars = ProblemVariables::new();

    // Initialize variables for buying (alpha) and selling (beta) positions
    let (alpha, beta) = initialize_positions(&mut vars, num_assets, &liquidity);

    // Build the objective function (minimize net investment)
    let (net_investment, _income, expenditure) =
        build_objective(&alpha, &beta, &market_prices, &transaction_costs);

    // Create the optimization problem
    let mut problem = vars.minimise(net_investment.clone()).using(default_solver);

    // **Capital constraint**: expenditure <= capital
    problem = problem.with(constraint!(expenditure.clone() <= capital));

    // **State-wise payoff constraints**
    add_state_payoff_constraints(
        &mut problem,
        &alpha,
        &beta,
        &option_data,
        &asset_prices,
        net_investment.clone(), // Pass net_investment instead of income and expenditure
    );

    // Solve the optimization problem
    let solution = problem.solve();

    // Performance metrics
    let duration = start_time.elapsed();
    println!("Optimization completed in {:?}", duration);

    match solution {
        Ok(sol) => {
            // Solution accuracy (objective function value)
            let objective_value = sol.eval(&net_investment);
            println!("Objective function value: {}", objective_value);

            // If the objective value is not significantly negative, return an error
            if objective_value >= -1e-6 {
                return Err("No arbitrage opportunity found.".to_string());
            }

            // Retrieve final positions (net weights) for each option
            let positions: Vec<f64> = alpha
                .iter()
                .zip(beta.iter())
                .map(|(&a, &b)| sol.value(a) - sol.value(b))
                .collect();

            Ok(positions)
        }
        Err(e) => {
            // Error handling for infeasible problems
            Err(format!("Optimization failed: {}", e))
        }
    }
}

fn add_state_payoff_constraints(
    problem: &mut (impl SolverModel + Clone),
    alpha: &[Variable],
    beta: &[Variable],
    option_data: &[OptionData],
    asset_prices: &[f64],
    net_investment: Expression, // Changed parameter
) {
    let num_states = asset_prices.len();

    for state in 0..num_states {
        let mut state_payoff = Expression::from(0.0);
        for (i, option) in option_data.iter().enumerate() {
            let intrinsic_value = match option.option_type.as_str() {
                "call" => f64::max(asset_prices[state] - option.k, 0.0),
                "put" => f64::max(option.k - asset_prices[state], 0.0),
                _ => 0.0,
            };
            state_payoff = state_payoff + intrinsic_value * (alpha[i] - beta[i]);
        }
        // Net profit in state = state_payoff - net_investment
        let net_profit = state_payoff - net_investment.clone();
        *problem = problem.clone().with(constraint!(net_profit >= 0.0));
    }
}

fn initialize_positions(
    vars: &mut ProblemVariables,
    num_assets: usize,
    liquidity: &[f64],
) -> (Vec<Variable>, Vec<Variable>) {
    let alpha: Vec<Variable> = (0..num_assets)
        .map(|i| vars.add(variable().min(0.0).max(liquidity[i])))
        .collect();

    let beta: Vec<Variable> = (0..num_assets)
        .map(|i| vars.add(variable().min(0.0).max(liquidity[i])))
        .collect();

    (alpha, beta)
}

fn build_objective(
    alpha: &[Variable],
    beta: &[Variable],
    market_prices: &[f64],
    transaction_costs: &[f64],
) -> (Expression, Expression, Expression) {
    // Net income from selling options (proceeds minus transaction costs)
    let income = beta
        .iter()
        .enumerate()
        .map(|(i, &b)| (market_prices[i] - transaction_costs[i]) * b)
        .sum::<Expression>();

    // Cost of buying options (price plus transaction costs)
    let expenditure = alpha
        .iter()
        .enumerate()
        .map(|(i, &a)| (market_prices[i] + transaction_costs[i]) * a)
        .sum::<Expression>();

    // Net investment (initial net cash outflow)
    let net_investment = expenditure.clone() - income.clone();

    (net_investment, income, expenditure)
}

/// Portfolio construction function.
pub fn construct_portfolio(
    option_data: Vec<OptionData>,
    capital: f64,
    steps: usize,
    transaction_costs: Vec<f64>,
    liquidity: Vec<f64>,
) -> Result<Portfolio, String> {
    // Market parameters (these would come from current market data)
    let s0 = option_data[0].s;
    let r = option_data[0].r;
    let sigma = option_data[0].sigma;
    let t = option_data[0].t;

    // Estimate probabilities using a binomial tree model
    let (asset_prices, _probabilities) = estimate_probabilities(s0, r, sigma, t, steps);

    let market_prices: Vec<f64> = option_data.iter().map(|o| o.market_price).collect();

    // Find optimal portfolio weights via linear programming
    let portfolio_weights = find_arbitrage(
        market_prices,
        transaction_costs,
        capital,
        liquidity,
        asset_prices,
        &option_data,
    )?;

    // Create portfolio holdings
    let holdings = option_data
        .iter()
        .zip(portfolio_weights.iter())
        .map(|(option, &weight)| (option.name.clone(), weight))
        .collect();

    Ok(Portfolio { holdings })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case with basic sample data
    #[test]
    fn test_basic_construct_portfolio() {
        let option_data = vec![
            OptionData {
                name: "Call Option 1".to_string(),
                s: 100.0,
                k: 100.0,
                t: 1.0,
                r: 0.05,
                sigma: 0.2,
                market_price: 10.0,
                option_type: "call".to_string(),
            },
            OptionData {
                name: "Put Option 1".to_string(),
                s: 100.0,
                k: 100.0,
                t: 1.0,
                r: 0.05,
                sigma: 0.2,
                market_price: 8.0,
                option_type: "put".to_string(),
            },
        ];

        let capital = 10000.0;
        let steps = 3;
        let transaction_costs = vec![1.0, 1.0];
        let liquidity = vec![1000.0, 1000.0];

        let portfolio_result = construct_portfolio(
            option_data.clone(),
            capital,
            steps,
            transaction_costs,
            liquidity,
        );

        assert!(portfolio_result.is_ok());
        let portfolio = portfolio_result.unwrap();

        for (name, position) in portfolio.holdings {
            println!("Option: {}, Position Size: {}", name, position);
        }
    }
}
