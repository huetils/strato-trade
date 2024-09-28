use rand::Rng;
use strato_model::pricing::bs::black_scholes_call;
use strato_model::pricing::bs::black_scholes_put;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Generate options using the slightly incorrect pricing model
    let mut option_data = Vec::new();
    for i in 0..10 {
        let mut rng = rand::thread_rng();
        let option_type = if rng.gen_bool(0.5) { "call" } else { "put" };
        let name = format!("Option{}", i + 1);
        let s = 100.0 + rng.gen_range(0.0..=i as f64); // Varying underlying price
        let k = 90.0 + rng.gen_range(0.0..=i as f64 * 1.5); // Varying strike price
        let t = rng.gen_range(0.1..=1.0); // Time to maturity (random range)
        let r = rng.gen_range(0.01..=0.1); // Risk-free rate (random range)
        let sigma = rng.gen_range(0.1..=0.5); // Volatility (random range)

        // Price using the incorrect Black-Scholes model with added randomness
        let market_price = if option_type == "call" {
            black_scholes_call(s, k, t, r, sigma) + rng.gen_range(-1.0..=1.0)
        } else {
            black_scholes_put(s, k, t, r, sigma) + rng.gen_range(-1.0..=1.0)
        };

        option_data.push(strato_model::mft::stochastic_arbitrage::OptionData {
            name,
            s,
            k,
            t,
            r,
            sigma,
            option_type: option_type.to_string(),
            market_price,
        });
    }

    // Append an option with a correct price using the provided Black-Scholes model
    let correct_price = black_scholes_call(100.0, 90.0, 0.5, 0.05, 0.2);
    option_data.push(strato_model::mft::stochastic_arbitrage::OptionData {
        name: "Correctly Priced Option".to_string(),
        s: 100.0,
        k: 90.0,
        t: 0.5,
        r: 0.05,
        sigma: 0.2,
        option_type: "call".to_string(),
        market_price: correct_price,
    });

    // pretty print the option data
    for option in &option_data {
        println!(
            "{}: S={}, K={}, T={}, R={}, Sigma={}, Type={}, Market Price={}",
            option.name,
            option.s,
            option.k,
            option.t,
            option.r,
            option.sigma,
            option.option_type,
            option.market_price
        );
    }

    // Total capital for the portfolio
    let capital = 14_000_000.0;

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
    let liquidity = vec![f64::INFINITY; option_data.len()];

    // Construct the portfolio
    let portfolio = strato_model::mft::stochastic_arbitrage::construct_portfolio(
        option_data,
        capital,
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
