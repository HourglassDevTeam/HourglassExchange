use chrono::{NaiveDate, Duration};
use std::{sync::Arc};
use tokio::sync::RwLock;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2024, 8, 9).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 9).expect("Invalid end date");

    // 在循环外部获取所有表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    // 遍历日期范围
    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();

        // 筛选出当日的 `union` 表
        let union_tables: Vec<String> = table_names.iter()
            .filter(|table_name| {
                if table_name.contains("union") {
                    if let Some(table_date) = unilink_execution::sandbox::utils::chrono_operations::extract_date(table_name) {
                        if let Ok(parsed_date) = NaiveDate::parse_from_str(&table_date, "%Y_%m_%d") {
                            return parsed_date == current_date;
                        }
                    }
                }
                false
            })
            .cloned()
            .collect();

        if union_tables.is_empty() {
            // 没有找到当日的 union 表，发送通知
            println!("[UniLinkExecution] : No union tables found for date: {}", date_str);
        } else {
            for table_name in union_tables {
                let new_table_name = format!("{}_replacing", table_name);
                let database = database.clone();
                let client = Arc::clone(&client);

                let alter_query = format!(
                    "CREATE TABLE {}.{} ENGINE = ReplacingMergeTree() ORDER BY (timestamp, side, symbol) AS SELECT * FROM {}.{}",
                    database, new_table_name, database, table_name
                );

                let drop_old_table_query = format!("DROP TABLE {}.{}", database, table_name);

                let rename_query = format!("RENAME TABLE {}.{} TO {}.{}", database, new_table_name, database, table_name);

                match client.client.read().await.query(&alter_query).execute().await {
                    Ok(_) => {
                        println!("[UniLinkExecution] : Successfully created new table with ReplacingMergeTree: {}.{}", database, new_table_name);

                        // 删除旧表
                        match client.client.read().await.query(&drop_old_table_query).execute().await {
                            Ok(_) => {
                                println!("[UniLinkExecution] : Successfully dropped old table: {}.{}", database, table_name);

                                // 重命名新表为原来的表名
                                match client.client.read().await.query(&rename_query).execute().await {
                                    Ok(_) => println!("[UniLinkExecution] : Successfully renamed table: {}.{} to {}.{}", database, new_table_name, database, table_name),
                                    Err(e) => eprintln!("[UniLinkExecution] : Error renaming table: {}", e),
                                }
                            }
                            Err(e) => eprintln!("[UniLinkExecution] : Error dropping old table: {}", e),
                        }
                    }
                    Err(e) => eprintln!("[UniLinkExecution] : Error creating new table: {}", e),
                };
            }
        }

        // 迭代到下一天
        current_date += Duration::days(1);
    }

    println!("[UniLinkExecution] : All union tables within the date range have been converted to ReplacingMergeTree with the specified sort key.");
}
