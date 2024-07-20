use std::fmt::Debug;

use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;

use crate::common_skeleton::datafeed::event::MarketEvent;

/// Live feed for events.
pub struct LiveFeed<Event>
{
    pub(crate) receiver: UnboundedReceiver<MarketEvent<Event>>,
}

impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static
{
    pub fn recv_next(&mut self) -> Option<MarketEvent<Event>>
    {
        // 尝试从接收器中接收事件
        self.receiver.try_recv().ok()
    }
}
