use tokio::sync::mpsc;
use crate::common_skeleton::datafeed::Feed;

#[derive(Debug)]
pub struct LiveFeed<Event>
{   // 此处不设发送端，发送端由subscriber实现
    pub market_rx: mpsc::UnboundedReceiver<Event>,
}


impl<Event> MarketFeedDistributor<Event> for LiveFeed<Event>
{
    /// 实现 MarketGenerator trait，用于生成下一个市场 `Event`。

    fn fetch_next(&mut self) -> Feed<Event>
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
