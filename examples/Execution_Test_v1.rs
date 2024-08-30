use chrono::{Duration as ChronoDuration, NaiveDate};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::test_utils::create_test_account;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 创建测试账户实例，并将其封装在 Arc<Mutex<>> 中
    let account = Arc::new(Mutex::new(create_test_account().await));

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";

    // 定义日期范围
    let start_date = NaiveDate::from_ymd_opt(2024, 5, 5).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2024, 5, 5).unwrap();

    // 逐日遍历日期范围
    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = Arc::new(current_date.format("%Y-%m-%d").to_string());

        // 获取游标，这里假设你有一个方法 cursor_union_trades 用于查询 union 表
        let cursor_result = client.cursor_unioned_public_trades(exchange, instrument, &date_str).await;

        match cursor_result {
            Ok(mut cursor) => {
                // 创建通道
                let (tx, mut _rx) = mpsc::unbounded_channel();

                // 克隆 Arc 以在异步任务中使用
                let date_str_clone = Arc::clone(&date_str);
                let account_clone = Arc::clone(&account);

                // 启动一个任务来从游标读取数据并发送到通道
                let cursor_task = tokio::spawn(async move {
                    loop {
                        match cursor.next().await {
                            Ok(Some(trade)) => {
                                // 打印每个交易数据
                                println!("[{}]: {:?}", date_str_clone, trade);

                                if tx.send(trade).is_err() {
                                    eprintln!("[UniLinkExecution] : Failed to send trade, receiver might be closed.");
                                    break;
                                }
                            }
                            Ok(None) => {
                                println!("[UniLinkExecution] : Cursor data processing for date {} is complete.", date_str_clone);
                                break;
                            }
                            Err(_e) => {
                                eprintln!("[UniLinkExecution] : No data available for date {}. Skipping to next date.", date_str_clone);
                                break;
                            }
                        }
                    }
                });

                // 启动一个接收器任务，处理通道中的交易数据
                let receiver_task = tokio::spawn(async move {
                    while let Some(trade) = _rx.recv().await {
                        let mut account_locked = account_clone.lock().await;
                        if let Err(e) = account_locked.handle_trade_data(trade).await {
                            eprintln!("[UniLinkExecution] : Error handling trade data: {:?}", e);
                        }
                    }
                });

                // 等待任务完成
                if let Err(e) = cursor_task.await {
                    eprintln!("[UniLinkExecution] : Cursor task for {} was aborted: {:?}", date_str, e);
                }

                if let Err(e) = receiver_task.await {
                    eprintln!("[UniLinkExecution] : Receiver task was aborted: {:?}", e);
                }
            }
            Err(e) => {
                eprintln!("[UniLinkExecution] : Error fetching trades for {}: {:?}", date_str, e);
            }
        }

        // 更新到下一天
        current_date += ChronoDuration::days(1);
    }
}
