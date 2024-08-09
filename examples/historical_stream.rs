use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::{sync::mpsc::unbounded_channel, task};

use unilink_execution::{
    common_skeleton::datafeed::event::MarketEvent,
    simulated::{account::account_market_feed::*, clickhouse_api::queries_operations::*},
};

lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}

#[tokio::main]
async fn main()
{
    let client = Arc::new(ClickHouseClient::new());

    // 定义单个参数集
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = "2024_03_03";
    let end_date = "2024_07_03";
    let batch_size = 1000000;

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhouseTrade>> = AccountDataStreams::new();

    // 创建一个唯一的 stream_id
    let stream_id = format!("{}_{}_{}", exchange, instrument, channel);

    // 创建 unbounded_channel
    let (tx, rx) = unbounded_channel::<MarketEvent<ClickhouseTrade>>();

    // 将接收者添加到 AccountMarketStreams 中
    account_streams.add_stream(stream_id.clone(), rx);

    // 创建异步任务并将句柄存储到 handles 向量中
    let handle = task::spawn(async move {
        match client.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, batch_size)
                    .await
        {
            | Ok(mut rx) => {
                while let Some(event) = rx.recv().await {
                    println!("{:?}", event); // NOTE 调试开关
                    if tx.send(event).is_err() {
                        eprintln!("发送市场事件失败");
                        break;
                    }
                }
            }
            | Err(e) => {
                eprintln!("查询事件失败: {}", e);
            }
        }
    });

    // 等待异步任务完成
    if let Err(e) = handle.await {
        eprintln!("任务失败: {:?}", e);
    }
}
