use account::Account;
use serde::Deserialize;
use std::fmt::Debug;
use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use warp::Filter;

use crate::{error::ExecutionError, sandbox::sandbox_client::SandBoxClientEvent};

pub mod account;
pub mod clickhouse_api;
pub mod instrument_orders;
pub mod sandbox_client;
pub mod sandbox_orderbook;
pub mod utils;
pub mod ws_trade;


// 定义网络事件结构体
#[derive(Debug, Deserialize)]
struct NetworkEvent {
    event_type: String,
    _payload: String,
    // 根据实际需要添加其他字段
}


#[derive(Debug)]
pub struct SandBoxExchange<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub event_sandbox_rx: UnboundedReceiver<SandBoxClientEvent>,
    pub account: Account<Event>,
}

impl<Event> SandBoxExchange<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn initiator() -> ExchangeInitiator<Event>
    {
        ExchangeInitiator::new()
    }


    /// 本地运行 [`SandBoxExchange`] 并响应各种 [`SandBoxClientEvent`]。
    pub async fn run_local(mut self) {
        self.process_events().await;
    }

    /// 网络运行 [`SandBoxExchange`]，并从网络接收事件
    pub async fn run_online(mut self) {
        // 创建一个通道，用于内部事件传递
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        // 创建 warp 路由
        let route = warp::path("event")
            .and(warp::body::json())
            .map(move |network_event: NetworkEvent| {
                let event_tx_clone = event_tx.clone();

                // 异步处理网络事件并发送到通道
                tokio::spawn(async move {
                    let event = match network_event.event_type.as_str() {
                        "FetchOrdersOpen" => {
                            let (response_tx, _response_rx) = oneshot::channel();
                            SandBoxClientEvent::FetchOrdersOpen(response_tx)
                        }
                        "FetchBalances" => {
                            let (response_tx, _response_rx) = oneshot::channel();
                            SandBoxClientEvent::FetchBalances(response_tx)
                        }
                        // 处理其他事件类型...
                        _ => {
                            eprintln!("Unknown event type");
                            return;
                        }
                    };

                    // 发送事件到通道
                    event_tx_clone.send(event).expect("Failed to send event");
                });

                warp::reply::reply()
            });

        // 启动 warp 服务器
        let warp_server = warp::serve(route).run(([127, 0, 0, 1], 3030));

        // 同时运行 warp 服务器和事件处理逻辑
        tokio::select! {
            _ = warp_server => {},
            _ = self.process_events() => {},
        }
    }

    /// 处理接收到的内部事件
    async fn process_events(&mut self) {
        while let Some(event) = self.event_sandbox_rx.recv().await {
            match event {
                SandBoxClientEvent::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx).await,
                SandBoxClientEvent::FetchBalances(response_tx) => self.account.fetch_balances(response_tx).await,
                SandBoxClientEvent::OpenOrders((open_requests, response_tx)) => self.account.open_requests_into_pendings(open_requests, response_tx).await,
                SandBoxClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx).await,
                SandBoxClientEvent::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx).await,
                SandBoxClientEvent::FetchMarketEvent(market_event) => self.account.match_orders(market_event).await,
            }
        }
    }
}

impl<Event> Default for ExchangeInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_sandbox_rx: Some(rx),
               account: None }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    event_sandbox_rx: Option<UnboundedReceiver<SandBoxClientEvent>>,
    account: Option<Account<Event>>,
}

impl<Event> ExchangeInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_sandbox_rx(self, value: UnboundedReceiver<SandBoxClientEvent>) -> Self
    {
        Self { event_sandbox_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Account<Event>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SandBoxExchange<Event>, ExecutionError>
    {
        Ok(SandBoxExchange { event_sandbox_rx: self.event_sandbox_rx.ok_or_else(|| ExecutionError::InitiatorIncomplete("event_sandbox_rx".to_string()))?,
                             account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))? })
    }
}
