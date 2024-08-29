use account::Account;
use mpsc::UnboundedReceiver;
use std::fmt::Debug;
use tokio::sync::mpsc;
use warp::Filter;

use crate::{
    error::ExecutionError,
    network::{event::NetworkEvent, is_port_in_use},
    sandbox::sandbox_client::SandBoxClientEvent,
};

pub mod account;
pub mod clickhouse_api;
pub mod instrument_orders;
pub mod sandbox_client;
pub mod sandbox_orderbook;
pub mod utils;
pub mod ws_trade;

#[derive(Debug)]
pub struct SandBoxExchange
{
    pub event_sandbox_rx: UnboundedReceiver<SandBoxClientEvent>,
    pub account: Account,
}

impl SandBoxExchange
{
    pub fn initiator() -> ExchangeInitiator
    {
        ExchangeInitiator::new()
    }

    /// 本地运行 [`SandBoxExchange`] 并响应各种 [`SandBoxClientEvent`]。
    pub async fn run_local(mut self)
    {
        self.process_events().await;
    }

    /// 网络运行 [`SandBoxExchange`]，并从网络接收事件
    pub async fn run_online(mut self)
    {
        let address = ([127, 0, 0, 1], 3030);

        // 检查端口是否已经被占用
        if is_port_in_use(address) {
            return;
        }

        // 创建一个通道，用于内部事件传递
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        // 创建 warp 路由
        let route = warp::path("event").and(warp::body::json()).map(move |network_event: NetworkEvent| {
                                                                   let event_tx_clone = event_tx.clone();

                                                                   // 异步处理网络事件并发送到通道
                                                                   tokio::spawn(async move {
                                                                       match network_event.parse_payload() {
                                                                           | Ok(event) => {
                                                                               // 发送事件到通道
                                                                               if let Err(e) = event_tx_clone.send(event) {
                                                                                   eprintln!("Failed to send event: {:?}", e);
                                                                               }
                                                                           }
                                                                           | Err(e) => eprintln!("Failed to parse event: {}", e),
                                                                       }
                                                                   });

                                                                   warp::reply::reply()
                                                               });

        // 启动 warp 服务器
        let warp_server = warp::serve(route).run(address);

        // 同时运行 warp 服务器和事件处理逻辑
        tokio::select! {
            _ = warp_server => {},
            _ = self.process_events() => {},
        }
    }

    /// 处理接收到的内部事件
    async fn process_events(&mut self)
    {
        while let Some(event) = self.event_sandbox_rx.recv().await {
            match event {
                | SandBoxClientEvent::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx).await,
                | SandBoxClientEvent::FetchBalances(response_tx) => self.account.fetch_balances(response_tx).await,
                | SandBoxClientEvent::OpenOrders((open_requests, response_tx)) => self.account.process_requests_into_pendings(open_requests, response_tx).await,
                | SandBoxClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx).await,
                | SandBoxClientEvent::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx).await,
                | SandBoxClientEvent::FetchMarketEvent(market_event) => self.account.match_orders(market_event).await,
            }
        }
    }
}

impl Default for ExchangeInitiator
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_sandbox_rx: Some(rx),
               account: None }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator
{
    pub(crate) event_sandbox_rx: Option<UnboundedReceiver<SandBoxClientEvent>>,
    pub(crate) account: Option<Account>,
}

impl ExchangeInitiator
{
    pub fn new() -> Self
    {
        Self { event_sandbox_rx: None,
               account: None }
    }

    pub fn event_sandbox_rx(self, value: UnboundedReceiver<SandBoxClientEvent>) -> Self
    {
        Self { event_sandbox_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Account) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SandBoxExchange, ExecutionError>
    {
        Ok(SandBoxExchange { event_sandbox_rx: self.event_sandbox_rx.ok_or_else(|| ExecutionError::InitiatorIncomplete("event_sandbox_rx".to_string()))?,
                             account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))? })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::test_util::create_test_account;
    use std::net::TcpListener;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn initiator_should_create_exchange_initiator_with_default_values()
    {
        let initiator = ExchangeInitiator::new();
        assert!(initiator.event_sandbox_rx.is_none());
        assert!(initiator.account.is_none());
    }

    #[tokio::test]
    async fn initiator_should_set_event_sandbox_rx()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let initiator = ExchangeInitiator::new().event_sandbox_rx(rx);
        assert!(initiator.event_sandbox_rx.is_some());
    }

    #[tokio::test]
    async fn initiator_should_set_account()
    {
        let account = create_test_account().await;
        let initiator = ExchangeInitiator::new().account(account);
        assert!(initiator.account.is_some());
    }

    #[tokio::test]
    async fn initiator_should_return_error_if_event_sandbox_rx_is_missing()
    {
        let account = create_test_account().await;
        let initiator = ExchangeInitiator::new().account(account);
        let result = initiator.initiate();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn initiator_should_return_error_if_account_is_missing()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let initiator = ExchangeInitiator::new().event_sandbox_rx(rx);
        let result = initiator.initiate();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_online_should_return_if_port_is_in_use()
    {
        // 占用端口 3030
        let _listener = TcpListener::bind("127.0.0.1:3030").unwrap();

        let (_tx, rx) = mpsc::unbounded_channel();
        let account: Account = create_test_account().await;
        let exchange = SandBoxExchange { event_sandbox_rx: rx, account };
        let address = ([127, 0, 0, 1], 3030);
        assert!(is_port_in_use(address));
        exchange.run_online().await;
    }
}
