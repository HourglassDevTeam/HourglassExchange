use crate::{
    error::ExchangeError,
    hourglass::{
        account::account_handlers::{balance_handler::BalanceHandler, position_handler::PositionHandler, trade_handler::TradeHandler},
        hourglass_client::HourglassClientEvent,
    },
    network::{event::NetworkEvent, is_port_in_use},
};
use account::HourglassAccount;
use mpsc::UnboundedReceiver;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use warp::Filter;

pub mod account;
pub mod clickhouse_api;
pub mod config_request;
pub mod hourglass_client;
pub mod hourglass_orderbook;
pub mod open_orders_book;
pub mod risk_reserve;
pub mod utils;
pub mod ws_trade;
// pub enum TradeEventSource {
//     RealTime(UnboundedReceiver<MarketEvent<MarketTrade>>),
//     Backtest(RowCursor<MarketTrade>),
// }

pub struct HourglassExchange
    where HourglassAccount: PositionHandler + TradeHandler + BalanceHandler
{
    /// data_source could be added here as a daughter struct with variants.
    // #[allow(dead_code)]
    // pub data_source: TradeEventSource,
    pub event_hourglass_rx: UnboundedReceiver<HourglassClientEvent>,
    // pub market_event_tx: UnboundedReceiver<MarketEvent<MarketTrade>>,
    pub account: Arc<Mutex<HourglassAccount>>,
}

impl HourglassExchange
{
    pub fn builder() -> ExchangeBuilder
    {
        ExchangeBuilder::new()
    }


    pub fn get_account(&self) -> Arc<Mutex<HourglassAccount>> {
        Arc::clone(&self.account)
    }

    /// 本地运行 [`HourglassExchange`] 并响应各种 [`HourglassClientEvent`]。
    pub async fn run_local(mut self)
    {
        self.process_events().await;
    }

    /// 网络运行 [`HourglassExchange`]，并从网络接收事件
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
        while let Some(event) = self.event_hourglass_rx.recv().await {
            match event {
                | HourglassClientEvent::FetchOrdersOpen(response_tx) => self.account.lock().await.fetch_orders_open_and_respond(response_tx).await,
                | HourglassClientEvent::FetchTokenBalance(token, response_tx) => self.account.lock().await.fetch_token_balance_and_respond(&token, response_tx).await,
                | HourglassClientEvent::FetchTokenBalances(response_tx) => self.account.lock().await.fetch_token_balances_and_respond(response_tx).await,
                | HourglassClientEvent::OpenOrders((open_requests, response_tx)) => self.account.lock().await.open_orders(open_requests, response_tx).await.expect("Failed to open."),
                | HourglassClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.lock().await.cancel_orders(cancel_requests, response_tx).await,
                | HourglassClientEvent::CancelOrdersAll(response_tx) => self.account.lock().await.cancel_orders_all(response_tx).await,
                | HourglassClientEvent::FetchAllPositions(response_tx) => self.account.lock().await.fetch_positions_and_respond(response_tx).await,
                | HourglassClientEvent::FetchLongPosition(instrument, response_tx) => self.account.lock().await.fetch_long_position_and_respond(&instrument, response_tx).await,
                | HourglassClientEvent::FetchShortPosition(instrument, response_tx) => self.account.lock().await.fetch_short_position_and_respond(&instrument, response_tx).await,
                | HourglassClientEvent::DepositTokens(deposit_request) => {
                    self.account.lock().await.deposit_multiple_coins_and_respond(deposit_request.0, deposit_request.1).await;
                }
                | HourglassClientEvent::ConfigureInstruments(position_configs, response_tx) => {
                    let _ = self.account.lock().await.preconfigure_positions(position_configs, response_tx).await;
                }
                | HourglassClientEvent::LetItRoll => {
                    println!("Received NoOp event, no action's taken.");
                }
            }
        }
    }
}

impl Default for ExchangeBuilder
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_hourglass_rx: Some(rx),
               account: None /* market_event_tx: None,
                              * data_source: None, */ }
    }
}
pub struct ExchangeBuilder
{
    pub(crate) event_hourglass_rx: Option<UnboundedReceiver<HourglassClientEvent>>,
    pub(crate) account: Option<Arc<Mutex<HourglassAccount>>>,
    // pub(crate) market_event_tx: Option<UnboundedReceiver<MarketEvent<MarketTrade>>>,
    // pub(crate) data_source: Option<TradeEventSource>,
}

impl ExchangeBuilder
{
    pub fn new() -> Self
    {
        Self { event_hourglass_rx: None,
               account: None /* market_event_tx: None,
                              * data_source: None, */ }
    }

    pub fn event_hourglass_rx(self, value: UnboundedReceiver<HourglassClientEvent>) -> Self
    {
        Self { event_hourglass_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Arc<Mutex<HourglassAccount>>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<HourglassExchange, ExchangeError>
    {
        Ok(HourglassExchange { event_hourglass_rx: self.event_hourglass_rx.ok_or_else(|| ExchangeError::BuilderIncomplete("event_hourglass_rx".to_string()))?,
                               // market_event_tx: self.market_event_tx.ok_or_else(|| ExecutionError::BuilderIncomplete("market_event_tx".to_string()))?,
                               account: self.account.ok_or_else(|| ExchangeError::BuilderIncomplete("account".to_string()))? })
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
    async fn builder_should_create_exchange_builder_with_default_values()
    {
        let builder = ExchangeBuilder::new();
        assert!(builder.event_hourglass_rx.is_none());
        assert!(builder.account.is_none());
    }

    #[tokio::test]
    async fn builder_should_set_event_hourglass_rx()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let builder = ExchangeBuilder::new().event_hourglass_rx(rx);
        assert!(builder.event_hourglass_rx.is_some());
    }

    #[tokio::test]
    async fn builder_should_set_account()
    {
        let account = create_test_account().await;
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let builder = ExchangeBuilder::new().account(account.clone());
        assert!(builder.account.is_some());
    }

    #[tokio::test]
    async fn builder_should_return_error_if_event_hourglass_rx_is_missing()
    {
        let account = create_test_account().await;
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let builder = ExchangeBuilder::new().account(account.clone());
        let result = builder.initiate();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn builder_should_return_error_if_account_is_missing()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let builder = ExchangeBuilder::new().event_hourglass_rx(rx);
        let result = builder.initiate();
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
        let exchange = HourglassExchange { event_hourglass_rx: rx, account };
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
