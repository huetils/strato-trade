use good_lp::constraint;
use good_lp::default_solver;
use good_lp::variable;
use good_lp::Expression;
use good_lp::ProblemVariables;
use good_lp::Solution;
use good_lp::SolverModel;
use statrs::distribution::ContinuousCDF;
use statrs::distribution::Normal;

// Define option data structure
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
    /// Volatility
    pub sigma: f64,
    /// Option type (call or put)
    pub option_type: String,
    /// Current market price
    pub market_price: f64,
}

pub struct Portfolio {
    /// Portfolio holdings (option name, position size)
    pub holdings: Vec<(String, f64)>,
}

// Black-Scholes pricing model
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

/// Implements a linear programming model for arbitrage detection in options trading, including short-selling.
///
/// # Overview
///
/// This function solves the following optimization problem to maximize arbitrage profit from options mispricing:
///
/// **Objective Function**:
///
/// Maximize:
/// ∑(i=1 to n) ((pᵢ - cᵢ) × wᵢ)
///
/// **Subject to Constraints**:
///
/// - **Position Bounds**:
///   -C ≤ wᵢ ≤ C, for all i ∈ {1, 2, ..., n}
///
/// - **Capital Constraint**:
///   ∑(i=1 to n) wᵢ ≤ C
///
/// - **Liquidity Constraints**:
///   wᵢ ≤ Lᵢ, for all i ∈ {1, 2, ..., n}
///
/// # Notation
///
/// - n: Number of assets (options).
/// - wᵢ: Position size for asset i (number of contracts), which can be positive (long) or negative (short).
/// - pᵢ: Mispricing of asset i (theoretical price minus market price).
/// - cᵢ: Transaction cost for trading asset i.
/// - C: Total capital available for trading.
/// - Lᵢ: Liquidity limit for asset i.
///
/// # Parameters
///
/// - `prices`: `Vec<f64>`
///   - A vector containing the mispricing (pᵢ) for each asset.
///   - **Mispricing Calculation**: pᵢ = Theoretical Priceᵢ - Market Priceᵢ.
///
/// - `transaction_costs`: `Vec<f64>`
///   - A vector containing the transaction costs (cᵢ) for each asset.
///   - Represents costs such as fees, commissions, or slippage.
///
/// - `capital`: `f64`
///   - The total capital (C) available for constructing the portfolio.
///   - Limits the total amount invested across all positions.
///
/// - `liquidity`: `Vec<f64>`
///   - A vector containing the liquidity limits (Lᵢ) for each asset.
///   - Ensures that the position size does not exceed market liquidity.
///
/// # Returns
///
/// - `Vec<f64>`
///   - A vector of optimal position sizes (wᵢ) for each asset.
///   - Positive values indicate **long positions** (buying options).
///   - Negative values indicate **short positions** (selling/writing options).
///
/// # Function Workflow
///
/// 1. **Variable Definition**:
///    - Decision variables wᵢ are created for each asset, representing the position sizes.
///    - Bounds are set to allow both long and short positions: -C ≤ wᵢ ≤ C.
///
/// 2. **Objective Function Setup**:
///    - The objective is to maximize total arbitrage profit:
///      Profit = ∑(i=1 to n) ((pᵢ - cᵢ) × wᵢ)
///
/// 3. **Constraints Addition**:
///    - **Capital Constraint**:
///      - Total invested capital should not exceed available capital:
///        ∑(i=1 to n) wᵢ ≤ C
///
///    - **Liquidity Constraints**:
///      - Position sizes should not exceed liquidity limits:
///        wᵢ ≤ Lᵢ, for all i
///
/// 4. **Problem Solving**:
///    - The linear programming problem is solved using an appropriate solver.
///    - Optimal position sizes wᵢ are obtained.
///
/// # Example Usage
///
/// ```rust
/// use your_crate::find_arbitrage;
///
/// // Mispricing for each asset (theoretical price - market price)
/// let prices = vec![2.5, -1.0, 0.5];
///
/// // Transaction costs for each asset
/// let transaction_costs = vec![0.05, 0.05, 0.05];
///
/// // Total capital available
/// let capital = 10000.0;
///
/// // Liquidity limits for each asset
/// let liquidity = vec![100.0, 150.0, 80.0];
///
/// // Find optimal arbitrage positions
/// let weights = find_arbitrage(prices, transaction_costs, capital, liquidity);
///
/// // Display the results
/// for (i, weight) in weights.iter().enumerate() {
///     println!("Asset {}: Position Size = {:.2}", i + 1, weight);
/// }
/// ```
///
/// # Notes
///
/// - **Short Positions**:
///   - Negative position sizes indicate that the strategy involves selling or writing options.
///   - Useful when an option is overpriced compared to its theoretical value.
///
/// - **Long Positions**:
///   - Positive position sizes indicate buying options.
///   - Useful when an option is underpriced.
///
/// - **Transaction Costs**:
///   - Incorporating transaction costs ensures that the strategy accounts for real-world trading expenses.
///
/// - **Solver Assumptions**:
///   - The function assumes that the solver can find an optimal solution.
///   - If the solver fails, the function may panic or return an error.
///
/// # Mathematical Concepts
///
/// - **Linear Programming**:
///   - A method to achieve the best outcome in a mathematical model whose requirements are represented by linear relationships.
///   - In this context, it's used to maximize profit while satisfying constraints.
///
/// - **Arbitrage Opportunity**:
///   - Occurs when there is a price difference between the theoretical value and the market price of an option.
///   - The strategy exploits these differences to make risk-free profits.
///
/// # Implementation Details
///
/// - **Decision Variables**:
///   - Represented by the `weights` vector.
///   - Created dynamically to accommodate any number of assets.
///
/// - **Objective Function**:
///   - Implemented as an `Expression` that the solver can interpret.
///
/// - **Constraints**:
///   - Capital and liquidity constraints ensure practical feasibility.
///
/// # Limitations
///
/// - **Market Impact**:
///   - The model does not account for the potential market impact of large trades.
///
/// - **Risk Factors**:
///   - Assumes that prices will move towards their theoretical values.
///   - Does not account for market volatility or unexpected events.
///
/// # References
///
/// - **Black-Scholes Model**:
///   - Used to compute theoretical option prices (see `black_scholes_price` function).
///
/// - **Linear Programming Solvers**:
///   - The `good_lp` crate is used for formulating and solving the linear programming problem.
///
/// # See Also
///
/// - `construct_portfolio`: Uses this function to build an optimal portfolio based on current market data.
///
/// # Function Definition
///
pub fn find_arbitrage(
    prices: Vec<f64>,
    transaction_costs: Vec<f64>,
    capital: f64,
    liquidity: Vec<f64>,
) -> Vec<f64> {
    let num_assets = prices.len(); // The number of options (assets) we are optimizing over.

    // Create a new problem variable instance
    let mut vars = ProblemVariables::new();

    // Define the decision variables (weights for each option) dynamically with bounds.
    // Each `weight[i]` represents how much of option `i` to buy (positive) or sell (negative).
    // The bounds `min(-capital)` and `max(capital)` ensure the position sizes do not exceed available capital,
    // and they allow both long (positive) and short (negative) positions.
    let weights: Vec<_> = (0..num_assets)
        .map(|_i| vars.add(variable().min(-capital).max(capital))) // Allow short (-capital) and long (+capital) positions
        .collect();

    // Set up the objective function for maximizing arbitrage profit.
    // This maps over the `num_assets`, representing each option.
    // The formula here corresponds to the mathematical objective function:
    //
    // Maximize:
    // ∑(i=1 to n) ((pᵢ - cᵢ) × wᵢ)
    //
    // Explanation:
    // - `prices[i]` represents the mispricing (pᵢ) of asset `i`, calculated as (theoretical price - market price).
    // - `transaction_costs[i]` represents the cost (cᵢ) of trading asset `i` (e.g., fees, slippage).
    // - `weights[i]` represents the position size (wᵢ) for asset `i`, i.e., how much of this asset to buy/sell.
    //
    // The expression `-(prices[i] - transaction_costs[i]) * weights[i]` captures the total profit for each option:
    // - If `prices[i] > transaction_costs[i]` (underpriced), a long position (positive weight) is profitable.
    // - If `prices[i] < transaction_costs[i]` (overpriced), a short position (negative weight) is profitable.
    //
    // We use the sum across all assets to aggregate the total profit.
    let objective: Expression = (0..num_assets)
        .map(|i| -(prices[i] - transaction_costs[i]) * weights[i])
        .sum(); // Summing over all `i` (each asset) to get the total profit across all positions.

    // Create the linear programming problem.
    // The solver will maximize the objective function defined above, which seeks to maximize arbitrage profit.
    let mut problem = vars.maximise(objective).using(default_solver);

    // Add a capital constraint:
    // The total capital invested (sum of weights) should not exceed the available capital (C).
    // This corresponds to the constraint: ∑(i=1 to n) wᵢ ≤ C
    problem = problem.with(constraint!(weights.iter().sum::<Expression>() <= capital));

    // Add individual liquidity constraints:
    // Each position size (wᵢ) should not exceed the available liquidity for that option (Lᵢ).
    // This ensures the positions respect market liquidity constraints.
    for i in 0..num_assets {
        problem = problem.with(constraint!(weights[i] <= liquidity[i]));
    }

    // Solve the problem:
    // The solver will find the optimal weights (wᵢ) that maximize the arbitrage profit while satisfying
    // the capital and liquidity constraints.
    let solution = problem.solve().unwrap();

    // Retrieve the optimal weights (position sizes) for each asset.
    // The `solution.value(var)` extracts the optimal position size for each option (wᵢ).
    weights.iter().map(|&var| solution.value(var)).collect()
}

/// Portfolio construction function
/// This is only intended for demonstration purposes and should not be used in production
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
