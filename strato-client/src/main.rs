use rand::Rng;
use std::time::Duration;
use std::time::Instant;
use strato_model::grid::dynamic::manage_grids;
use strato_model::grid::dynamic::GridParams;
use strato_utils::vars::ohlc::Ohlc;

fn generate_candle(previous_close: f64, _sentiment: &str, direction: &mut bool) -> Ohlc {
    let mut rng = rand::thread_rng();
    let max_change = 0.05; // max 5% change per ohlc

    let open = previous_close;
    let close = if *direction {
        (previous_close * (1.0 + rng.gen_range(0.0..max_change))).clamp(100.0, 500.0)
    } else {
        (previous_close * (1.0 - rng.gen_range(0.0..max_change))).clamp(100.0, 500.0)
    };

    let high = (open.max(close) * (1.0 + rng.gen_range(0.0..0.02))).clamp(100.0, 500.0);
    let low = (open.min(close) * (1.0 - rng.gen_range(0.0..0.02))).clamp(100.0, 500.0);

    // Change direction if the price hits the upper or lower bound
    if close >= 500.0 {
        *direction = false;
    } else if close <= 100.0 {
        *direction = true;
    }

    Ohlc {
        open,
        high,
        low,
        close,
    }
}

fn test_execute_trades(
    ohlc_collection: &[Ohlc],
    entry_conditions: &[bool],
    exit_conditions: &[bool],
    initial_balance: f64,
) -> (f64, usize, usize, usize, f64) {
    let fee_percentage = 0.0005; // 0.05% fee
    let mut balance = initial_balance;
    let mut total_trades = 0;
    let mut winning_trades = 0;
    let mut losing_trades = 0;
    let mut drawdown = 0.0;
    let mut peak_balance = initial_balance;

    for (i, ohlc) in ohlc_collection.iter().enumerate() {
        if entry_conditions[i] {
            total_trades += 1;
            let entry_price = ohlc.close;
            let mut exit_price = entry_price;

            // Simulate the trade exit
            for j in i..ohlc_collection.len() {
                if exit_conditions[j] {
                    exit_price = ohlc_collection[j].close;
                    break;
                }
            }

            let trade_profit = exit_price - entry_price;
            let fee = fee_percentage * ((entry_price + exit_price) / 2.0);
            let net_profit = trade_profit - fee;
            balance += net_profit;

            if net_profit > 0.0 {
                winning_trades += 1;
            } else {
                losing_trades += 1;
            }

            if balance > peak_balance {
                peak_balance = balance;
            }

            let current_drawdown = (peak_balance - balance) / peak_balance;
            if current_drawdown > drawdown {
                drawdown = current_drawdown;
            }
        }
    }

    (
        balance,
        total_trades,
        winning_trades,
        losing_trades,
        drawdown,
    )
}

fn main() {
    let initial_balance = 100.0;
    let params = GridParams::default();
    let mut ohlc_collection = vec![];
    let mut current_price = 100.0;
    let mut sentiment = "bullish";
    let mut direction = true; // true for upward trend, false for downward trend
    let mut last_switch = Instant::now();

    while ohlc_collection.len() < 1440 {
        if last_switch.elapsed() > Duration::from_secs(30) {
            sentiment = if sentiment == "bullish" {
                "bearish"
            } else {
                "bullish"
            };
            last_switch = Instant::now();
        }

        let ohlc = generate_candle(current_price, sentiment, &mut direction);
        current_price = ohlc.close;
        ohlc_collection.push(ohlc);
        println!(
            "Ohlc {}: Open: {:.2}, High: {:.2}, Low: {:.2}, Close: {:.2}",
            ohlc_collection.len(),
            ohlc.open,
            ohlc.high,
            ohlc.low,
            ohlc.close
        );
    }

    let (entry_conditions, exit_conditions) = manage_grids(&ohlc_collection, &params);
    let (final_balance, total_trades, winning_trades, losing_trades, drawdown) =
        test_execute_trades(
            &ohlc_collection,
            &entry_conditions,
            &exit_conditions,
            initial_balance,
        );

    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    println!("Final Balance: {}", final_balance);
    println!("Total Trades: {}", total_trades);
    println!("Winning Trades: {}", winning_trades);
    println!("Losing Trades: {}", losing_trades);
    println!("Win Rate: {:.2}%", win_rate);
    println!("Drawdown: {:.2}%", drawdown * 100.0);
}
