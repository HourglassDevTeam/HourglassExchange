use chrono::{Duration, NaiveDate};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::collections::HashSet;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main()
{
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let mut start_date = NaiveDate::from_ymd_opt(2019, 11, 20).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 9).expect("Invalid end date"); // 设置结束日期
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let all_table_names: HashSet<_> = client.get_union_table_names(&database).await.into_par_iter().collect();

    // 初始化一个 HashSet 来存储所有日期的字符串
    let mut all_dates: HashSet<String> = HashSet::new();

    // 初始化一个 HashSet 来存储没有表的日期
    let mut dates_without_tables: HashSet<String> = HashSet::new();

    // 填充 all_dates HashSet
    while start_date <= end_date {
        let date_str = start_date.format("%Y_%m_%d").to_string();
        all_dates.insert(date_str.clone());
        start_date += Duration::days(1);
    }

    // 筛选符合日期范围的表名
    let mut valid_table_names = HashSet::new();
    for date in &all_dates {
        let matching_tables: Vec<_> = all_table_names.par_iter().filter(|table_name| table_name.contains(date)).cloned().collect();
        if matching_tables.is_empty() {
            dates_without_tables.insert(date.clone());
        }
        valid_table_names.extend(matching_tables);
    }

    println!("Dates without tables:");
    for date in dates_without_tables {
        println!("{}", date);
    }
}
