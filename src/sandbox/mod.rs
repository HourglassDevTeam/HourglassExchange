use crate::{
    error::ExchangeError,
    network::{event::NetworkEvent, is_port_in_use},
    sandbox::sandbox_client::SandBoxClientEvent,
};
use account::Account;
use mpsc::UnboundedReceiver;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use warp::Filter;

pub mod account;
pub mod clickhouse_api;
pub mod config_request;
pub mod instrument_orders;
pub mod sandbox_client;
pub mod sandbox_orderbook;
pub mod utils;
pub mod ws_trade;
// pub enum TradeEventSource {
//     RealTime(UnboundedReceiver<MarketEvent<MarketTrade>>),
//     Backtest(RowCursor<MarketTrade>),
// }

pub struct SandBoxExchange
{
    /// data_source could be added here as a daughter struct with variants.
    // #[allow(dead_code)]
    // pub data_source: TradeEventSource,
    pub event_sandbox_rx: UnboundedReceiver<SandBoxClientEvent>,
    // pub market_event_tx: UnboundedReceiver<MarketEvent<MarketTrade>>,
    pub account: Arc<Mutex<Account>>,
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
                | SandBoxClientEvent::FetchOrdersOpen(response_tx) => self.account.lock().await.fetch_orders_open_and_respond(response_tx).await,
                | SandBoxClientEvent::FetchTokenBalance(token, response_tx) => self.account.lock().await.fetch_token_balance_and_respond(&token, response_tx).await,
                | SandBoxClientEvent::FetchTokenBalances(response_tx) => self.account.lock().await.fetch_token_balances_and_respond(response_tx).await,
                | SandBoxClientEvent::OpenOrders((open_requests, response_tx)) => self.account.lock().await.open_orders(open_requests, response_tx).await.expect("Failed to open."),
                | SandBoxClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.lock().await.cancel_orders(cancel_requests, response_tx).await,
                | SandBoxClientEvent::CancelOrdersAll(response_tx) => self.account.lock().await.cancel_orders_all(response_tx).await,
                | SandBoxClientEvent::FetchAllPositions(response_tx) => self.account.lock().await.fetch_positions_and_respond(response_tx).await,
                | SandBoxClientEvent::FetchLongPosition(instrument, response_tx) => self.account.lock().await.fetch_long_position_and_respond(&instrument, response_tx).await,
                | SandBoxClientEvent::FetchShortPosition(instrument, response_tx) => self.account.lock().await.fetch_short_position_and_respond(&instrument, response_tx).await,
                | SandBoxClientEvent::DepositTokens(deposit_request) => {
                    self.account.lock().await.deposit_multiple_coins_and_respond(deposit_request.0, deposit_request.1).await;
                }
                | SandBoxClientEvent::ConfigureInstruments(position_configs, response_tx) => {
                    let _ = self.account.lock().await.preconfigure_positions(position_configs, response_tx).await;
                }
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
               account: None /* market_event_tx: None,
                              * data_source: None, */ }
    }
}
pub struct ExchangeInitiator
{
    pub(crate) event_sandbox_rx: Option<UnboundedReceiver<SandBoxClientEvent>>,
    pub(crate) account: Option<Arc<Mutex<Account>>>,
    // pub(crate) market_event_tx: Option<UnboundedReceiver<MarketEvent<MarketTrade>>>,
    // pub(crate) data_source: Option<TradeEventSource>,
}

impl ExchangeInitiator
{
    pub fn new() -> Self
    {
        Self { event_sandbox_rx: None,
               account: None /* market_event_tx: None,
                              * data_source: None, */ }
    }

    pub fn event_sandbox_rx(self, value: UnboundedReceiver<SandBoxClientEvent>) -> Self
    {
        Self { event_sandbox_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Arc<Mutex<Account>>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SandBoxExchange, ExchangeError>
    {
        Ok(SandBoxExchange { event_sandbox_rx: self.event_sandbox_rx.ok_or_else(|| ExchangeError::InitiatorIncomplete("event_sandbox_rx".to_string()))?,
                             // market_event_tx: self.market_event_tx.ok_or_else(|| ExecutionError::InitiatorIncomplete("market_event_tx".to_string()))?,
                             account: self.account.ok_or_else(|| ExchangeError::InitiatorIncomplete("account".to_string()))? })
    }

    // pub fn trade_event_source(self, value: TradeEventSource) -> Self
    // {
    //     Self { data_source: Some(value), ..self }
    // }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::test_utils::create_test_account;
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
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let initiator = ExchangeInitiator::new().account(account.clone());
        assert!(initiator.account.is_some());
    }

    #[tokio::test]
    async fn initiator_should_return_error_if_event_sandbox_rx_is_missing()
    {
        let account = create_test_account().await;
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let initiator = ExchangeInitiator::new().account(account.clone());
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
        let account = create_test_account().await;
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let exchange = SandBoxExchange { event_sandbox_rx: rx, account };
        let address = "127.0.0.1:3030".parse().unwrap(); // Convert to a SocketAddr
        assert!(is_port_in_use(address));
        exchange.run_online().await;
    }
    // Function to check if a port is in use
    fn is_port_in_use(address: std::net::SocketAddr) -> bool
    {
        TcpListener::bind(address).is_err()
    }
}
