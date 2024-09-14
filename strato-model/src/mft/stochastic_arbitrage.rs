use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variable;
use good_lp::Expression;
use good_lp::ProblemVariables;
use good_lp::Solution;
use good_lp::SolverModel;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;

/// OptionData represents the characteristics of an option (call or put).
///
/// # Fields
///
/// - `name`: The name of the option (e.g., "Call1").
/// - `s`: Underlying asset price.
/// - `k`: Strike price.
/// - `t`: Time to maturity (in years).
/// - `r`: Risk-free interest rate.
/// - `sigma`: Volatility of the asset.
/// - `option_type`: Type of the option ("call" or "put").
/// - `market_price`: The current market price of the option.
#[derive(Clone, Debug, Default)]
pub struct OptionData {
    pub name: String,
    pub s: f64,
    pub k: f64,
    pub t: f64,
    pub r: f64,
    pub sigma: f64,
    pub option_type: String,
    pub market_price: f64,
}

/// Portfolio holds the result of the constructed portfolio after optimization.
///
/// # Fields
///
/// - `holdings`: A vector of tuples containing the option name and its
///   corresponding position size.
pub struct Portfolio {
    pub holdings: Vec<(String, f64)>,
}

/// Black-Scholes pricing model for calculating the theoretical price of an
/// option.
///
/// # Parameters
///
/// - `s`: Current price of the underlying asset.
/// - `k`: Strike price of the option.
/// - `t`: Time to expiration (in years).
/// - `r`: Risk-free interest rate.
/// - `sigma`: Volatility of the underlying asset.
/// - `option_type`: Type of the option ("call" or "put").
///
/// # Returns
///
/// - Returns the theoretical price of the option based on the Black-Scholes
///   model.
fn black_scholes_price(s: f64, k: f64, t: f64, r: f64, sigma: f64, option_type: &str) -> f64 {
    let norm = Normal::new(0.0, 1.0).unwrap();
    let d1 = (f64::ln(s / k) + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();

    match option_type {
        "call" => s * norm.cdf(d1) - k * (f64::exp(-r * t)) * norm.cdf(d2),
        "put" => k * (f64::exp(-r * t)) * norm.cdf(-d2) - s * norm.cdf(-d1),
        _ => panic!("Invalid option type. Must be 'call' or 'put'"),
    }
}

/// Solves a linear programming model for arbitrage detection in options
/// trading, incorporating short-selling and stochastic dominance constraints
/// (SSD).
///
/// # Overview
///
/// This function solves the optimization problem:
///
/// **Objective Function**:
/// Maximize the arbitrage profit:
/// ∑(i=1 to n) ((pᵢ - cᵢ) × wᵢ)
///
/// **Constraints**:
///
/// - **Position Bounds**: -C ≤ wᵢ ≤ C, for all i (C is the available capital).
/// - **Capital Constraint**: ∑(i=1 to n) wᵢ ≤ C.
/// - **Liquidity Constraints**: wᵢ ≤ Lᵢ, for all i (Lᵢ is the liquidity limit
///   for each option).
/// - **Stochastic Dominance Constraints**: The portfolio is constrained to
///   stochastically dominate the index returns at various risk levels.
///
/// # Parameters
///
/// - `prices`: A vector of mispricing values (theoretical price - market price)
///   for each option.
/// - `transaction_costs`: A vector of transaction costs for each option.
/// - `capital`: The total capital available for constructing the portfolio.
/// - `liquidity`: A vector of liquidity limits for each option.
/// - `index_returns`: A vector of index returns used for applying SSD
///   constraints.
/// - `risk_levels`: A slice of risk levels for which the SSD constraints are
///   applied.
///
/// # Returns
///
/// - Returns a vector of optimal position sizes for each option, where positive
///   values indicate long positions and negative values indicate short
///   positions.
pub fn find_arbitrage(
    prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
    index_returns: Vec<f64>,
    risk_levels: &[f64],
) -> Vec<f64> {
    let num_assets = prices.len(); // The number of options/assets we are optimizing over.

    // Initialize decision variables (weights for each option)
    let mut vars = ProblemVariables::new();
    let weights: Vec<_> = (0..num_assets)
        // Allow both long and short positions.
        .map(|_i| vars.add(variable().min(-capital).max(capital)))
        .collect();

    // Objective Function: Maximize arbitrage profit
    let objective: Expression = (0..num_assets)
        // Profit per option: (mispricing - cost) * position size
        .map(|i| -(prices[i] - transaction_costs[i]) * weights[i])
        .sum(); // Total profit across all options

    // Setup the linear programming problem
    let mut problem = vars.maximise(objective).using(default_solver);

    // Add capital constraint: ∑wᵢ ≤ C
    problem = problem.with(constraint!(weights.iter().sum::<Expression>() <= capital));

    // Add liquidity constraints: wᵢ ≤ Lᵢ for all i
    for i in 0..num_assets {
        problem = problem.with(constraint!(weights[i] <= liquidity[i]));
    }

    // Convert weights from `Vec<Variable>` to `Vec<Expression>` for SSD
    let weight_expressions: Vec<Expression> = weights.iter().map(|&w| w.into()).collect();

    // Add stochastic dominance constraints (SSD)
    add_stochastic_dominance_constraints(
        &mut problem,
        &weight_expressions,
        &index_returns,
        risk_levels,
    );

    // Solve the linear programming problem
    let solution = problem.solve().unwrap();

    // Return the optimal weights (position sizes) for each asset
    weights.iter().map(|&var| solution.value(var)).collect()
}

/// Adds stochastic dominance constraints (SSD) to the optimization problem.
///
/// This ensures that the portfolio stochastically dominates the index returns
/// at the specified risk levels.
///
/// # Parameters
///
/// - `problem`: The linear programming problem.
/// - `portfolio_payoffs`: A slice of the portfolio's payoff expressions.
/// - `index_payoffs`: A slice of the index payoffs (e.g., returns).
/// - `risk_levels`: A slice of risk levels at which the dominance constraint is
///   applied.
fn add_stochastic_dominance_constraints(
    problem: &mut (impl SolverModel + Clone),
    portfolio_payoffs: &[Expression],
    index_payoffs: &[f64],
    risk_levels: &[f64],
) {
    let num_assets = portfolio_payoffs.len();

    // Iterate through each risk level and apply SSD constraints
    for &risk_level in risk_levels {
        for i in 0..num_assets {
            // Adjust portfolio and index payoffs based on the risk level
            let portfolio_risk_adjusted = portfolio_payoffs[i].clone() * risk_level;
            let index_risk_adjusted = index_payoffs[i] * risk_level;

            // Add a constraint: portfolio_payoff >= index_payoff at this risk level
            problem
                .clone()
                .with(constraint!(portfolio_risk_adjusted >= index_risk_adjusted));
        }
    }
}

/// Constructs an optimal portfolio by finding the best positions in a set of
/// options.
///
/// # Parameters
///
/// - `option_data`: A vector of OptionData objects representing the options
///   available for the portfolio.
/// - `capital`: The total capital available for constructing the portfolio.
/// - `risk_levels`: A slice of risk levels for stochastic dominance
///   constraints.
/// - `index_returns`: A vector of index returns used for applying SSD
///   constraints.
///
/// # Returns
///
/// - Returns a Portfolio object that contains the option holdings and their
///   respective positions.
pub fn construct_portfolio(
    option_data: Vec<OptionData>,
    capital: f64,
    risk_levels: &[f64],
    index_returns: Vec<f64>, // Pass real or simulated index returns as input
    transaction_costs: Vec<f64>,
    liquidity: Vec<f64>,
) -> Portfolio {
    let mut prices: Vec<f64> = Vec::new();

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
    let portfolio_weights = find_arbitrage(
        prices,
        transaction_costs,
        capital,
        liquidity,
        index_returns, // Pass actual index returns here
        risk_levels,   // Pass the input risk levels to find_arbitrage
    );

    // Create portfolio holdings: map each option to its corresponding position size
    let holdings = option_data
        .iter()
        .zip(portfolio_weights.iter())
        .map(|(option, &weight)| (option.name.clone(), weight))
        .collect();

    Portfolio { holdings }
}
