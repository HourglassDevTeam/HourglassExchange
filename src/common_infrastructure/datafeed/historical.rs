use std::{fmt::Debug, sync::Arc};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::{common_infrastructure::datafeed::public_event::PublicEvent, sandbox::clickhouse_api::queries_operations::ClickHouseClient};

pub struct HistoricalFeed
{
    // NOTE ClickHouseClient is opted and hardcoded for simplicity, as a makeshift solution.
    pub database_client: Arc<ClickHouseClient>,
    pub receiver: UnboundedReceiver<PublicEvent>,
}

impl HistoricalFeed
{
    pub async fn recv_next(&mut self) -> Option<PublicEvent>
    {
        self.receiver.recv().await
    }
}
