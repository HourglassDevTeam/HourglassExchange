/// NOTE : 在这个实现中，`oneshot`通道被用来在异步函数之间传递一次性消息。`oneshot`通道的主要特点是它只允许一个发送者和一个接收者，这使得它非常适合用于短期的、单次的请求-响应模式。这种模式在模拟交易所与客户端之间的通信中非常有效。下面详细解释为什么使用`oneshot`通道：
///     1. **单次通信**：每个请求和响应都是一次性的。每当客户端需要从模拟交易所获取数据或执行操作时，都会生成一个新的`oneshot`通道。这些通道只用于传递这一次请求的结果。`oneshot`通道确保了请求和响应的一一对应关系。
///     2. **异步编程模型**：在Rust的异步编程中，`oneshot`通道非常适合用于异步函数之间的同步。客户端在发送请求后，可以通过等待接收者来获取响应。这种方式避免了复杂的状态管理和并发问题。
///     3. **明确的生命周期**：使用`oneshot`通道，可以明确地界定请求和响应的生命周期。在请求发送后，客户端会等待响应，在响应接收后，通道就会被销毁。这避免了长时间持有资源，从而提高了程序的性能和可靠性。
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::{
    model::order::{Cancelled, Open, Order},
    simulated::SimulatedEvent,
    AccountEvent, ExecutionClient, ExecutionError, ExecutionId, RequestCancel, RequestOpen, TokenBalance,
};

/// 模拟[`ExecutionClient`]实现
/// [`SimulatedExchange`](super::exchange::SimulatedExchange).
#[derive(Clone, Debug)]
pub struct SimulatedClient {
    pub request_tx: mpsc::UnboundedSender<SimulatedEvent>,
}

#[async_trait]
impl ExecutionClient for SimulatedClient {
    type Config = mpsc::UnboundedSender<SimulatedEvent>;

    const CLIENT: ExecutionId = ExecutionId::Simulated;

    async fn init(request_tx: Self::Config, _: mpsc::UnboundedSender<AccountEvent>) -> Self {
        Self { request_tx }
    }

    // 从模拟交易所接收开放订单的响应。
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(SimulatedEvent::FetchOrdersOpen(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送获取开放订单FetchOrdersOpen请求失败");
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收获取开放订单 FetchOrdersOpen 响应失败")
    }

    // 从模拟交易所接收账户余额的响应。
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(SimulatedEvent::FetchBalances(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送获取账户余额 FetchBalances 请求失败");
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收获取账户余额 FetchBalances 响应失败")
    }

    // 从模拟交易所接收开启订单的响应。
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(SimulatedEvent::OpenOrders((open_requests, response_tx)))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 OpenOrders 请求失败");
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 OpenOrders 响应失败")
    }

    // 从模拟交易所接收取消订单的响应。
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(SimulatedEvent::CancelOrders((cancel_requests, response_tx)))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 CancelOrders 请求失败");
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 CancelOrders 响应失败")
    }

    // 从模拟交易所接收取消所有订单的响应。
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(SimulatedEvent::CancelOrdersAll(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 CancelOrdersAll 请求失败");
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 CancelOrdersAll 响应失败")
    }
}
