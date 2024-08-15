use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::clickhouse_api::query_builder::ClickHouseQueryBuilder;


#[tokio::main]
async fn main() {
    let query_builder = ClickHouseQueryBuilder::new();
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_05_05";
    let database_name = client.construct_database_name(exchange,instrument,channel);
    let union_table_name = client.construct_union_table_name(exchange,instrument,channel,date);
    let query = query_builder.select("*").from(&database_name, &union_table_name).build();
    let trade_datas = client.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await;
    println!("{:?}",  trade_datas)
}