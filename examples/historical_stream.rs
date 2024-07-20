use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::{
    sync::{mpsc::unbounded_channel},
    task,
};
use unilink_execution::{
    common_skeleton::datafeed::event::MarketEvent,
    simulated_exchange::{account::account_market_feed::*, load_from_clickhouse::queries_operations::*},
};
lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}

#[tokio::main]
async fn main() {
    let client = Arc::new(ClickHouseClient::new());

    // 定义Exchange、Instrument、Channel和起止日期的字符串变量
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", "2024_07_03", 1000000)];

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhouseTrade>> = AccountDataStreams::new();

    // 存储所有的异步任务句柄
    let mut handles = Vec::new();

    // Voila.录入循环开始。
    for (exchange, instrument, channel, start_date, end_date, batch_size) in stream_params {
        let client = client.clone();
        let stream_id = format!("{}_{}_{}", exchange, instrument, channel);

        // 在循环外部创建 unbounded_channel
        let (tx, rx_clone) = unbounded_channel::<MarketEvent<ClickhouseTrade>>();

        account_streams.add_stream(stream_id.clone(), rx_clone);

        // 创建异步任务并将句柄存储到 handles 向量中
        let handle = task::spawn(async move {
            match client.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, batch_size).await {
                Ok(mut rx) => {
                    while let Some(event) = rx.recv().await {
                        // println!("{event:?}"); // NOTE 调试开关
                        if tx.send(event).is_err() {
                            eprintln!("Failed to send event");
                            break;
                        }
                    }
                    println!("Finished processing events for stream: {}", stream_id);
                }
                Err(e) => {
                    eprintln!("Failed to query events: {}", e);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有异步任务完成
    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task failed: {:?}", e);
        }
    }
}
