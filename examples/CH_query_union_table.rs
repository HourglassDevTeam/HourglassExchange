use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_05_05";
    let trades = client.query_unioned_trade_table(exchange, instrument, channel, date).await;
    println!("{:?}", trades)
}
