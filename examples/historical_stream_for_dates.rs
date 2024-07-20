use std::sync::Arc;

use futures::StreamExt;
use lazy_static::lazy_static;

use unilink_execution::{
    common_skeleton::datafeed::historical::HistoricalFeed,
    simulated_exchange::{account::account_market_feed::*, load_from_clickhouse::queries_operations::*},
};

lazy_static! {
    pub static ref CLIENT: Arc<ClickHouseClient> = Arc::new(ClickHouseClient::new());
}

#[tokio::main]
async fn main()
{
    let mut account_streams = AccountDataStreams::new();

    // Define the parameters for the streams
    let stream_params = vec![("binance", "futures", "trades", "2024_03_03", "2024_07_03"),
                             /* Add more parameters if needed */];

    for (exchange, instrument, channel, start_date, end_date) in stream_params {
        let stream = CLIENT.query_unioned_trade_table_batched_for_dates(exchange, instrument, channel, start_date, end_date, 1000000);

        let feed = HistoricalFeed { database_client: CLIENT.clone(),
                                    receiver: Box::pin(stream) };

        let stream_id = format!("{}_{}_{}", exchange, instrument, channel);
        account_streams.add_stream(stream_id, DataStream::Historical(feed));
    }

    loop {
        for (id, stream) in account_streams.streams.iter_mut() {
            let result = stream.next().await;
            match result {
                | Some(data) => {
                    println!("Stream {}: {:?}", id, data);
                }
                | None => continue,
            }
        }
    }
}
