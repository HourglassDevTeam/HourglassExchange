use std::pin::Pin;
use std::sync::Arc;

use futures::{stream, Stream, StreamExt};
use crate::error::ExecutionError;
use crate::simulated_exchange::load_from_clickhouse::queries_operations::ClickHouseClient;

pub struct HistoricalFeed<Event>
{
    // NOTE ClickHouseClient is opted and hardcoded for simplicity, as a makeshift solution.
    pub database_client: Arc<ClickHouseClient>,
    pub stream: Pin<Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>>,
}

impl<Event> HistoricalFeed<Event> where Event: Clone + Send + Sync + 'static
{

    pub fn new(client: Arc<ClickHouseClient>, exchange: &str, instrument: &str, channel: &str, date: &str) -> Self {
        let initial_stream = client.query_union_table_batched(exchange, instrument, channel, date);
        // Transform Vec<WsTrade> into a stream of WsTrade
        let flattened_stream = initial_stream.flat_map(|result_vec| {
            match result_vec {
                Ok(vec) => stream::iter(vec.into_iter().map(Ok)).boxed(),
                Err(e) => stream::once(async move { Err(e) }).boxed(),
            }
        });

        HistoricalFeed {
            database_client: client,
            stream: Box::pin(flattened_stream),
        }
    }
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Result<Event, ExecutionError>> + Send)>
    {
        self.stream.as_mut()
    }
}
