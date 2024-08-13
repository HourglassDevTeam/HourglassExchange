use chrono::{Duration, NaiveDate};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::utils::chrono_operations::extract_date;

#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();
    // fields for binance
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2024, 3, 3).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 7, 3).expect("Invalid end date");
    let mut current_date = start_date;

    // 在循环外部获取所有的表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();
        println!("Database: {}", database); // 打印 database 变量的调试信息

        // 筛选出指定日期的表名
        let tables: Vec<String> = table_names
            .iter()
            .filter(|table_name| {
                if let Some(table_date) = extract_date(table_name) {
                    table_date == date_str
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        println!("Processing tables on Date: {}", date_str); // 打印日期和表名的调试信息

        if !tables.is_empty() {
            let new_table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date_str);
            match client.create_unioned_tables_for_date(&database, &new_table_name, &tables).await {
                Ok(_) => println!("Successfully created table: {}.{}", database, new_table_name),
                Err(e) => eprintln!("Error creating table: {}", e),
            }
        } else {
            println!("No data for date: {}", date_str);
        }

        // iterating to the next day
        current_date += Duration::days(1);
    }
}
