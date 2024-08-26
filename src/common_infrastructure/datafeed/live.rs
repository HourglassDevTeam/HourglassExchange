// // NOTE 先创建Workspace再启用这个模块。
// use std::fmt::Debug;
//
// use mpsc::UnboundedReceiver;
// use tokio::sync::mpsc;
// use cerebro_data::event::MarketEvent;
//
// /// Live feed for events.
// #[allow(dead_code)]
// pub struct LiveFeed
// {
//     pub(crate) receiver: UnboundedReceiver<MarketEvent>,
// }
//
// impl LiveFeed
// {
//     pub async fn recv(&mut self) -> Option<MarketEvent>
//     {
//         // 尝试从接收器中接收事件
//         self.receiver.recv().await
//     }
// }
//
//
// impl LiveFeed {
//     pub fn new(receiver: UnboundedReceiver<MarketEvent>) -> Self {
//         LiveFeed {
//             receiver,
//         }
//     }
// }
