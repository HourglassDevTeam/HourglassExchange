use async_trait::async_trait;
use mpsc::UnboundedSender;
use oneshot::Sender;
use tokio::sync::{mpsc, mpsc::UnboundedReceiver, oneshot};
use SandBoxClientEvent::{CancelOrders, CancelOrdersAll, FetchBalances, FetchOrdersOpen, OpenOrders};
use crate::{
    common_infrastructure::{
        balance::TokenBalance,
        datafeed::event::MarketEvent,
        order::{Cancelled, Open, Order, Pending},
    },
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade,
    AccountEvent, ClientExecution, ExchangeVariant, ExecutionError, RequestCancel, RequestOpen,
};

#[derive(Debug)]
pub struct SandBoxClient
{
    pub local_timestamp: i64,
    pub request_tx: UnboundedSender<SandBoxClientEvent>, // NOTE 这是向模拟交易所端发送信号的发射器。注意指令格式是SandBoxClientEvent
    pub strategy_signal_rx: UnboundedReceiver<SandBoxClientEvent>, // NOTE 这是从策略收取信号的接收器。注意指令格式是SandBoxClientEvent
}

// NOTE 模拟交易所客户端可向模拟交易所发送的命令
// 定义类型别名以简化复杂的类型
type OpenOrderResults = Vec<Result<Order<Pending>, ExecutionError>>;
type CancelOrderResults = Vec<Result<Order<Cancelled>, ExecutionError>>;
type RequestOpenOrders = (Vec<Order<RequestOpen>>, Sender<OpenOrderResults>);
type RequestCancelOrders = (Vec<Order<RequestCancel>>, Sender<CancelOrderResults>);

// 模拟交易所客户端可向模拟交易所发送的命令
#[derive(Debug)]
pub enum SandBoxClientEvent
{
    FetchMarketEvent(MarketEvent<ClickhousePublicTrade>),
    FetchOrdersOpen(Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders(RequestOpenOrders),
    CancelOrders(RequestCancelOrders),
    CancelOrdersAll(Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
}

#[async_trait]
impl ClientExecution for SandBoxClient
{
    // 注意：客户端的类型自然地由交易所决定并与其保持一致。
    const CLIENT_KIND: ExchangeVariant = ExchangeVariant::SandBox;

    // 注意：在我们的场景中，沙盒交易所的“可选”配置参数是一个 UnboundedSender。
    type Config = (UnboundedSender<SandBoxClientEvent>, UnboundedReceiver<SandBoxClientEvent>);

    async fn init(config: Self::Config, _: UnboundedSender<AccountEvent>, local_timestamp: i64) -> Self
    {
        // 从 config 元组中解构出 request_tx 和 request_rx
        let (request_tx, request_rx) = config;

        // 使用 request_tx 和 request_rx 初始化 SandBoxClient
        Self { request_tx,
               strategy_signal_rx: request_rx,
               local_timestamp }
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

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Pending>, ExecutionError>>
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
mod tests {
    use super::*;
    use tokio::sync::mpsc;


    #[tokio::test]
    async fn test_fetch_orders_open() {
        // 创建通道，用于请求和响应通信
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();

        let client = SandBoxClient {
            local_timestamp: 1622547800,
            request_tx: request_tx.clone(), // 请求通道的发送者用于客户端发送请求
            strategy_signal_rx: mpsc::unbounded_channel().1, // 虚拟的接收者，用于接收策略信号
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

        // 断言任务已经完成
        // 注意：在这里，我们不再需要对 client_task 进行额外的断言，因为在上面已经通过 .await 确保任务完成
        println!("Test completed");
    }
}
#[tokio::test]
async fn test_open_orders() {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    let (response_tx, response_rx) = oneshot::channel();

    let client = SandBoxClient {
        local_timestamp: 1622547800,
        request_tx: request_tx.clone(),
        strategy_signal_rx: mpsc::unbounded_channel().1,
    };

    let open_request = Order {
        kind: crate::common_infrastructure::order::OrderExecutionType::Limit,
        exchange: ExchangeVariant::Binance,
        instrument: crate::common_infrastructure::instrument::Instrument::new(
            "BTC",
            "USDT",
            crate::common_infrastructure::instrument::kind::InstrumentKind::Perpetual,
        ),
        client_ts: chrono::Utc::now().timestamp_millis(),
        client_order_id: crate::common_infrastructure::event::ClientOrderId(uuid::Uuid::new_v4()),
        side: crate::common_infrastructure::Side::Buy,
        state: RequestOpen {
            reduce_only: false,
            price: 50000.0,
            size: 1.0,
        },
    };

    let client_task = tokio::spawn(async move {
        let orders = client.open_orders(vec![open_request]).await;
        assert_eq!(orders.len(), 1, "Expected one pending order");
        assert_eq!(orders[0].as_ref().unwrap().state.price, 50000.0);
    });

    println!("Waiting to receive event from request_rx");
    if let Some(SandBoxClientEvent::OpenOrders((orders, tx))) = request_rx.recv().await {
        println!("Received OpenOrders event");
        assert_eq!(orders.len(), 1);
        let order = &orders[0];
        assert_eq!(order.state.price, 50000.0);

        match tx.send(vec![Ok(Order {
            kind: order.kind,
            exchange: order.exchange,
            instrument: order.instrument.clone(),
            client_ts: order.client_ts,
            client_order_id: order.client_order_id,
            side: order.side,
            state: Pending {
                reduce_only: order.state.reduce_only,
                price: order.state.price,
                size: order.state.size,
                predicted_ts: chrono::Utc::now().timestamp_millis(),
            },
        })]) {
            Ok(_) => println!("Response sent successfully"),
            Err(e) => panic!("Failed to send response: {:?}", e),
        }
    } else {
        panic!("Did not receive OpenOrders event");
    }

    println!("Waiting to receive response from response_rx");
    let result: Vec<Result<Order<Pending>, ExecutionError>> = match response_rx.await {
        Ok(res) => {
            println!("Received response: {:?}", res);
            res
        }
        Err(e) => {
            panic!("Failed to receive response: {:?}", e);
        }
    };

    assert_eq!(result.len(), 1, "Expected one pending order in response");

    client_task.await.expect("Client task should complete successfully");
    println!("Test completed");
}



// #[tokio::test]
    // async fn test_cancel_orders_all() {
    //     // 创建一个模拟的 SandBoxClientEvent 发射器和接收器
    //     let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    //     let (response_tx, response_rx) = oneshot::channel::<Result<Vec<Order<Cancelled>>, ExecutionError>>();
    //
    //     // 初始化 SandBoxClient
    //     let client = SandBoxClient {
    //         local_timestamp: 1622547800,
    //         request_tx: request_tx.clone(),
    //         strategy_signal_rx: request_rx,
    //     };
    //
    //     // 模拟向客户端发送 CancelOrdersAll 请求
    //     tokio::spawn(async move {
    //         let _ = client.cancel_orders_all().await;
    //     });
    //
    //     // 模拟从 SandBoxClientEvent 接收器获取 CancelOrdersAll 事件
    //     if let Some(SandBoxClientEvent::CancelOrdersAll(tx)) = request_rx.recv().await {
    //         assert_eq!(tx.send(Ok(vec![])), Ok(()));
    //     }
    //
    //     // 验证 CancelOrdersAll 的响应
    //     let result = response_rx.await.unwrap();
    //     assert!(result.is_ok());
    //     assert_eq!(result.unwrap(), vec![]);
    // }

