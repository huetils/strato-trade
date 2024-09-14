pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Example option data
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

    // Available capital for constructing the portfolio
    let capital = 10000.0; // Example capital

    // Construct the portfolio
    let portfolio =
        strato_model::mft::stochastic_arbitrage::construct_portfolio(option_data, capital);

    // Display the portfolio holdings
    for (name, position) in portfolio.holdings {
        println!("Option: {}, Position: {:.2}", name, position);
    }

    Ok(())
}
