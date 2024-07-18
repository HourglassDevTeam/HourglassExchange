use std::{pin::Pin, sync::Arc};

use futures::Stream;

use crate::simulated_exchange::load_from_clickhouse::queries_operations::ClickHouseClient;

pub struct HistoricalFeed<Event>
{
    // NOTE ClickHouseClient is opted and hardcoded for simplicity, as a makeshift solution.
    pub database_client: Arc<ClickHouseClient>,
    pub stream: Pin<Box<dyn Stream<Item = Event> + Send>>,
}

impl<Event> HistoricalFeed<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Event> + Send)>
    {
        println!("Polled historical feed");
        self.stream.as_mut()
    }
}
