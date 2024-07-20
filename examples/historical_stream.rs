use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::sync::mpsc::unbounded_channel;

use unilink_execution::{
    common_skeleton::datafeed::event::MarketEvent,
    simulated_exchange::{account::account_market_feed::*, load_from_clickhouse::queries_operations::*},
};

// 使用 lazy_static 宏来创建一个全局静态变量 CLIENT
lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}

#[tokio::main]
async fn main()
{
    // 定义交易所、金融工具、频道和日期的字符串变量
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", 1000000)];

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhouseTrade>> = AccountDataStreams::new();

    // 循环创建和添加数据流
    for (exchange, instrument, channel, date, batch_size) in stream_params {
        // 调用 CLIENT 的 query_unioned_trade_table_batched_for_dates 方法获取数据流
        let events = CLIENT.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, date, date, batch_size)
                           .await;
        println!("Query returned {} events", events.len());

        // 创建一个 MPSC 通道
        let (tx, rx) = unbounded_channel::<MarketEvent<ClickhouseTrade>>();

        // 将通道接收端添加到 AccountDataStreams
        let stream_id = format!("{}_{}_{}_{}", exchange, instrument, channel, date);
        account_streams.add_stream(stream_id.clone(), rx);
        println!("Hooooray ! New stream has been added.");

        // 遍历事件并打印和发送
        for event in events {
            println!("{:?}", event);
            if tx.send(event).is_err() {
                eprintln!("Failed to send event");
                break;
            }
        }

    }
}
