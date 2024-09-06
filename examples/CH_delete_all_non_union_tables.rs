use rayon::{iter::IntoParallelRefIterator, prelude::ParallelIterator};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();

    // 设置参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";

    // 将上述变量拼接成数据库名
    let database = client.construct_database_name(exchange, instrument, channel);

    // 获取所有表名
    let mut all_tables = client.get_table_names(&database).await;

    // 获取总non-union表的数量，用于进度汇报
    let total_tables = all_tables.par_iter().filter(|table_name| !table_name.contains("union")).count();

    if total_tables == 0 {
        println!("[UniLinkEx] : No non-union tables found to delete.");
        return;
    }

    let mut processed_tables = 0;

    // 遍历所有表，删除不包含 "union" 字样的表，并汇报进度
    while let Some(table_name) = all_tables.par_iter().find_any(|table_name| !table_name.contains("union")).cloned() {
        let drop_query = format!("DROP TABLE {}.{}", database, table_name);
        println!("[UniLinkEx] : Executing query: {}", drop_query);
        println!("[UniLinkEx] : Executing query: {}", drop_query);

        match client.client.read().await.query(&drop_query).execute().await {
            | Ok(_) => {
                println!("[UniLinkEx] : Successfully dropped table: {}.{}", database, table_name);
                all_tables.retain(|name| name != &table_name); // 删除已经处理的表名
                processed_tables += 1;
            }
            | Err(e) => eprintln!("[UniLinkEx] : Error dropping table: {}.{}", table_name, e),
        }

        // 计算并打印进度
        let progress = (processed_tables as f64 / total_tables as f64) * 100.0;
        println!("[UniLinkEx] : Progress: {:.2}% ({} of {})", progress, processed_tables, total_tables);
    }

    println!("[UniLinkEx] : Deletion of non-union tables is complete.");
}
