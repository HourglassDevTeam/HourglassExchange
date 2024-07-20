use std::fmt::Debug;

use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use crate::common_skeleton::datafeed::event::MarketEvent;

/// Live feed for events.
#[allow(dead_code)]
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


impl<Event> LiveFeed<Event> {
    /// Creates a new `LiveFeed`.
    pub fn new(sender: UnboundedSender<MarketEvent<Event>>) -> Self {
        LiveFeed {
            receiver: mpsc::unbounded_channel().1,
        }
    }
}