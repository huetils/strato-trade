# strato-client

```rust
/// The main function that runs the trading strategy.
///
/// This function receives real-time market data, processes it using the TradingState methods,
/// and executes trades based on the Parametrized Linear Model and the specified thresholds.
#[tokio::main]
async fn main() {
    init_logging();

    let mut trading_state = TradingState::new(1000.0, "BTC/USDT");

    let streams = Streams::<OrderBooksL2>::builder()
        .subscribe([(Aevo, "btc", "usd", InstrumentKind::Perpetual, OrderBooksL2)])
        .init()
        .await
        .unwrap();

    let mut joined_stream = streams.join().await;

    while let Some(market_event) = joined_stream.recv().await {
        let order_book = market_event.kind;
        let bid: f64 = order_book.bids.levels[0].price;
        let ask: f64 = order_book.asks.levels[0].price;
        let spread: f64 = TradingState::calculate_spread(bid, ask);
        let last_price: f64 = (bid + ask) / 2.0;

        // Calculate volume order imbalance
        let bid_volume: f64 = order_book.bids.levels.iter().map(|bid| bid.amount).sum();
        let ask_volume: f64 = order_book.asks.levels.iter().map(|ask| ask.amount).sum();
        let current_voi: f64 = TradingState::calculate_voi(bid_volume, ask_volume);

        // Calculate Order Imbalance Ratio (OIR)
        let current_oir: f64 = TradingState::calculate_oir(bid_volume, ask_volume);

        // Calculate Mid-Price
        let mid_price = TradingState::calculate_mid_price(bid, ask);

        // Calculate Mid-Price Basis (MPB)
        let current_mpb: f64 = TradingState::calculate_mpb(last_price, mid_price);

        // Check if a trade should be made
        if TradingState::is_threshold_constrained(spread, DEFAULT_SPREAD_THRESHOLD)
            && TradingState::is_voi_detected(current_voi)
        {
            // Get trading signal
            let signal = trading_state.parametrized_linear_model(current_voi, current_oir, current_mpb, None, None);

            // Execute trade based on signal
            if signal > 0.0 {
                trading_state.execute_trade(bid, Side::Buy, TRADE_SIZE, DEFAULT_TRANSACTION_COST);
            } else if signal < 0.0 {
                trading_state.execute_trade(ask, Side::Sell, TRADE_SIZE, DEFAULT_TRANSACTION_COST);
            }
        }

        // Calculate the current portfolio value
        let portfolio_value = trading_state.calculate_portfolio_value(bid);
        debug!(
            "Current portfolio value: ${:.2} at {}",
            portfolio_value,
            Utc::now()
        );
    }
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .pretty()
        // Install this Tracing subscriber as global default
        .init()
}
```
