// 引入所需的模块
use cerebro_data::{
    event::{DataKind, MarketEvent},
    exchange::okx::Okx,
    streams::Streams,
    subscription::{
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use cerebro_integration::model::instrument::kind::InstrumentKind;
use tokio_stream::StreamExt;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // 为不同交易所初始化 MarketEvent<DataKind> 流
    let streams: Streams<MarketEvent<DataKind>> = Streams::builder_multi()
        // 为不同交易所添加 PublicTrades 流
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades),
                (Okx, "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
            ])
        )
        .init()
        .await
        .unwrap();

    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, data)) = joined_stream.next().await {
        println!("Exchange: {exchange}, MarketEvent<DataKind>: {data:?}");
    }
}
