
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

    // 调用 CLIENT 的 query_union_table_batched 方法获取数据流
    let stream = CLIENT.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, "2024_03_03","2024_07_03",10000);

    // 创建一个 HistoricalFeed 实例
    let feed = HistoricalFeed { database_client: CLIENT.to_owned(),
                                stream: Box::pin(stream) };

    let mut account_stream = AccountMarketStream::new(MarketStream::Historical(feed));


    loop {
        let result = account_stream.data_stream.next().await;
        match result {
            Some(data) => {
                println!("{:?}", data);
            },
            None => break,
        }
    }
}
