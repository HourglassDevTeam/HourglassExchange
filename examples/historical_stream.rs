use std::sync::Arc;
use lazy_static::lazy_static;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::Mutex;
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

    // 创建一个 Arc<Mutex<HashMap>> 来存储所有的发送者
    let tx_map = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Voila.录入循环开始。
    for (exchange, instrument, channel, start_date, end_date, batch_size) in stream_params {
        let client = client.clone();
        let stream_id = format!("{}_{}_{}", exchange, instrument, channel);

        // 在循环外部创建 unbounded_channel
        let (tx, rx_clone) = unbounded_channel::<MarketEvent<ClickhouseTrade>>();

        // 将发送者插入到 HashMap 中
        tx_map.lock().await.insert(stream_id.clone(), tx);
        account_streams.add_stream(stream_id.clone(), rx_clone);

        // 克隆 Arc 指针以便在异步任务中使用
        let tx_map = Arc::clone(&tx_map);

        tokio::spawn(async move {
            match client.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, batch_size).await {
                Ok(mut rx) => {
                    while let Some(event) = rx.recv().await {
                        let tx_map = tx_map.lock();
                        if let Some(tx) = tx_map.await.get(&stream_id) {
                            if tx.send(event).is_err() {
                                eprintln!("Failed to send event");
                                break;
                            }
                        }
                    }
                    println!("Finished processing events for stream: {}", stream_id);
                }
                Err(e) => {
                    eprintln!("Failed to query events: {}", e);
                }
            }
        });
    }
}