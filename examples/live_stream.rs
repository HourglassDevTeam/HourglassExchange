// 引入所需的模块
use cerebro_data::{
    exchange::okx::Okx,
    streams::Streams,
    subscription::trade::PublicTrades,
};
use cerebro_data::exchange::ExchangeId;
use cerebro_integration::model::instrument::kind::InstrumentKind;

#[tokio::main]
async fn main() {
    let mut streams = Streams::<PublicTrades>::builder()
        // 添加 PublicTrades 流
        .subscribe([
        (Okx::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
    ])
        .init()
        .await
        .unwrap();

    let mut okx_feed = streams
        .select(ExchangeId::Okx)
        .unwrap();

    let live_feed = okx_feed.unwrap();

    while let Some(trade) = okx_feed.recv().await {
        println!("MarketEvent<PublicTrade>: {trade:?}");
    }
}
