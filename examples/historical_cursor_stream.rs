use unilink_execution::sandbox::account::account_market_feed::AccountDataStreams;
use std::{sync::Arc, time::Duration};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use tokio::{sync::mpsc, time::timeout};

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024-05-05";
    let base = "1000BONK";
    let quote = "USDT";

    // 获取游标
    let cursor_result = client.cursor_public_trades(exchange, instrument, date, base, quote).await;

    match cursor_result {
        Ok(mut cursor) => {
            // 创建通道
            let (tx, rx) = mpsc::unbounded_channel();

            // 启动一个任务来从游标读取数据并发送到通道
            let cursor_task = tokio::spawn(async move {
                loop {
                    match timeout(Duration::from_secs(5), cursor.next()).await {
                        Ok(Ok(Some(trade))) => {
                            println!("{:?}", &trade);
                            if tx.send(trade).is_err() {
                                // 如果发送失败（例如接收者已关闭），退出循环
                                eprintln!("[UniLinkExecution] : Failed to send trade, receiver might be closed.");
                                break;
                            }
                        }
                        Ok(Ok(None)) => {
                            println!("[UniLinkExecution] : Cursor data processing for date {} is complete.", date);
                            break;
                        }
                        Ok(Err(e)) => {
                            eprintln!("[UniLinkExecution] : Error while reading cursor: {:?}", e);
                            break;
                        }
                        Err(_) => {
                            eprintln!("[UniLinkExecution] : Timeout while reading cursor");
                            break;
                        }
                    }
                }
            });

            // 创建 AccountDataStreams 实例
            let mut data_streams = AccountDataStreams::new();

            // 将接收器添加到 AccountDataStreams
            data_streams.add_stream("binance_futures_trades".to_string(), rx);

            // 这里你可以选择等待 `cursor_task` 完成，或在其他地方进行更多操作
            if let Err(e) = cursor_task.await {
                eprintln!("[UniLinkExecution] : Cursor task was aborted: {:?}", e);
            }

            // 你可以继续使用 data_streams 进行进一步的操作，例如合并、排序等
        }
        Err(e) => {
            eprintln!("[UniLinkExecution] : Error fetching trades: {:?}", e);
        }
    }
}
