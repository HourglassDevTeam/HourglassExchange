use chrono::{Duration, NaiveDate};
use futures::future::join_all;
use futures::TryFutureExt;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2019, 11, 4).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 12).expect("Invalid end date"); // 设置结束日期
    let database = format!("{}_{}_{}", exchange, instrument, channel);

    // 计算总天数，用于进度汇报
    let total_days = (end_date - start_date).num_days() + 1;

    // 获取所有表名，只获取一次
    let table_names = match client.get_table_names(&database).await {
        Ok(names) => names,
        Err(_) => vec![], // 处理错误情况
    };

    // 遍历日期范围
    let mut processed_days = 0;
    let mut current_date = start_date;

    while current_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = current_date.format("%Y_%m_%d").to_string();
        processed_days += 1; // 更新已处理的天数

        // 筛选出当前日期相关的表名，并且包含 "union" 字样
        let filtered_tables: Vec<String> = table_names
            .iter()
            .filter(|table_name| table_name.contains(&date_str) && table_name.contains("union"))
            .cloned()
            .collect();

        // 并行执行表优化操作
        let optimization_tasks: Vec<_> = filtered_tables
            .into_iter()
            .map(|table_name| {
                let table_path = format!("{}.{}", database, table_name);
                println!("Optimizing table: {}", table_path);

                // 将 table_path 移动到闭包中
                client.optimize_table(&table_path.clone()).map_err(move |e| {
                    eprintln!("Error optimizing table {}: {}", table_path, e);
                    e
                })
            })
            .collect();

        let results = join_all(optimization_tasks).await;
        let processed_tables = results.len();

        // 打印当前日期处理的进度
        let progress = (processed_days as f64 / total_days as f64) * 100.0;
        println!(
            "Date: {} - Tables processed: {}/{}, Total progress: {:.2}%",
            date_str, processed_tables, filtered_tables.len(), progress
        );

        // 迭代到下一天
        current_date += Duration::days(1);
    }

    println!("Optimization is complete for {} days.", total_days);
}
