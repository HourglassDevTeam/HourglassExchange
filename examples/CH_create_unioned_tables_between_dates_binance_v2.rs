use std::collections::HashMap;
use rayon::prelude::*;
use chrono::{Duration, NaiveDate};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::utils::chrono_operations::extract_date;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2019, 12, 7).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 3, 3).expect("Invalid end date"); // 设置结束日期
    let mut current_date = start_date;

    // 计算总天数，用于进度汇报
    let total_days = (end_date - start_date).num_days() + 1;
    let mut processed_days = 0;

    // 在循环外部获取所有的表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    // 创建一个表名与日期的字典
    let table_date_map: HashMap<String, String> = table_names
        .iter()
        .filter_map(|table_name| {
            if let Some(table_date) = extract_date(table_name) {
                Some((table_name.clone(), table_date))
            } else {
                None
            }
        })
        .collect();

    // 遍历日期范围
    while current_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = current_date.format("%Y_%m_%d").to_string();

        // 更新进度
        processed_days += 1;
        let progress = (processed_days as f64 / total_days as f64) * 100.0;
        println!("Processing tables on date : {} ({:.2}%)", date_str, progress); // 打印当前处理的日期和进度

        // 筛选出与当前日期匹配的表名
        let tables: Vec<String> = table_date_map
            .par_iter()
            .filter(|&(_, table_date)| table_date == &date_str)
            .map(|(table_name, _)| table_name.clone())
            .collect();

        // 如果找到对应的表，则创建联合表
        if !tables.is_empty() {
            let new_table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date_str);
            match client.create_unioned_table_for_date(&database, &new_table_name, &tables, true).await {
                Ok(_) => println!("Successfully created table: {}.{}", database, new_table_name),
                Err(e) => eprintln!("Error creating table: {}", e),
            }
        } else {
            println!("No data for date: {}", date_str);
        }

        // 迭代到下一天
        current_date += Duration::days(1);
    }

    println!("Union tables' creation is done for : {} days ", total_days); // 最终进度汇报
}