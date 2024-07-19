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
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", 100000),
                             /* Add more parameters as needed */];

    // 创建 AccountMarketStreams 实例
    let mut account_streams = AccountMarketStreams::new();

    // 循环创建和添加数据流
    for (exchange, instrument, channel, date, batch_size) in stream_params {
        // 调用 CLIENT 的 query_unioned_trade_table_batched_for_dates 方法获取数据流
        let stream = CLIENT.query_unioned_trade_table_batched(exchange, instrument, channel, date, batch_size);

        // 创建一个 HistoricalFeed 实例
        let feed = HistoricalFeed { database_client: CLIENT.clone(), // 使用 to_owned 克隆 Arc<ClickHouseClient>
                                    stream: Box::pin(stream) };

        // 生成唯一的 stream_id 并将流添加到 account_streams 中
        let stream_id = format!("{}_{}_{}", exchange, instrument, channel);
        account_streams.add_stream(stream_id, MarketStream::Historical(feed));
    }

    // 循环处理所有数据流
    loop {
        for (id, stream) in account_streams.streams.iter_mut() {
            let result = stream.next().await;
            match result {
                | Some(data) => {
                    println!("数据流 {}: {:?}", id, data);
                }
                | None => continue,
            }
        }
    }
}
