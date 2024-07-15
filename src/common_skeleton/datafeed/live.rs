use tokio::sync::mpsc;

#[derive(Debug)]
pub struct LiveFeed<Event>
{
    pub market_rx: mpsc::UnboundedReceiver<Event>,
}

