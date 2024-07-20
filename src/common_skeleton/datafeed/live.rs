// NOTE 先创建Workspace再启用这个模块。
// use std::fmt::Debug;
//
// use mpsc::UnboundedReceiver;
// use tokio::sync::mpsc;
// use cerebro_data::event::MarketEvent;
//
// /// Live feed for events.
// #[allow(dead_code)]
// pub struct LiveFeed<Event>
// {
//     pub(crate) receiver: UnboundedReceiver<MarketEvent<Event>>,
// }
//
// impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static
// {
//     pub async fn recv(&mut self) -> Option<MarketEvent<Event>>
//     {
//         // 尝试从接收器中接收事件
//         self.receiver.recv().await
//     }
// }
//
//
// impl<Event> LiveFeed<Event> {
//     pub fn new(receiver: UnboundedReceiver<MarketEvent<Event>>) -> Self {
//         LiveFeed {
//             receiver,
//         }
//     }
// }
