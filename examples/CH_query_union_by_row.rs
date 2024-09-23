use hourglass::hourglass::clickhouse_api::queries_operations::ClickHouseClient;
use std::time::Instant;

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_05_05";

    // EXAMPLE 1 MANUALLY BUILD THE QUERY
    // let database_name = client.construct_database_name(exchange, instrument, "trades");
    // let table_name = client.construct_union_table_name(exchange, instrument, "trades", date);
    // let query = ClickHouseQueryBuilder::new()
    //     .select("symbol, side, price, timestamp, amount")
    //     .from( &database_name, &table_name)
    //     .order("timestamp", Some("DESC"))
    //     .build();
    // println!("Constructed query {}", query);
    //
    // let client_ref = client.client.read().await;
    // let mut cursor = client_ref.query(&query).fetch::<ClickhousePublicTrade>().unwrap();

    // EXAMPLE 2 USE PREDEFINED METHOD
    let mut cursor = client.cursor_unioned_public_trades_for_test(exchange, instrument, date).await.unwrap();
    let start_time = Instant::now();
    while let Ok(Some(row)) = cursor.next().await {
        println!("{:?}", row)
    }
    let duration = start_time.elapsed();
    println!("ClickhousePublicTrade data fetched in: {:?}", duration);
}
