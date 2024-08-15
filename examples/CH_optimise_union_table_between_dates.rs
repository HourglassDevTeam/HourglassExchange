use chrono::{Duration, NaiveDate};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let mut start_date = NaiveDate::from_ymd_opt(2024, 3, 4).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 7, 3).expect("Invalid end date"); // 设置结束日期
    let database = format!("{}_{}_{}", exchange, instrument, channel);

    // 计算总天数，用于进度汇报
    let total_days = (end_date - start_date).num_days() + 1;
    let mut processed_days = 0;

    // 遍历日期范围
    while start_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = start_date.format("%Y_%m_%d").to_string();
        processed_days += 1; // 更新已处理的天数

        // 获取当前日期的所有表名
        let table_names = client.get_table_names(&database).await;
        let mut processed_tables = 0;

        // 筛选出包含 "union" 字样的表名
        for table_name in &table_names {
            if table_name.contains("union") {
                let table_path = format!("{}.{}", database, table_name);
                println!("Optimizing table: {}", table_path);

                // 执行优化操作
                if let Err(e) = client.optimize_table(&table_path).await {
                    eprintln!("Error optimizing table {}: {}", table_path, e);
                }

                processed_tables += 1;
            }
        }

        // 打印当前日期处理的进度
        let progress = (processed_days as f64 / total_days as f64) * 100.0;
        println!("Date: {} - Tables processed: {}/{}, Total progress: {:.2}%",
                 date_str, processed_tables, table_names.len(), progress);

        // 迭代到下一天
        start_date += Duration::days(1);
    }

    println!("Optimization is complete for {} days.", total_days);
}