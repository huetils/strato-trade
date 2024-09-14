pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let option_data = vec![
        strato_model::mft::stochastic_arbitrage::OptionData {
            name: "Call1".to_string(),
            s: 100.0,
            k: 90.0,
            t: 0.5,
            r: 0.05,
            sigma: 0.2,
            option_type: "call".to_string(),
            market_price: 8.0,
        },
        strato_model::mft::stochastic_arbitrage::OptionData {
            name: "Put1".to_string(),
            s: 100.0,
            k: 110.0,
            t: 0.5,
            r: 0.05,
            sigma: 0.2,
            option_type: "put".to_string(),
            market_price: 12.0,
        },
    ];

    // Total capital available for constructing the portfolio
    let capital = 10000.0;

    // Define risk levels for different investor profiles
    let risk_levels = &[0.01, 0.1, 0.5, 1.0, 2.0];

    // Simulated or historical index returns
    let index_returns = vec![1.5, 0.5, 0.2];

    // Simulate transaction costs
    let transaction_costs: Vec<f64> = vec![0.01; option_data.len()];
    // Simulate liquidity limits
    let liquidity: Vec<f64> = vec![100.0; option_data.len()];

    // Construct the portfolio
    let portfolio = strato_model::mft::stochastic_arbitrage::construct_portfolio(
        option_data,
        capital,
        risk_levels,
        index_returns,
        transaction_costs,
        liquidity,
    );

    // Display portfolio holdings
    for (name, position) in portfolio.holdings {
        println!("Option: {}, Position: {:.2}", name, position);
    }

    Ok(())
}
