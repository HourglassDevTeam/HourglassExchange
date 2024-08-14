use std::sync::Arc;

use tokio::{sync::mpsc::unbounded_channel, task};
use std::sync::LazyLock;

use unilink_execution::{
    common_infrastructure::datafeed::event::MarketEvent,
    sandbox::{account::account_market_feed::*, clickhouse_api::queries_operations::*},
};
use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;

static CLIENT: LazyLock<Arc<ClickHouseClient>> = LazyLock::new(|| {
    Arc::new(ClickHouseClient::new())
});

#[tokio::main]
async fn main() {
    // 直接使用全局的 CLIENT
    let client = CLIENT.clone();

    // 定义单个参数集
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = "2024_03_04";
    let end_date = "2024_03_05";
    let batch_size = 1000000;

    // 创建 AccountMarketStreams 实例
    let mut account_streams: AccountDataStreams<MarketEvent<ClickhousePublicTrade>> = AccountDataStreams::new();

    // 创建一个唯一的 stream_id
    let stream_id = format!("{}_{}_{}", exchange, instrument, channel);

    // 创建 unbounded_channel
    let (tx, rx) = unbounded_channel::<MarketEvent<ClickhousePublicTrade>>();

    // 将接收者添加到 AccountMarketStreams 中
    account_streams.add_stream(stream_id.clone(), rx);

    // 创建异步任务并将句柄存储到 handle 中
    let handle = task::spawn(async move {
        match client.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, batch_size).await {
            Ok(mut rx) => {
                while let Some(event) = rx.recv().await {
                    if let Err(e) = tx.send(event) {
                        eprintln!("Failed to send market event: {:?}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to query events: {:?}", e);
            }
        }
    });

    // 等待异步任务完成
    if let Err(e) = handle.await {
        eprintln!("Task failed: {:?}", e);
    }
}
