use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::sync::mpsc::unbounded_channel;

use unilink_execution::common_skeleton::datafeed::event::MarketEvent;
use unilink_execution::simulated_exchange::{account::account_market_feed::*, load_from_clickhouse::queries_operations::*};

// 使用 lazy_static 宏来创建一个全局静态变量 CLIENT
lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}

#[tokio::main]
async fn main() {
    // 定义交易所、金融工具、频道和日期的字符串变量
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", 1000000)];

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhouseTrade>> = AccountDataStreams::new();

    // 循环创建和添加数据流
    for (exchange, instrument, channel, date, batch_size) in stream_params {
        // 调用 CLIENT 的 query_unioned_trade_table_batched_for_dates 方法获取数据流
        let events = CLIENT
            .query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, date, date, batch_size)
            .await;

        // 创建一个 MPSC 通道
        let (tx, rx) = unbounded_channel();
        for event in events {
            tx.send(event).unwrap();
        }

        // 将通道接收端添加到 AccountDataStreams
        account_streams.add_stream(format!("{}_{}_{}_{}", exchange, instrument, channel, date), rx);
    }

    // 示例：处理每个数据流中的事件
    for (stream_id, mut receiver) in account_streams.streams {
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                println!("Received event in stream {}: {:?}", stream_id, event);
            }
        });
    }
}
