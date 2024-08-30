use crate::common::order::identification::OrderId;
use crate::common::order::OrderRole;
use crate::{
    common::{
        balance::TokenBalance,
        datafeed::market_event::MarketEvent,
        order::{
            states::{cancelled::Cancelled, open::Open, pending::Pending, request_cancel::RequestCancel},
            Order,
        },
    },
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade,
    AccountEvent, ClientExecution, Exchange, ExecutionError, RequestOpen,
};
use async_trait::async_trait;
use mpsc::UnboundedSender;
use oneshot::Sender;
use tokio::sync::{mpsc, mpsc::UnboundedReceiver, oneshot};
use SandBoxClientEvent::{CancelOrders, CancelOrdersAll, FetchBalances, FetchOrdersOpen, OpenOrders};

#[derive(Debug)]
pub struct SandBoxClient
{
    pub request_tx: UnboundedSender<SandBoxClientEvent>, // NOTE 这是向模拟交易所端发送信号的发射器。注意指令格式是SandBoxClientEvent
    pub market_event_rx : UnboundedReceiver<MarketEvent<MarketTrade>>
}


// NOTE 模拟交易所客户端可向模拟交易所发送的命令
// 定义类型别名以简化复杂的类型
pub type OpenOrderResults = Vec<Result<Order<Open>, ExecutionError>>;
pub type CancelOrderResults = Vec<Result<Order<Cancelled>, ExecutionError>>;
pub type RequestOpenOrders = (Vec<Order<RequestOpen>>, Sender<OpenOrderResults>);
pub type RequestCancelOrders = (Vec<Order<RequestCancel>>, Sender<CancelOrderResults>);

// 模拟交易所客户端可向模拟交易所发送的命令
#[derive(Debug)]
pub enum SandBoxClientEvent
{
    FetchMarketEvent(MarketEvent<MarketTrade>),
    FetchOrdersOpen(Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders(RequestOpenOrders),
    CancelOrders(RequestCancelOrders),
    CancelOrdersAll(Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
}

#[async_trait]
impl ClientExecution for SandBoxClient
{
    const CLIENT_KIND: Exchange = Exchange::SandBox;
    type Config = (UnboundedSender<SandBoxClientEvent>, UnboundedReceiver<MarketEvent<MarketTrade>>);

    async fn init(config: Self::Config, _: UnboundedSender<AccountEvent>) -> Self {
        // 从 config 元组中解构出 request_tx 和 market_event_rx
        let (request_tx, market_event_rx) = config;

        // 使用 request_tx 和 market_event_rx 初始化 SandBoxClient
        Self {
            request_tx,
            market_event_rx,
        }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        self.request_tx
            .send(FetchOrdersOpen(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send FetchOrdersOpen request");
        // 从模拟交易所接收开放订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive FetchOrdersOpen response")
    }

    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        self.request_tx
            .send(FetchBalances(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send FetchBalances request");
        // 从模拟交易所接收账户余额的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive FetchBalances response")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        self.request_tx
            .send(OpenOrders((open_requests, response_tx)))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send OpenOrders request");
        // 从模拟交易所接收开启订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive OpenOrders response")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        self.request_tx
            .send(CancelOrders((cancel_requests, response_tx)))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send CancelOrders request");
        // 从模拟交易所接收取消订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive CancelOrders response")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>
    {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        self.request_tx
            .send(CancelOrdersAll(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send CancelOrdersAll request");
        // 从模拟交易所接收取消所有订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive CancelOrdersAll response")
    }
}
#[cfg(test)]
mod tests
{
    use super::*;
    use tokio::sync::mpsc;


    #[tokio::test]
    async fn test_fetch_orders_open() {
        // 创建通道，用于请求和响应通信
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        let (market_event_tx, market_event_rx) = mpsc::unbounded_channel(); // 添加 market_event_rx 通道

        let client = SandBoxClient {
            request_tx: request_tx.clone(),
            market_event_rx, // 这里添加 market_event_rx
        };

        // 启动一个异步任务来调用客户端的 fetch_orders_open 方法
        let client_task = tokio::spawn(async move {
            // 等待 fetch_orders_open 方法完成，并期待没有错误发生
            let orders = client.fetch_orders_open().await.expect("fetch_orders_open failed");
            // 断言获取到的订单列表是空的
            assert!(orders.is_empty(), "Expected an empty list of orders");
        });

        // 等待客户端发送 FetchOrdersOpen 请求事件
        // 预期客户端会发送一个获取打开订单的请求
        let request_event = request_rx.recv().await.expect("Expected FetchOrdersOpen event");

        // 匹配接收到的事件，确认它是 FetchOrdersOpen 类型
        if let FetchOrdersOpen(tx) = request_event {
            // 使用 oneshot 通道的发送者发送一个模拟的响应
            // 这里模拟返回一个空的订单列表
            let _ = tx.send(Ok(vec![]));
        } else {
            // 如果接收到的事件不是预期的 FetchOrdersOpen 类型，使用 panic 使测试失败
            panic!("Received unexpected event type");
        }

        // 等待客户端任务完成，确保 fetch_orders_open 方法已成功执行
        client_task.await.expect("Client task should complete successfully");
    }
    //
    // #[tokio::test]
    // async fn test_open_orders() {
    //     // 创建通道用于发送和接收请求
    //     let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    //     let (market_event_tx, market_event_rx) = mpsc::unbounded_channel();
    //
    //     // 初始化 SandBoxClient
    //     let client = SandBoxClient {
    //         request_tx: request_tx.clone(),
    //         market_event_rx, // 添加 market_event_rx
    //     };
    //
    //     // 模拟一个订单请求
    //     let open_request = Order {
    //         kind: crate::common::order::order_instructions::OrderInstruction::Limit,
    //         exchange: Exchange::Binance,
    //         instrument: crate::common::instrument::Instrument::new(
    //             "BTC",
    //             "USDT",
    //             crate::common::instrument::kind::InstrumentKind::Perpetual,
    //         ),
    //         client_ts: chrono::Utc::now().timestamp_millis(),
    //         cid: crate::common::order::identification::client_order_id::ClientOrderId(Option::from("OJBK".to_string())),
    //         side: crate::common::Side::Buy,
    //         state: RequestOpen {
    //             reduce_only: false,
    //             price: 50000.0,
    //             size: 1.0,
    //         },
    //     };
    //
    //     // 启动一个异步任务来调用客户端的 open_orders 方法
    //     let client_task = tokio::spawn(async move {
    //         // 调用 open_orders 方法并验证返回的订单信息
    //         let orders = client.open_orders(vec![open_request]).await;
    //         println!("Client received response: {:?}", orders); // 打印客户端接收到的响应
    //         assert_eq!(orders.len(), 1, "Expected one open order");
    //         assert_eq!(orders[0].as_ref().unwrap().state.price, 50000.0);
    //     });
    //
    //     // 处理接收到的 OpenOrders 请求事件
    //     if let Some(SandBoxClientEvent::OpenOrders((orders, tx))) = request_rx.recv().await {
    //         println!("Received OpenOrders event");
    //
    //         // 将订单转换为 Pending 状态，并等待市场事件
    //         let pending_orders = orders
    //             .into_iter()
    //             .map(|order| {
    //                 Order {
    //                     kind: order.kind,
    //                     exchange: order.exchange,
    //                     instrument: order.instrument,
    //                     client_ts: order.client_ts,
    //                     cid: order.cid,
    //                     side: order.side,
    //                     state: Pending {
    //                         reduce_only: order.state.reduce_only,
    //                         price: order.state.price,
    //                         size: order.state.size,
    //                         predicted_ts: chrono::Utc::now().timestamp_millis(),
    //                         request_id: crate::common::order::identification::request_id::RequestId(1241241241),
    //                     },
    //                 }
    //             })
    //             .collect::<Vec<_>>();
    //
    //         // 模拟市场事件来触发订单转换
    //         let market_event = MarketEvent {
    //             exchange_time: chrono::Utc::now().timestamp_millis(),
    //             received_time: chrono::Utc::now().timestamp_millis(),
    //             exchange: Exchange::Binance,
    //             instrument: crate::common::instrument::Instrument::new(
    //                 "BTC",
    //                 "USDT",
    //                 crate::common::instrument::kind::InstrumentKind::Perpetual,
    //             ),
    //             kind: MarketTrade {
    //                 exchange: "Binance".to_string(),
    //                 symbol: "BTCUSDT".to_string(),
    //                 side: "buy".to_string(),
    //                 price: 50000.0,
    //                 timestamp: chrono::Utc::now().timestamp_millis(),
    //                 amount: 1.0,
    //             },
    //         };
    //
    //         // 发送市场事件
    //         market_event_tx.send(market_event).expect("Failed to send market event");
    //
    //         // 将挂起订单转换为打开订单，并发送响应
    //         let open_orders_results = pending_orders
    //             .into_iter()
    //             .map(|pending_order| {
    //                 Ok(Order {
    //                     kind: pending_order.kind,
    //                     exchange: pending_order.exchange,
    //                     instrument: pending_order.instrument,
    //                     client_ts: pending_order.client_ts,
    //                     cid: pending_order.cid,
    //                     side: pending_order.side,
    //                     state: Open {
    //                         id: OrderId::new(0, 0, 0),
    //                         price: pending_order.state.price,
    //                         size: pending_order.state.size,
    //                         filled_quantity: 0.0,
    //                         order_role: OrderRole::Maker,
    //                         received_ts: chrono::Utc::now().timestamp_millis(),
    //                     },
    //                 })
    //             })
    //             .collect::<Vec<_>>();
    //
    //         println!("Response being sent: {:?}", open_orders_results); // 打印将要发送的响应
    //
    //         // 发送响应，并确认是否成功发送
    //         if tx.send(open_orders_results).is_ok() {
    //             println!("Response sent successfully");
    //         } else {
    //             println!("Failed to send OpenOrders response");
    //         }
    //     } else {
    //         panic!("Did not receive OpenOrders event");
    //     }
    //
    //     // 确保客户端任务完成
    //     client_task.await.expect("Client task should complete successfully");
    // }
    //

    #[tokio::test]
    async fn test_cancel_orders_all()
    {
        // 创建一个模拟的 SandBoxClientEvent 发射器和接收器
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        let (market_event_tx, market_event_rx) = mpsc::unbounded_channel();
        let (_response_tx, _response_rx) = oneshot::channel::<Result<Vec<Order<Cancelled>>, ExecutionError>>();

        // 初始化 SandBoxClient
        let client = SandBoxClient {
            request_tx: request_tx.clone(),
            market_event_rx, // 添加 market_event_rx
        };

        // 启动一个异步任务来调用客户端的 cancel_orders_all 方法
        let client_task = tokio::spawn(async move {
            let result = client.cancel_orders_all().await;
            println!("Client received response: {:?}", result); // 打印客户端接收到的响应
            assert!(result.is_ok(), "Expected a successful result");
            assert!(result.unwrap().is_empty(), "Expected an empty list of cancelled orders");
        });

        // 模拟从 SandBoxClientEvent 接收器获取 CancelOrdersAll 事件
        if let Some(SandBoxClientEvent::CancelOrdersAll(tx)) = request_rx.recv().await {
            println!("Received CancelOrdersAll event");

            // 发送一个空的取消订单列表作为响应
            let response = Ok(vec![]);
            println!("Response being sent: {:?}", response); // 打印将要发送的响应

            // 发送响应，并确认是否成功发送
            if tx.send(response).is_ok() {
                println!("Response sent successfully");
            } else {
                println!("Failed to send CancelOrdersAll response");
            }
        } else {
            panic!("Did not receive CancelOrdersAll event");
        }

        // 确保客户端任务完成
        client_task.await.expect("Client task should complete successfully");
    }
}
