#![feature(async_iterator)]
// NOTE 这个示例是示范性，但是不健全的。并未把 [WsTrade] 流转化为 [MarketEvent] 流。
// ### 深度解释 `to_owned` 的作用
//
// `to_owned` 方法在这个代码中的作用是克隆 `CLIENT` 这个全局静态变量的 `Arc<ClickHouseClient>` 引用。`Arc` 是一种原子引用计数的智能指针，允许在多个线程之间安全地共享数据。当我们调用 `to_owned` 方法时，实际上是创建了一个新的 `Arc` 实例，该实例的引用计数增加了1，并且它仍然指向原来的 `ClickHouseClient` 实例。
//
// 具体来说，在这段代码中：
//
// 1. **全局静态变量 `CLIENT`**：
// - `CLIENT` 是一个 `Arc<ClickHouseClient>`，通过 `lazy_static` 宏初始化并全局共享。
// - 由于 `Arc` 的存在，`ClickHouseClient` 实例可以在程序的多个部分之间安全地共享，而不会因为过早释放而导致的问题。
//
// 2. **使用 `to_owned` 克隆 `Arc`**：
// - 在创建 `HistoricalFeed` 实例时，`database_client: CLIENT.to_owned()` 这一行代码中调用了 `to_owned` 方法。
// - 这个方法会创建一个新的 `Arc` 指针，这个指针与 `CLIENT` 共享相同的 `ClickHouseClient` 实例，同时将引用计数增加了1。
// - 这样，新的 `Arc` 指针可以在 `HistoricalFeed` 中使用，并确保 `ClickHouseClient` 实例在 `HistoricalFeed` 使用期间不会被释放。
//
// 通过这些设计，代码可以安全地在不同的任务或线程中共享 `ClickHouseClient` 实例，而不会遇到并发问题或过早释放的问题。这样不仅提高了代码的可维护性和稳定性，还确保了在高并发环境中的性能和安全性。

// 用于创建全局静态变量
use std::sync::Arc;

use futures::StreamExt;
use lazy_static::lazy_static;

// 用于原子引用计数的智能指针
use unilink_execution::{
    // 导入项目中的模块和结构体
    common_skeleton::datafeed::historical::HistoricalFeed,
    simulated_exchange::account::account_market_feed::*,
    simulated_exchange::load_from_clickhouse::queries_operations::*,
};

// 使用 lazy_static 宏来创建一个全局静态变量 CLIENT
lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}
#[tokio::main]
async fn main()
{
    // 定义交易所、金融工具、频道和日期的字符串变量
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_03_03";

    // 调用 CLIENT 的 query_union_table_batched 方法获取数据流
    let stream = CLIENT.query_unioned_trade_table_batched(exchange, instrument, channel, date, 100000);

    // 创建一个 HistoricalFeed 实例
    let feed = HistoricalFeed { database_client: CLIENT.to_owned(),
                                stream: Box::pin(stream) };

    let mut account_stream = AccountMarketStreams::new(MarketStream::Historical(feed));

    loop {
        let result = account_stream.data_stream.next().await;
        match result {
            | Some(data) => {
                println!("{:?}", data);
            }
            | None => break,
        }
    }
}
