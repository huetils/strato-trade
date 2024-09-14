use rand::Rng;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Generate 50 option instruments with varying data (calls and puts)
    let mut option_data = Vec::new();
    for i in 0..5 {
        let mut rng = rand::thread_rng();
        let option_type = if rng.gen_bool(0.5) { "call" } else { "put" };
        let name = format!("Option{}", i + 1);
        let s = 100.0 + rng.gen_range(0.0..=i as f64); // Varying underlying price
        let k = 90.0 + rng.gen_range(0.0..=i as f64 * 1.5); // Varying strike price
        let t = rng.gen_range(0.1..=1.0); // Time to maturity (random range)
        let r = rng.gen_range(0.01..=0.1); // Risk-free rate (random range)
        let market_price = if option_type == "call" {
            8.0 + rng.gen_range(0.0..=i as f64 * 0.1) // Varying market price for calls
        } else {
            12.0 + rng.gen_range(0.0..=i as f64 * 0.2) // Varying market price for puts
        } + rng.gen_range(-1.0..=1.0); // Add additional randomness to market price

        option_data.push(strato_model::mft::stochastic_arbitrage::OptionData {
            name,
            s,
            k,
            t,
            r,
            option_type: option_type.to_string(),
            market_price,
        });
    }

    // push where the market price is 1
    option_data.push(strato_model::mft::stochastic_arbitrage::OptionData {
        name: "Option6".to_string(),
        s: 100.0,
        k: 90.0,
        t: 0.5,
        r: 0.05,
        option_type: "call".to_string(),
        market_price: 1.0,
    });

    // Total capital for the portfolio
    let capital = 100.0;

    // Risk levels for various investor profiles
    let risk_levels = &[0.01, 0.1, 0.5];

    // Simulated or historical index returns (should match the number of assets)
    let index_returns = vec![1.5, 0.5, 0.2, 1.0, 0.8, 0.7, 1.1, 0.9, 0.6, 0.4];
    let index_returns = index_returns
        .iter()
        .cycle()
        .take(50)
        .cloned()
        .collect::<Vec<_>>();

    // Transaction costs per option (randomized for diversity)
    let transaction_costs = vec![0.05; 50]; // Assume constant transaction costs for now

    // Liquidity constraints (allowing large positions)
    let liquidity = vec![1000.0; 50]; // Ensures positions aren't excessively large

    // Adjust risk aversion
    let risk_aversion = 0.1; // Moderate risk aversion

    // Construct the portfolio
    let portfolio = strato_model::mft::stochastic_arbitrage::construct_portfolio(
        option_data,
        capital,
        risk_aversion,
        risk_levels,
        index_returns,
        transaction_costs,
        liquidity,
    );

    // Output the portfolio holdings
    for (name, position) in portfolio.holdings {
        println!("Option: {}, Position: {:.2}", name, position);
    }

    Ok(())
}
