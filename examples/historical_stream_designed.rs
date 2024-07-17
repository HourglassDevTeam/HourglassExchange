use std::sync::Arc;
use lazy_static::lazy_static;
use unilink_execution::{common_skeleton::datafeed::historical::HistoricalFeed, simulated_exchange::load_from_clickhouse::queries_operations::*};


lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}


#[tokio::main]
async fn main()
{
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_03_03";

    let stream = CLIENT.query_union_table_batched(exchange, instrument, channel, date);
    HistoricalFeed { database_client: CLIENT.to_owned(), stream: Box::pin(stream) };
}
