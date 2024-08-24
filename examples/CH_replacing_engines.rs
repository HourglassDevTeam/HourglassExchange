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
    let start_date = NaiveDate::from_ymd_opt(2024, 5, 5).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 5, 5).expect("Invalid end date");

    // 在循环外部获取所有表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    // 筛选出所有 `union` 表
    let union_tables: Vec<String> = table_names.into_iter()
        .filter(|table_name| table_name.contains("union"))
        .collect();

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

    println!("[UniLinkExecution] : All union tables have been converted to ReplacingMergeTree with the specified sort key.");
}
