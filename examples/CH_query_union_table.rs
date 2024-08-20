use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
// use unilink_execution::sandbox::clickhouse_api::query_builder::ClickHouseQueryBuilder;

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_05_05";

    // EXAMPLE 1 MANUALLY BUILD THE QUERY
    // let query_builder = ClickHouseQueryBuilder::new();
    // let database_name = client.construct_database_name(exchange,instrument,channel);
    // let union_table_name = client.construct_union_table_name(exchange,instrument,channel,date);
    // let query = query_builder.select("*").from(&database_name, &union_table_name).build();
    // let trades = client.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await;
    // println!("{:?}",  trades)

    // EXAMPLE 2 USE PREDEFINED METHOD
    let trades = client.query_unioned_trade_table(exchange, instrument, channel, date).await;
    println!("{:?}", trades)
}
