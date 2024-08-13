use hftbacktest::backtest::assettype::LinearAsset;
use hftbacktest::backtest::data::read_npz_file;
use hftbacktest::backtest::models::CommonFees;
use hftbacktest::backtest::models::IntpOrderLatency;
use hftbacktest::backtest::models::PowerProbQueueFunc3;
use hftbacktest::backtest::models::ProbQueueModel;
use hftbacktest::backtest::models::TradingValueFeeModel;
use hftbacktest::backtest::recorder::BacktestRecorder;
use hftbacktest::backtest::AssetBuilder;
use hftbacktest::backtest::Backtest;
use hftbacktest::backtest::DataSource;
use hftbacktest::backtest::ExchangeKind;
use hftbacktest::prelude::ApplySnapshot;
use hftbacktest::prelude::Bot;
use hftbacktest::prelude::HashMapMarketDepth;
use strato_model::hft::hft_oir::exec_backtest_hft_oir;

fn prepare_backtest() -> Backtest<HashMapMarketDepth> {
    let latency_data = (20240501..20240532)
        .map(|date| DataSource::File(format!("latency_{date}.npz")))
        .collect();

    let latency_model = IntpOrderLatency::new(latency_data);
    let asset_type = LinearAsset::new(1.0);
    let queue_model = ProbQueueModel::new(PowerProbQueueFunc3::new(3.0));

    let data = (20240501..20240532)
        .map(|date| DataSource::File(format!("1000SHIBUSDT_{date}.npz")))
        .collect();

    let hbt = Backtest::builder()
        .add_asset(
            AssetBuilder::new()
                .data(data)
                .latency_model(latency_model)
                .asset_type(asset_type)
                .fee_model(TradingValueFeeModel::new(CommonFees::new(-0.00005, 0.0007)))
                .queue_model(queue_model)
                .depth(|| {
                    let mut depth = HashMapMarketDepth::new(0.000001, 1.0);
                    depth.apply_snapshot(
                        &read_npz_file("1000SHIBUSDT_20240501_SOD.npz", "data").unwrap(),
                    );
                    depth
                })
                .exchange(ExchangeKind::NoPartialFillExchange)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    hbt
}

fn main() {
    tracing_subscriber::fmt::init();

    let order_qty = 1.0;

    let mut hbt = prepare_backtest();
    let mut recorder = BacktestRecorder::new(&hbt);

    exec_backtest_hft_oir(&mut hbt, &mut recorder, order_qty).unwrap();
    hbt.close().unwrap();
    recorder.to_csv("gridtrading", ".").unwrap();
}
