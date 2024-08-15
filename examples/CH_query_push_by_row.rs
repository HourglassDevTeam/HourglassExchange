use std::time::Instant;
use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::clickhouse_api::query_builder::ClickHouseQueryBuilder;

#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_05_05";
    let base = "1000RATS";
    let quote = "USDT";

    let database_name = client.construct_database_name(exchange, instrument, "trades");
    let table_name = client.construct_table_name(exchange, instrument, "trades", date, base, quote);
    let query = ClickHouseQueryBuilder::new()
        .select("symbol, side, price, timestamp, amount")
        .from( &database_name, &table_name)
        .order("timestamp", Some("DESC"))
        .build();
    println!("[UniLinkExecution] : Constructed query {}", query);

    let client_ref = client.client.read().await;
    let mut cursor = client_ref.query(&query).fetch::<ClickhousePublicTrade>().unwrap();
    let start_time = Instant::now();
    while let Ok(Some(row)) = cursor.next().await {
        println!("{:?}", row)
    }
    let duration = start_time.elapsed();
    println!("ClickhousePublicTrade data fetched in: {:?}", duration);
}