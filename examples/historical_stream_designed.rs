use unilink_execution::common_skeleton::datafeed::historical::HistoricalFeed;
use unilink_execution::simulated_exchange::load_from_clickhouse::queries_operations::ClickHouseClient;
use unilink_execution::simulated_exchange::ws_trade::WsTrade;

// Usage of `query_union_table_batched` to create a `HistoricalFeed`
#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_03_03";

    let stream = client.query_union_table_batched(exchange, instrument, channel, date);
    let feed: HistoricalFeed<WsTrade> = HistoricalFeed::new(client, "binance", "futures", "trades", "2024_03_03");

}
