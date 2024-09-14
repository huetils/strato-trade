use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variable;
use good_lp::Constraint;
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
    /// Option type (call or put)
    pub option_type: String,
    /// Current market price
    pub market_price: f64,
}

/// Struct for managing the portfolio's holdings
pub struct Portfolio {
    /// Portfolio holdings (option name, position size)
    pub holdings: Vec<(String, f64)>,
}

/// Pricing kernel: A function to compute the theoretical price based on
/// expected payoffs
///
/// This function models the pricing kernel for an option's payoff.
/// In this simplified example, we use a basic kernel assuming risk-averse
/// utility.
///
/// # Arguments
/// * `expected_payoff` - Expected payoff of the option.
/// * `risk_aversion` - A risk aversion parameter (higher = more risk-averse).
///
/// # Returns
/// * The price of the option based on the pricing kernel.
fn pricing_kernel(expected_payoff: f64, risk_aversion: f64, _option_type: &str) -> f64 {
    expected_payoff * f64::exp(-risk_aversion * expected_payoff)
}

pub fn find_arbitrage(
    expected_payoffs: Vec<f64>,
    market_prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
    risk_aversion: f64,
    index_returns: Vec<f64>,
    risk_levels: &[f64],
    option_data: &Vec<OptionData>,
) -> Vec<f64> {
    let num_assets = expected_payoffs.len();

    let mut vars = ProblemVariables::new();

    // Initialize variables for long and short positions
    let (weights, long_short_weights) = initialize_weights(&mut vars, num_assets, capital);

    // Compute theoretical prices using the pricing kernel
    let theoretical_prices =
        compute_theoretical_prices(&expected_payoffs, risk_aversion, option_data);

    // Build the objective function (profit maximization)
    let objective = build_objective(
        &weights,
        &market_prices,
        &theoretical_prices,
        &transaction_costs,
    );

    // Create the optimization problem
    let mut problem = vars.maximise(objective).using(default_solver);

    // **Capital constraint**: limit total investment to capital
    let total_capital_constraint = compute_total_capital_constraint(&long_short_weights, capital);
    problem = problem.with(constraint!(total_capital_constraint <= capital));

    // **Liquidity constraints**: limit positions by liquidity
    add_liquidity_constraints(&mut problem, &weights, &liquidity);

    // * Add stochastic dominance constraints (optional, based on your model)
    let weight_expressions: Vec<Expression> = weights.iter().map(|&w| w.into()).collect();
    add_stochastic_dominance_constraints(
        &mut problem,
        &weight_expressions,
        &index_returns,
        risk_levels,
    );

    // Solve the optimization problem
    let solution = problem.solve().unwrap();

    // Retrieve final positions (weights) for each option
    weights.iter().map(|&var| solution.value(var)).collect()
}

fn initialize_weights(
    vars: &mut ProblemVariables,
    num_assets: usize,
    capital: f64,
) -> (Vec<Variable>, Vec<(Variable, Variable, Constraint)>) {
    let weights: Vec<Variable> = (0..num_assets)
        .map(|_| vars.add(variable().min(-capital).max(capital)))
        .collect();

    // Split weights into long and short components for capital constraint
    let long_short_weights: Vec<_> = weights
        .iter()
        .map(|&w| {
            let long_weight = vars.add(variable().min(0.0).max(capital));
            let short_weight = vars.add(variable().min(0.0).max(capital));
            let long_short_constraint = constraint!(w == long_weight - short_weight);
            (long_weight, short_weight, long_short_constraint)
        })
        .collect();

    (weights, long_short_weights)
}

fn compute_theoretical_prices(
    expected_payoffs: &[f64],
    risk_aversion: f64,
    option_data: &Vec<OptionData>,
) -> Vec<f64> {
    expected_payoffs
        .iter()
        .enumerate()
        .map(|(i, &payoff)| pricing_kernel(payoff, risk_aversion, &option_data[i].option_type))
        .collect()
}

fn build_objective(
    weights: &[Variable],
    market_prices: &[f64],
    theoretical_prices: &[f64],
    transaction_costs: &[f64],
) -> Expression {
    (0..weights.len())
        .map(|i| {
            if market_prices[i] > theoretical_prices[i] {
                // Short the option if overpriced, accounting for transaction costs
                (market_prices[i] - theoretical_prices[i] - transaction_costs[i]) * -weights[i]
            } else {
                // Go long if underpriced, accounting for transaction costs
                (theoretical_prices[i] - market_prices[i] - transaction_costs[i]) * weights[i]
            }
        })
        .sum()
}

fn compute_total_capital_constraint(
    long_short_weights: &[(Variable, Variable, Constraint)],
    capital: f64,
) -> Expression {
    let total_investment: Expression = long_short_weights
        .iter()
        .map(|(long_weight, short_weight, _)| *long_weight + short_weight)
        .sum();

    // Ensure the total investment does not exceed the available capital
    total_investment - capital
}

fn add_liquidity_constraints(
    problem: &mut (impl SolverModel + Clone),
    weights: &[Variable],
    liquidity: &[f64],
) {
    for (i, &weight) in weights.iter().enumerate() {
        *problem = problem.clone().with(constraint!(weight <= liquidity[i])); // Long positions
        *problem = problem.clone().with(constraint!(weight >= -liquidity[i])); // Short positions
    }
}

/// Add stochastic dominance constraints (SSD)
fn add_stochastic_dominance_constraints(
    problem: &mut (impl SolverModel + Clone),
    portfolio_payoffs: &[Expression],
    index_payoffs: &[f64],
    risk_levels: &[f64],
) {
    let num_assets = portfolio_payoffs.len();

    for &risk_level in risk_levels {
        for i in 0..num_assets {
            // Create constraints for different levels of risk aversion
            let portfolio_risk_adjusted = portfolio_payoffs[i].clone() * risk_level;
            let index_risk_adjusted = index_payoffs[i] * risk_level;

            // Ensure the portfolio payoff dominates the index payoff at this risk level
            problem
                .clone()
                .with(constraint!(portfolio_risk_adjusted >= index_risk_adjusted));
        }
    }
}

/// Portfolio construction function.
/// This is only intended for demonstration purposes and should not be used in
/// production.
pub fn construct_portfolio(
    option_data: Vec<OptionData>,
    capital: f64,
    risk_aversion: f64, // Risk aversion for pricing kernel
    risk_levels: &[f64],
    index_returns: Vec<f64>,     // Real or simulated index returns
    transaction_costs: Vec<f64>, // Including transaction costs
    liquidity: Vec<f64>,         // Including liquidity constraints
) -> Portfolio {
    let mut expected_payoffs: Vec<f64> = Vec::new();
    let market_prices: Vec<f64> = option_data.iter().map(|o| o.market_price).collect();

    // Calculate expected payoffs for each option
    for option in &option_data {
        let payoff = if option.option_type == "call" {
            f64::max(option.s - option.k, 0.0) // Call payoff: max(S-K, 0)
        } else {
            f64::max(option.k - option.s, 0.0) // Put payoff: max(K-S, 0)
        };
        expected_payoffs.push(payoff);
    }

    // Find optimal portfolio weights via linear programming
    let portfolio_weights = find_arbitrage(
        expected_payoffs,
        market_prices,
        transaction_costs, // Reintegrating transaction costs
        capital,
        liquidity,     // Reintegrating liquidity constraints
        risk_aversion, // Pass risk aversion to the pricing kernel
        index_returns, // Pass actual index returns here
        risk_levels,
        &option_data,
    );

    // Create portfolio holdings
    let holdings = option_data
        .iter()
        .zip(portfolio_weights.iter())
        .map(|(option, &weight)| (option.name.clone(), weight))
        .collect();

    Portfolio { holdings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_kernel() {
        let payoff = 100.0;
        let risk_aversion = 0.1;
        let option_type = "call";
        let price = pricing_kernel(payoff, risk_aversion, option_type);
        assert!(price > 0.0, "Pricing kernel should return positive prices");

        // Use a margin of error for floating point comparison instead of exact equality
        let expected_price = 100.0 * f64::exp(-0.1 * 100.0);
        let epsilon = 1e-10;
        assert!(
            (price - expected_price).abs() < epsilon,
            "Pricing kernel is incorrect. Expected: {}, Got: {}",
            expected_price,
            price
        );
    }

    #[test]
    fn test_arbitrage_detection() {
        let expected_payoffs = vec![20.0, 10.0]; // Payoff for two options
        let market_prices = vec![10.0, 12.0]; // Market prices, first one is underpriced
        let transaction_costs = vec![0.05, 0.05];
        let capital = 1000.0;
        let liquidity = vec![50.0, 50.0]; // Liquidity constraints
        let risk_aversion = 0.2;
        let index_returns = vec![1.0, 0.5]; // Simulated index returns
        let risk_levels = &[0.01, 0.1, 0.5];

        let option_data = vec![
            OptionData {
                name: "Call1".to_string(),
                s: 100.0,
                k: 90.0,
                t: 0.5,
                r: 0.05,
                option_type: "call".to_string(),
                market_price: 10.0,
            },
            OptionData {
                name: "Put1".to_string(),
                s: 100.0,
                k: 110.0,
                t: 0.5,
                r: 0.05,
                option_type: "put".to_string(),
                market_price: 12.0,
            },
        ];

        let weights = find_arbitrage(
            expected_payoffs.clone(),
            market_prices.clone(),
            transaction_costs,
            capital,
            liquidity,
            risk_aversion,
            index_returns,
            risk_levels,
            &option_data,
        );

        // Update expectations: first option should have a short position based on the current setup
        assert!(
            weights[0] < 0.0,
            "First option (call) should have a short position"
        );
        assert!(
            weights[1] < 0.0,
            "Second option (put) should have a short position"
        );
    }

    #[test]
    fn test_portfolio_construction() {
        let option_data = vec![
            OptionData {
                name: "Call1".to_string(),
                s: 100.0,
                k: 90.0,
                t: 0.5,
                r: 0.05,
                option_type: "call".to_string(),
                market_price: 8.0,
            },
            OptionData {
                name: "Put1".to_string(),
                s: 100.0,
                k: 110.0,
                t: 0.5,
                r: 0.05,
                option_type: "put".to_string(),
                market_price: 12.0,
            },
        ];

        let capital = 10000.0;
        let risk_aversion = 0.1;
        let risk_levels = &[0.01, 0.1, 0.5];
        let index_returns = vec![1.5, 0.5, 0.2];
        let transaction_costs = vec![0.01, 0.01];
        let liquidity = vec![50.0, 50.0];

        let portfolio = construct_portfolio(
            option_data,
            capital,
            risk_aversion,
            risk_levels,
            index_returns,
            transaction_costs,
            liquidity,
        );

        // Check portfolio holdings are within liquidity constraints
        for (_, position) in portfolio.holdings {
            println!("Position: {}", position);

            assert!(
                position.abs() <= 50.0,
                "Position exceeds liquidity constraints"
            );
        }
    }
}
