use chrono::{Duration, NaiveDate};
use rayon::prelude::*;
use std::sync::Arc;
use unilink_execution::sandbox::{clickhouse_api::queries_operations::ClickHouseClient, utils::chrono_operations::extract_date};

#[tokio::main]
async fn main()
{
    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";

    // 在循环外部获取所有表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let start_date = NaiveDate::from_ymd_opt(2019, 11, 19).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 9).expect("Invalid end date");

    // 获取所有表名
    let table_names = client.get_table_names(&database).await;

    // 遍历每一天，并生成目标表名
    let mut current_date = start_date;
    while current_date <= end_date {
        // 生成当天的目标表名
        let target_table_name = format!("{}_{}_trades_union_{}", exchange, instrument, current_date.format("%Y_%m_%d"));

        // 过滤符合当前日期的 additional_table
        let filtered_table_names: Vec<String> = table_names.par_iter()
                                                           .filter(|table_name| {
                                                               if let Some(date_str) = extract_date(table_name) {
                                                                   if let Ok(table_date) = NaiveDate::parse_from_str(&date_str, "%Y_%m_%d") {
                                                                       return table_date == current_date && !table_name.contains("union");
                                                                   }
                                                               }
                                                               false
                                                           })
                                                           .cloned()
                                                           .collect();

        // 如果有符合的表，调用 insert_into_unioned_table 方法
        if !filtered_table_names.is_empty() {
            println!("\n[NOTE] : The filtered_table_names to be added into the target table are:");

            for table_name in &filtered_table_names {
                println!("- {}", table_name);
            }

            println!("\n[NOTE] : End of the list.\n");
            client.insert_into_unioned_table(&database, &target_table_name, &filtered_table_names, true).await.unwrap();
            current_date += Duration::days(1);
        }
        else {
            println!("[UniLinkExecution] : No tables found for the date: {}", current_date);
            current_date += Duration::days(1);
        }
    }
}
