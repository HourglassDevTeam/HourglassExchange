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
    let client = Arc::new(ClickHouseClient::new());

    // 定义Exchange、Instrument、Channel和起止日期的字符串变量
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", "2024_07_03", 1000000)];

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhouseTrade>> = AccountDataStreams::new();

    // Voila.录入循环开始。
    for (exchange, instrument, channel, start_date, end_date, batch_size) in stream_params {
        let client = client.clone();
        match client.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, batch_size)
                    .await
        {
            | Ok(mut rx) => {
                // FIXME 注意这个 unbounded_channel 在循环内部构建了 可能不是最佳实践。
                let (tx, rx_clone) = unbounded_channel::<MarketEvent<ClickhouseTrade>>();

                let stream_id = format!("{}_{}_{}", exchange, instrument, channel);
                account_streams.add_stream(stream_id.clone(), rx_clone);
                println!("Hooooray! New stream has been added.");

                while let Some(event) = rx.recv().await {
                    // println!("Received event: {:?}", event);  //打印开关
                    if tx.send(event).is_err() {
                        eprintln!("Failed to send event");
                        break;
                    }
                }
                println!("Finished processing events for stream: {}", stream_id);
            }
            | Err(e) => {
                eprintln!("Failed to query events: {}", e);
            }
        }
    }
}
