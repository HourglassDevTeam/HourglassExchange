// use std::{collections::HashMap, fmt, fmt::Debug};
//
// use mpsc::UnboundedReceiver;
// use tokio::sync::mpsc;
//
// // 定义一个数据流别名，用于标识每个数据流。
// pub type StreamID = String;
//
// // 定义一个结构体，用于管理多个数据流。
// pub struct AccountDataStreams
//     {
//     pub streams: HashMap<StreamID, UnboundedReceiver<DataKind>>, // 使用HashMap存储数据流，键为StreamID
// }
//
// impl Default for AccountDataStreams {
//     fn default() -> Self
//     {
//         Self::new()
//     }
// }
//
// // 为 AccountDataStreams 实现创建和增减数据流的方法，用于管理数据流。
// impl AccountDataStreams {
//     // 创建一个新的 AccountDataStreams 实例。
//     pub fn new() -> Self
//     {
//         Self { streams: HashMap::new() }
//     }
//
//     // 向AccountDataStreams中添加一个新的数据流。
//     pub fn add_stream(&mut self, identification: StreamID, receiver: UnboundedReceiver)
//     {
//         self.streams.insert(identification, receiver);
//     }
//
//     // 从AccountDataStreams中移除一个数据流。
//     pub fn remove_stream(&mut self, identification: StreamID)
//     {
//         self.streams.remove(&identification);
//     }
//
//     // 将所有数据流合并到一个新的接收器中，并按时间戳排序。
//     pub async fn join(self) -> UnboundedReceiver
//     {
//         let mut joined_rx = self.merge_streams().await;
//
//         // 监听合并后的接收器，收集所有事件后再排序并发送到新的接收器中
//         let (sorted_tx, sorted_rx) = mpsc::unbounded_channel();
//         tokio::spawn(async move {
//             let mut buffer = Vec::new();
//
//             // 将所有事件收集到buffer中
//             while let Some(event) = joined_rx.recv().await {
//                 buffer.push(event);
//             }
//
//             // 对buffer中的事件进行排序
//             buffer.sort();
//
//             // 将排序后的事件发送到新的接收器中
//             for e in buffer {
//                 let _ = sorted_tx.send(e);
//             }
//         });
//
//         sorted_rx
//     }
//
//     // 将所有数据流合并到一个新的接收器中，不进行排序。
//     pub async fn join_without_sort(self) -> UnboundedReceiver
//     {
//         self.merge_streams().await
//     }
//
//     // 合并所有数据流到一个新的接收器中。
//     async fn merge_streams(self) -> UnboundedReceiver
//
//     {
//         let (joined_tx, joined_rx) = mpsc::unbounded_channel();
//
//         for mut exchange_rx in self.streams.into_values() {
//             let joined_tx = joined_tx.clone();
//
//             tokio::spawn(async move {
//                 while let Some(event) = exchange_rx.recv().await {
//                     let _ = joined_tx.send(event);
//                 }
//             });
//         }
//
//         joined_rx
//     }
// }
//
// // 为 AccountDataStreams 实现 Debug trait，方便调试。
// impl Debug for AccountDataStreams {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
//     {
//         // 打印 AccountDataStreams 的调试信息，包括流的标识符。
//         let stream_keys: Vec<_> = self.streams.keys().collect();
//         f.debug_struct("AccountDataStreams").field("streams", &stream_keys).finish()
//     }
// }
//
// #[tokio::test]
// async fn add_stream_should_add_new_stream()
// {
//     let mut streams: AccountDataStreams<i32> = AccountDataStreams::new(); // 明确指定 Event 类型为 i32
//     let (_tx, rx) = mpsc::unbounded_channel();
//     streams.add_stream("stream1".to_string(), rx);
//     assert!(streams.streams.contains_key("stream1"));
// }
//
// #[tokio::test]
// async fn remove_stream_should_remove_existing_stream()
// {
//     let mut streams: AccountDataStreams<i32> = AccountDataStreams::new(); // 明确指定 Event 类型为 i32
//     let (_tx, rx) = mpsc::unbounded_channel();
//     streams.add_stream("stream1".to_string(), rx);
//     streams.remove_stream("stream1".to_string());
//     assert!(!streams.streams.contains_key("stream1"));
// }
//
// #[tokio::test]
// async fn join_should_merge_and_sort_streams()
// {
//     let mut streams: AccountDataStreams<i32> = AccountDataStreams::new(); // 明确指定 Event 类型为 i32
//     let (tx1, rx1) = mpsc::unbounded_channel();
//     let (tx2, rx2) = mpsc::unbounded_channel();
//     streams.add_stream("stream1".to_string(), rx1);
//     streams.add_stream("stream2".to_string(), rx2);
//
//     tokio::spawn(async move {
//         tx1.send(3).unwrap();
//         tx2.send(1).unwrap();
//         tx1.send(2).unwrap();
//     });
//
//     let mut joined_rx = streams.join().await;
//     assert_eq!(joined_rx.recv().await, Some(1));
//     assert_eq!(joined_rx.recv().await, Some(2));
//     assert_eq!(joined_rx.recv().await, Some(3));
// }
//
// #[tokio::test]
// async fn join_without_sort_should_merge_streams()
// {
//     let mut streams: AccountDataStreams<i32> = AccountDataStreams::new(); // 明确指定 Event 类型为 i32
//     let (tx1, rx1) = mpsc::unbounded_channel();
//     let (tx2, rx2) = mpsc::unbounded_channel();
//     streams.add_stream("stream1".to_string(), rx1);
//     streams.add_stream("stream2".to_string(), rx2);
//
//     tokio::spawn(async move {
//         tx1.send(3).unwrap();
//         tx2.send(1).unwrap();
//         tx1.send(2).unwrap();
//     });
//
//     let mut joined_rx = streams.join_without_sort().await;
//     let mut received = vec![];
//     while let Some(event) = joined_rx.recv().await {
//         received.push(event);
//     }
//     received.sort();
//     assert_eq!(received, vec![1, 2, 3]);
// }
