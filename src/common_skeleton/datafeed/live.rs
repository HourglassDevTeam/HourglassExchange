use tokio::sync::mpsc;
use crate::common_skeleton::datafeed::Feed;

#[derive(Debug)]
pub struct LiveFeed<Event>
{
    pub market_rx: mpsc::UnboundedReceiver<Event>,
}


impl<Event> LiveFeedGenerator<Event> for LiveFeed<Event>
{
    /// 实现 MarketGenerator trait，用于生成下一个市场 `Event`。

    fn next(&mut self) -> Feed<Event>
    {
        loop {
            match self.market_rx.try_recv() {
                | Ok(event) => break Feed::Next(event),
                | Err(mpsc::error::TryRecvError::Empty) => continue,
                | Err(mpsc::error::TryRecvError::Disconnected) => break Feed::Finished,
            }
        }
    }
}
