use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();

    // 设置参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";

    // 将上述变量拼接成数据库名
    let database = client.construct_database_name(exchange, instrument, channel);

    // 获取所有表名
    let all_tables =  client.get_table_names(&database).await;

    // 筛选出不包含 "union" 字样的表名
    let non_union_tables: Vec<String> = all_tables
        .into_iter()
        .filter(|table_name| !table_name.contains("union"))
        .collect();

    // 获取总non-union表的数量
    let total_tables = non_union_tables.len();
    if total_tables == 0 {
        println!("No non-union tables found to delete.");
        return;
    }

    // 删除不包含 "union" 字样的表，并汇报进度
    for (index, table) in non_union_tables.iter().enumerate() {
        let drop_query = format!("DROP TABLE {}.{}", database, table);
        println!("[ClickHouse] : Executing query: {}", drop_query);
        println!("[UniLinkExecution] : Executing query: {}", drop_query);

        match client.client.read().await.query(&drop_query).execute().await {
            Ok(_) => println!("Successfully dropped table: {}.{}", database, table),
            Err(e) => eprintln!("Error dropping table: {}.{}", table, e),
        }

        // 计算并打印进度
        let progress = ((index + 1) as f64 / total_tables as f64) * 100.0;
        println!("Progress: {:.2}% ({} of {})", progress, index + 1, total_tables);
    }

    println!("Deletion of non-union tables is complete.");
}
