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

    // 注释：
    // - MarketEvent<DataKind> 可以使用自定义枚举来提供更多的灵活性。
    // - 每次调用 StreamBuilder::subscribe() 都会为传递的订阅创建一个新的 WebSocket 连接。

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

    // 将所有交易所流合并为一个单一的 tokio_stream::StreamMap
    // 注释：
    //  - 使用 `streams.select(ExchangeId)` 与单独的交易所流进行交互！
    //  - 使用 `streams.join()` 将所有交易所流合并成一个单一的 mpsc::UnboundedReceiver！
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, data)) = joined_stream.next().await {
        println!("Exchange: {exchange}, MarketEvent<DataKind>: {data:?}");
    }
}
