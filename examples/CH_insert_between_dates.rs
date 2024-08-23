use chrono::NaiveDate;
use rayon::prelude::*;
use std::sync::Arc;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::utils::chrono_operations::extract_date;

#[tokio::main]
async fn main() {
    // 检测是否为 release 模式
    #[cfg(debug_assertions)]
    println!("[UniLinkExecution] : Running in debug mode");

    #[cfg(not(debug_assertions))]
    println!("[UniLinkExecution] : Running in release mode");

    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 定义参数
    let database = "my_database";
    let target_table_name = "target_table";
    let start_date = NaiveDate::from_ymd_opt(2019, 11, 19).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 10).expect("Invalid end date");

    // 获取所有表名
    let table_names = client.get_table_names(database).await;

    // 过滤符合日期范围的表
    let filtered_table_names: Vec<String> = table_names.par_iter()
        .filter(|table_name| {
            if let Some(date_str) = extract_date(table_name) {
                if let Ok(table_date) = NaiveDate::parse_from_str(&date_str, "%Y_%m_%d") {
                    return table_date >= start_date && table_date <= end_date;
                }
            }
            false
        })
        .cloned()
        .collect();

    // 调用 insert_into_unioned_table 方法
    match client.insert_into_unioned_table(database, target_table_name, &filtered_table_names, true).await {
        Ok(_) => println!("[UniLinkExecution] : Data inserted successfully."),
        Err(e) => eprintln!("[UniLinkExecution] : Error inserting data: {}", e),
    }
}