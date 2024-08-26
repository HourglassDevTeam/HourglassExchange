// /// NOTE currently under construction
// use serde::{Deserialize, Serialize};
//
// pub mod public_event;
// pub mod historical;
// pub mod live;
//
// pub trait MarketFeedDistributor
// {
//     // NOTE 历史市场事件迭代器和直接市场事件迭代器都会实现 MarketFeedDistributor，
//     //      但是方式不同。所以此处不定义细节。
//     fn fetch_next(&mut self) -> FeedStatus;
// }
//
// // #[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
// // pub enum FeedStatus
// // {
// //     Next(Event),
// //     Unhealthy, // Unhealthy 变体，表示市场数据流状态不健康
// //     Finished,  // Finished 变体，表示市场数据流已经结束
// // }

pub mod public_event;