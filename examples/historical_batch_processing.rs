// build an example to demonstrate querying

use unilink_execution::simulated_exchange::load_from_clickhouse::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();
    let exchange = "okex";
    let instrument = "swap";
    let channel = "trades";
    let date = "2024_03_03";

    // 查询 binance_futures_trades_union_2024_03_28 表
    let result = ClickHouseClient::query_union_table(&client, exchange, instrument, channel, date).await;
    match result {
        Ok(records) => println!("Query result: {:?}", records),
        Err(e) => eprintln!("Error querying table: {}", e),
    }
}
