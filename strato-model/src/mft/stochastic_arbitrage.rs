use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variable;
use good_lp::Constraint;
use good_lp::Expression;
use good_lp::ProblemVariables;
use good_lp::Solution;
use good_lp::SolverModel;
use good_lp::Variable;
use strato_pricer::bs::black_scholes_call;
use strato_pricer::bs::black_scholes_put;

/// Represents the data for an option.
#[derive(Clone, Debug, Default)]
pub struct OptionData {
    pub name: String,
    /// Underlying asset price (S).
    pub s: f64,
    /// Strike price (K).
    pub k: f64,
    /// Time to maturity in years (T).
    pub t: f64,
    /// Risk-free interest rate (r).
    pub r: f64,
    /// Volatility of the underlying asset (σ).
    pub sigma: f64,
    /// Option type: `"call"` or `"put"`.
    pub option_type: String,
    /// Current market price of the option.
    pub market_price: f64,
}

/// Manages the portfolio's holdings.
pub struct Portfolio {
    /// Portfolio holdings as a vector of (option name, position size).
    pub holdings: Vec<(String, f64)>,
}

/// Finds arbitrage opportunities and computes optimal portfolio weights using
/// linear programming.
///
/// # Arguments
///
/// * `market_prices` - Market prices of the options.
/// * `transaction_costs` - Transaction costs associated with buying/selling
///   options.
/// * `capital` - Total capital available for investment.
/// * `liquidity` - Liquidity constraints for each option.
/// * `index_returns` - Returns of a benchmark index in different states.
/// * `risk_levels` - Array of risk levels to consider (e.g., for stochastic
///   dominance).
/// * `option_data` - Data for each option.
///
/// # Returns
///
/// A vector of optimal positions (weights) for each option.
///
/// # Mathematical Formulation
///
/// The objective is to maximize the total expected profit:
///
/// Maximize: `Z = Σ (π_i * w_i)`
///
/// where:
/// - `π_i = P_theoretical_i - P_market_i - C_transaction_i` is the profit per
///   unit of option `i`.
/// - `w_i` is the position size (number of units) of option `i`.
///
/// **Constraints:**
///
/// 1. **Capital Constraint:** `Σ [(w_i^+ + w_i^-) * (P_market_i +
///    C_transaction_i)] ≤ Capital`
///
///    - Ensures the total investment does not exceed available capital.
///    - `w_i^+` and `w_i^-` are the long and short positions, respectively.
///
/// 2. **Position Relationship:** `w_i = w_i^+ - w_i^-`  for all `i`
///
///    - Relates net positions to long and short positions.
///
/// 3. **Liquidity Constraints:** `w_i^+ ≤ L_i`,  `w_i^- ≤ L_i` for all `i`
///
///    - `L_i` is the liquidity limit for option `i`.
///
/// 4. **Stochastic Dominance Constraints:** `Portfolio Return_s * Risk Level ≥
///    Index Return_s * Risk Level` for all `s`
///
///    - Ensures portfolio returns are acceptable compared to a benchmark at
///      different risk levels.
///    - `s` indexes the different market states/scenarios.
///
/// 5. **Position Limits:** `-I_max ≤ w_i * (P_market_i + C_transaction_i) ≤
///    I_max` for all `i`
///
///    - `I_max = Capital / n` is the maximum investment per option.
pub fn find_arbitrage(
    market_prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
    index_returns: Vec<f64>,
    risk_levels: &[f64],
    option_data: &[OptionData],
) -> Vec<f64> {
    let num_assets = market_prices.len();
    let num_states = index_returns.len();

    let mut vars = ProblemVariables::new();

    // Initialize variables for positions
    let (weights, w_plus, w_minus, equality_constraints) =
        initialize_weights(&mut vars, num_assets, &liquidity);

    // Compute theoretical prices using the Black-Scholes model
    let theoretical_prices = compute_theoretical_prices(option_data);

    // Build the objective function (profit maximization)
    let objective = build_objective(
        &weights,
        &market_prices,
        &theoretical_prices,
        &transaction_costs,
    );

    // Create the optimization problem
    let mut problem = vars.maximise(objective).using(default_solver);

    // Add equality constraints
    for c in equality_constraints {
        problem = problem.with(c);
    }

    // Capital constraint: limit total investment to capital
    let total_capital_constraint = compute_total_capital_constraint::<Expression>(
        &w_plus,
        &w_minus,
        &market_prices,
        &transaction_costs,
    );
    problem = problem.with(constraint!(total_capital_constraint <= capital));

    // Liquidity constraints
    add_liquidity_constraints(&mut problem, &w_plus, &w_minus, &liquidity);

    // Stochastic dominance constraints
    let mut portfolio_returns = vec![Expression::from(0.0); num_states];
    for s in portfolio_returns.iter_mut().take(num_states) {
        for (i, &w) in weights.iter().enumerate() {
            let option_return = theoretical_prices[i] - market_prices[i] - transaction_costs[i];
            *s = s.clone() + w * option_return;
        }
    }

    add_stochastic_dominance_constraints(
        &mut problem,
        &portfolio_returns,
        &index_returns,
        risk_levels,
    );

    // Position limit constraints
    let num_options = weights.len();
    let max_investment_per_option = capital / num_options as f64;

    for (i, &w) in weights.iter().enumerate() {
        let investment_in_option = w * (market_prices[i] + transaction_costs[i]);
        problem = problem.with(constraint!(
            investment_in_option.clone() <= max_investment_per_option
        ));
        problem = problem.with(constraint!(
            investment_in_option >= -max_investment_per_option
        ));
    }

    // Solve the optimization problem
    let solution = problem.solve().unwrap();

    // Retrieve final positions (weights) for each option
    weights.iter().map(|&var| solution.value(var)).collect()
}

/// Initializes variables for option positions and sets up equality constraints.
///
/// For each option, creates three variables:
/// - `w`: Net position in the option (can be positive or negative).
/// - `w_p`: Positive part of the position (long positions, ≥ 0).
/// - `w_m`: Negative part of the position (short positions, ≥ 0).
///
/// Enforces the equality constraint: `w = w_p - w_m`.
///
/// # Arguments
///
/// * `vars` - Mutable reference to `ProblemVariables` for variable management.
/// * `num_assets` - Number of options/assets.
/// * `liquidity` - Liquidity constraints for each option.
///
/// # Returns
///
/// A tuple containing:
/// - `weights`: Vector of net position variables.
/// - `w_plus`: Vector of long position variables.
/// - `w_minus`: Vector of short position variables.
/// - `constraints`: Vector of equality constraints (`w = w_p - w_m`).
///
/// # Mathematical Formulation
///
/// For each option `i`:
/// - Net position: `w_i = w_i^+ - w_i^-`
/// - Bounds:
///   - `w_i^+ ≥ 0`, `w_i^- ≥ 0`
///   - `-L_i ≤ w_i ≤ L_i`
fn initialize_weights(
    vars: &mut ProblemVariables,
    num_assets: usize,
    liquidity: &[f64],
) -> (Vec<Variable>, Vec<Variable>, Vec<Variable>, Vec<Constraint>) {
    let mut weights = Vec::with_capacity(num_assets);
    let mut w_plus = Vec::with_capacity(num_assets);
    let mut w_minus = Vec::with_capacity(num_assets);
    let mut constraints = Vec::with_capacity(num_assets);

    for i in liquidity.iter().take(num_assets) {
        let w = vars.add(variable().bounds(-i..*i));
        let w_p = vars.add(variable().bounds(0.0..*i));
        let w_m = vars.add(variable().bounds(0.0..*i));
        let c = constraint!(w == w_p - w_m);

        weights.push(w);
        w_plus.push(w_p);
        w_minus.push(w_m);
        constraints.push(c);
    }
    (weights, w_plus, w_minus, constraints)
}

/// Computes theoretical option prices using the Black-Scholes model.
///
/// # Arguments
///
/// * `option_data` - Vector of `OptionData` containing details of each option.
///
/// # Returns
///
/// A vector of theoretical prices for each option.
///
/// # Mathematical Formulation
///
/// For a **call** option, the Black-Scholes formula is:
///
/// `C = S * N(d_1) - K * exp(-rT) * N(d_2)`
///
/// For a **put** option:
///
/// `P = K * exp(-rT) * N(-d_2) - S * N(-d_1)`
///
/// where:
///
/// `d_1 = [ln(S / K) + (r + 0.5 * σ^2) * T] / (σ * sqrt(T))`  
/// `d_2 = d_1 - σ * sqrt(T)`
///
/// - `N(.)` is the cumulative distribution function of the standard normal
///   distribution.
/// - `S` is the current price of the underlying asset.
/// - `K` is the strike price.
/// - `r` is the risk-free interest rate.
/// - `σ` is the volatility.
/// - `T` is the time to maturity.
fn compute_theoretical_prices(option_data: &[OptionData]) -> Vec<f64> {
    option_data
        .iter()
        .map(|option| {
            if option.option_type == "call" {
                black_scholes_call(option.s, option.k, option.t, option.r, option.sigma)
            } else {
                black_scholes_put(option.s, option.k, option.t, option.r, option.sigma)
            }
        })
        .collect()
}

/// Builds the objective function for profit maximization.
///
/// The objective is to maximize the total expected profit from the portfolio.
///
/// # Arguments
///
/// * `weights` - Variables representing positions in options.
/// * `market_prices` - Market prices of the options.
/// * `theoretical_prices` - Theoretical prices from the Black-Scholes model.
/// * `transaction_costs` - Transaction costs for each option.
///
/// # Returns
///
/// An `Expression` representing the objective function.
///
/// # Mathematical Formulation
///
/// The profit per unit for option `i` is:
///
/// `π_i = P_theoretical_i - P_market_i - C_transaction_i`
///
/// The objective function is:
///
/// `Maximize Z = Σ (π_i * w_i)`
fn build_objective(
    weights: &[Variable],
    market_prices: &[f64],
    theoretical_prices: &[f64],
    transaction_costs: &[f64],
) -> Expression {
    weights
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            let profit_per_unit = theoretical_prices[i] - market_prices[i] - transaction_costs[i];
            profit_per_unit * w
        })
        .sum()
}

/// Computes the total capital constraint expression.
///
/// Ensures that the total investment does not exceed the available capital.
///
/// # Arguments
///
/// * `w_plus` - Variables for long positions.
/// * `w_minus` - Variables for short positions.
/// * `market_prices` - Market prices of the options.
/// * `transaction_costs` - Transaction costs for each option.
///
/// # Returns
///
/// An `Expression` representing the total capital constraint.
///
/// # Mathematical Formulation
///
/// The total investment is:
///
/// `Total Investment = Σ [(w_i^+ + w_i^-) * (P_market_i + C_transaction_i)]`
///
/// This must satisfy:
///
/// `Total Investment ≤ Capital`
fn compute_total_capital_constraint<S>(
    w_plus: &[Variable],
    w_minus: &[Variable],
    market_prices: &[f64],
    transaction_costs: &[f64],
) -> Expression
where
    S: Into<Expression> + std::iter::Sum<good_lp::Expression> + good_lp::IntoAffineExpression,
{
    w_plus
        .iter()
        .enumerate()
        .map(|(i, &w_p)| w_p * (market_prices[i] + transaction_costs[i]))
        .sum::<Expression>()
        + w_minus
            .iter()
            .enumerate()
            .map(|(i, &w_m)| w_m * (market_prices[i] + transaction_costs[i]))
            .sum::<S>()
}

/// Adds liquidity constraints to the optimization problem.
///
/// Ensures that the positions in each option do not exceed the available
/// liquidity.
///
/// # Arguments
///
/// * `problem` - Mutable reference to the solver model.
/// * `w_plus` - Variables for long positions.
/// * `w_minus` - Variables for short positions.
/// * `liquidity` - Liquidity limits for each option.
///
/// # Mathematical Formulation
///
/// For each option `i`:
///
/// `w_i^+ ≤ L_i`,  `w_i^- ≤ L_i`
fn add_liquidity_constraints(
    problem: &mut impl SolverModel,
    w_plus: &[Variable],
    w_minus: &[Variable],
    liquidity: &[f64],
) {
    for (i, (&w_p, &w_m)) in w_plus.iter().zip(w_minus).enumerate() {
        problem.add_constraint(constraint!(w_p <= liquidity[i]));
        problem.add_constraint(constraint!(w_m <= liquidity[i]));
    }
}

/// Adds stochastic dominance constraints to the optimization problem.
///
/// Ensures that the portfolio's returns are at least as good as the benchmark
/// index returns at different risk levels.
///
/// # Arguments
///
/// * `problem` - Mutable reference to the solver model.
/// * `portfolio_returns` - Expressions representing portfolio returns in each
///   state.
/// * `index_returns` - Index returns in each state.
/// * `risk_levels` - Array of risk levels to consider.
///
/// # Mathematical Formulation
///
/// For each state `s` and risk level `Risk Level`:
///
/// `Portfolio Return_s * Risk Level ≥ Index Return_s * Risk Level`
fn add_stochastic_dominance_constraints(
    problem: &mut impl SolverModel,
    portfolio_returns: &[Expression],
    index_returns: &[f64],
    risk_levels: &[f64],
) {
    let num_states = portfolio_returns.len();

    for &risk_level in risk_levels {
        for s in 0..num_states {
            let portfolio_risk_adjusted = portfolio_returns[s].clone() * risk_level;
            let index_risk_adjusted = index_returns[s] * risk_level;

            problem.add_constraint(constraint!(portfolio_risk_adjusted >= index_risk_adjusted));
        }
    }
}

/// Constructs the portfolio by finding optimal weights and assembling holdings.
///
/// **Note:** This is intended for demonstration purposes and should not be used
/// in production.
///
/// # Arguments
///
/// * `option_data` - Vector of `OptionData` for each option.
/// * `capital` - Total capital available for investment.
/// * `risk_levels` - Array of risk levels for stochastic dominance constraints.
/// * `index_returns` - Real or simulated index returns for benchmarking.
/// * `transaction_costs` - Transaction costs for each option.
/// * `liquidity` - Liquidity constraints for each option.
///
/// # Returns
///
/// A `Portfolio` containing the holdings (option names and positions).
///
/// # Example
///
/// ```
/// let option_data = vec![
///     OptionData {
///         name: "Option1".to_string(),
///         s: 100.0,
///         k: 90.0,
///         t: 0.5,
///         r: 0.05,
///         sigma: 0.2,
///         option_type: "call".to_string(),
///         market_price: 10.0,
///     },
///     // ... more options ...
/// ];
///
/// let capital = 100000.0;
/// let risk_levels = &[0.01, 0.1, 0.5];
/// let index_returns = vec![0.05, 0.02, -0.01]; // Simulated index returns
/// let transaction_costs = vec![0.05; option_data.len()];
/// let liquidity = vec![1000.0; option_data.len()];
///
/// let portfolio = construct_portfolio(
///     option_data,
///     capital,
///     risk_levels,
///     index_returns,
///     transaction_costs,
///     liquidity,
/// );
/// ```
pub fn construct_portfolio(
    option_data: Vec<OptionData>,
    capital: f64,
    risk_levels: &[f64],
    index_returns: Vec<f64>,
    transaction_costs: Vec<f64>,
    liquidity: Vec<f64>,
) -> Portfolio {
    let market_prices: Vec<f64> = option_data.iter().map(|o| o.market_price).collect();

    // Calculate expected payoffs for each option (not directly used in
    // optimization)
    let mut expected_payoffs: Vec<f64> = Vec::new();
    for option in &option_data {
        let payoff = if option.option_type == "call" {
            f64::max(option.s - option.k, 0.0)
        } else {
            f64::max(option.k - option.s, 0.0)
        };
        expected_payoffs.push(payoff);
    }

    // Find optimal portfolio weights via linear programming
    let portfolio_weights = find_arbitrage(
        market_prices,
        transaction_costs,
        capital,
        liquidity,
        index_returns,
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
