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
    let table_names = client.get_table_names(&database).await;

    // 计算总天数
    let total_days = (end_date - start_date).num_days() + 1;
    // 获取所有表名总数，用于进度汇报
    let mut total_tables = 0;
    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();
        total_tables += table_names
            .iter()
            .filter(|table_name| table_name.contains(&date_str) && table_name.contains("union"))
            .count();
        current_date += Duration::days(1);
    }

    // 重置日期，开始处理
    start_date = NaiveDate::from_ymd_opt(2024, 3, 4).expect("Invalid start date");

    // 初始化已处理的表数
    let mut processed_tables = 0;

    // 遍历日期范围
    while start_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = start_date.format("%Y_%m_%d").to_string();

        // 获取当前日期的所有表名

        // 筛选出包含 "union" 和当前日期字样的表名
        for table_name in &table_names {
            if table_name.contains("union") && table_name.contains(&date_str) {
                let table_path = format!("{}.{}", database, table_name);
                println!("Optimizing table: {}", table_path);

                // 执行优化操作
                if let Err(e) = client.optimize_table(&table_path).await {
                    eprintln!("Error optimizing table {}: {}", table_path, e);
                }

                // 更新已处理的表数
                processed_tables += 1;

                // 打印当前总进度
                let progress = (processed_tables as f64 / total_tables as f64) * 100.0;
                println!(
                    "Date: {} - Total tables processed: {}/{} (Total progress: {:.2}%)",
                    date_str, processed_tables, total_tables, progress
                );
            }
        }

        // 迭代到下一天
        start_date += Duration::days(1);
    }

    println!(
        "Optimization is complete for {} tables across {} days.",
        total_tables, total_days
    );
}
