use crate::{
    common::datafeed::market_event::MarketEvent,
    error::ExchangeError,
    hourglass::{
        account::account_handlers::{balance_handler::BalanceHandler, position_handler::PositionHandler, trade_handler::TradeHandler},
        clickhouse_api::{datatype::clickhouse_trade_data::MarketTrade, queries_operations::ClickHouseClient},
        hourglass_client_local_mode::HourglassClientEvent,
    },
    network::{event::NetworkEvent, is_port_in_use},
};
use account::HourglassAccount;
use clickhouse::query::RowCursor;
use mpsc::UnboundedReceiver;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc, mpsc::UnboundedSender, Mutex},
    time::{self, Duration},
};
use uuid::Uuid;
use warp::Filter;

pub mod account;
pub mod clickhouse_api;
pub mod config_request;
pub mod hourglass_client_local_mode;
pub mod hourglass_orderbook;
pub mod open_orders_book;
pub mod risk_reserve;
pub mod utils;
pub mod ws_trade;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;

pub enum DataSource
{
    RealTime(UnboundedReceiver<MarketEvent<MarketTrade>>),
    Backtest(RowCursor<MarketTrade>),
}
// 定义查询结果的数据结构
#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct UserInfo
{
    pub(crate) password_hash: String,
}

pub struct HourglassExchange
    where HourglassAccount: PositionHandler + TradeHandler + BalanceHandler
{
    pub client_event_rx: UnboundedReceiver<HourglassClientEvent>,
    pub market_event_tx: UnboundedSender<MarketTrade>,
    pub account: Arc<Mutex<HourglassAccount>>,
    pub data_source: DataSource,
    pub clickhouse_client: ClickHouseClient,
    pub active_sessions: Mutex<HashMap<String, Uuid>>, // 存储 session_token 和 username 的映射
}

impl HourglassExchange
{
    pub fn builder() -> ExchangeBuilder
    {
        ExchangeBuilder::new()
    }

    pub fn get_account(&self) -> Arc<Mutex<HourglassAccount>>
    {
        Arc::clone(&self.account)
    }

    #[allow(unused)]
    async fn register(&self, username: String, email: String, password: String) -> Result<(), ExchangeError>
    {
        // 加密密码
        let password_hash = hash(password, DEFAULT_COST).map_err(|_| ExchangeError::PasswordHashError)?;

        // 创建插入用户信息的 SQL
        let insert_query = format!(
                                   "INSERT INTO accounts.user_info (id, username, email, password_hash, created_at) \
            VALUES ('{}', '{}', '{}', '{}', '{}')",
                                   Uuid::new_v4(),
                                   username,
                                   email,
                                   password_hash,
                                   Utc::now()
        );

        // 执行插入操作
        self.clickhouse_client.client.read().await.query(&insert_query).execute().await.map_err(|_| ExchangeError::DatabaseError)?;

        Ok(())
    }

    #[allow(unused)]
    async fn login(&self, username: String, password: String) -> Result<String, ExchangeError>
    {
        // 查询用户的加密密码
        let select_query = format!("SELECT password_hash FROM accounts.user_info WHERE username = '{}'", username);

        // 执行查询并解析结果
        let result = self.clickhouse_client
                         .client
                         .read()
                         .await
                         .query(&select_query)
                         .fetch_one::<UserInfo>()
                         .await
                         .map_err(|_| ExchangeError::InvalidCredentials)?;

        let password_hash = result.password_hash;

        // 验证密码
        if verify(password, &password_hash).map_err(|_| ExchangeError::InvalidCredentials)? {
            let session_token = Uuid::new_v4().to_string();
            // 保存会话信息
            self.active_sessions.lock().await.insert(session_token.clone(), username.parse().unwrap());
            Ok(session_token)
        }
        else {
            Err(ExchangeError::InvalidCredentials)
        }
    }

    #[allow(unused)]
    /// 注销
    async fn logout(&self, session_token: String) -> Result<(), ExchangeError>
    {
        let mut sessions = self.active_sessions.lock().await;
        if sessions.remove(&session_token).is_some() {
            Ok(())
        }
        else {
            Err(ExchangeError::InvalidSession)
        }
    }

    pub async fn start(mut self)
    {
        let timeout = 1;
        let mut processed_count = 0; // 记录已处理的数据条目数

        loop {
            tokio::select! {
                    // 监听客户端信号
                     Some(event) = self.client_event_rx.recv() => {
                match event {
                    HourglassClientEvent::LetItRoll => {
                        if let Some(row) = self.process_next_data().await {
                            println!("processing LetItRoll");
                                println!("row: {:?}", row);
                            let mut account = self.account.lock().await;
                            let _ = account.handle_trade_data(&row).await;
                            processed_count += 1; // 每处理一个条目，计数器加1
                        } else {
                            // 如果没有更多数据
                            if processed_count > 0 {
                                println!("No more data available. Processed {} entries", processed_count);
                            } else {
                                println!("No data found.");
                            }
                            break; // 优雅退出循环
                        }
                    },
                            // 其他客户端事件处理
                            HourglassClientEvent::FetchOrdersOpen(response_tx) => {
                                self.account.lock().await.fetch_orders_open_and_respond(response_tx).await;
                            },
                            HourglassClientEvent::FetchTokenBalance(token, response_tx) => {
                                self.account.lock().await.fetch_token_balance_and_respond(&token, response_tx).await;
                            },
                            HourglassClientEvent::FetchTokenBalances(response_tx) => {
                                self.account.lock().await.fetch_token_balances_and_respond(response_tx).await;
                            },
                            HourglassClientEvent::OpenOrders((open_requests, response_tx)) => {
                                self.account.lock().await.open_orders(open_requests, response_tx).await.expect("Failed to open.");
                            },
                            HourglassClientEvent::CancelOrders((cancel_requests, response_tx)) => {
                                self.account.lock().await.cancel_orders(cancel_requests, response_tx).await;
                            },
                            HourglassClientEvent::CancelOrdersAll(response_tx) => {
                                self.account.lock().await.cancel_orders_all(response_tx).await;
                            },
                            HourglassClientEvent::FetchAllPositions(response_tx) => {
                                self.account.lock().await.fetch_positions_and_respond(response_tx).await;
                            },
                            HourglassClientEvent::FetchLongPosition(instrument, response_tx) => {
                                self.account.lock().await.fetch_long_position_and_respond(&instrument, response_tx).await;
                            },
                            HourglassClientEvent::FetchShortPosition(instrument, response_tx) => {
                                self.account.lock().await.fetch_short_position_and_respond(&instrument, response_tx).await;

                            },
                            HourglassClientEvent::DepositTokens(deposit_request) => {
                                self.account.lock().await.deposit_multiple_coins_and_respond(deposit_request.0, deposit_request.1).await;
                            },
                            HourglassClientEvent::ConfigureInstruments(position_configs, response_tx) => {
                                let _ = self.account.lock().await.preconfigure_positions(position_configs, response_tx).await;
                            }

                        HourglassClientEvent::Login(_) => {
                            todo!()
                        }

                        HourglassClientEvent::Register(_) => {
                            todo!()
                        }

                        HourglassClientEvent::Logout(_) => {
                todo!()
                        }
                    }
                }
                    // 加入超时机制，防止一直挂起
            _ = time::sleep(Duration::from_secs(timeout)) => {
                if processed_count > 0 {
                    println!("No more data available.");
                } else {
                    println!("No data found.");
                }
                break; // 超时后优雅退出循环
            }
                }
        }
    }

    /// 处理下一条数据
    async fn process_next_data(&mut self) -> Option<MarketTrade>
    {
        match &mut self.data_source {
            | DataSource::Backtest(cursor) => {
                // 这里 cursor 需要是 mutable 的
                if let Ok(Some(row)) = cursor.next().await {
                    // 发送市场数据给客户端
                    if let Err(e) = self.market_event_tx.send(row.clone()) {
                        eprintln!("Failed to send market data to client: {:?}", e);
                    }
                    Some(row)
                }
                else {
                    None
                }
            }
            | _ => {
                println!("Unhandled data source type");
                None
            }
        }
    }

    /// 网络运行 [`HourglassExchange`]，并从网络接收事件
    pub async fn run_online(self)
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
            _ = self.start() => {},
        }
    }
}

impl Default for ExchangeBuilder
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_hourglass_rx: Some(rx),
               account: None,
               market_event_tx: None,
               data_source: None }
    }
}
pub struct ExchangeBuilder
{
    pub(crate) event_hourglass_rx: Option<UnboundedReceiver<HourglassClientEvent>>,
    pub(crate) account: Option<Arc<Mutex<HourglassAccount>>>,
    pub(crate) market_event_tx: Option<UnboundedSender<MarketTrade>>,
    pub(crate) data_source: Option<DataSource>,
}

impl ExchangeBuilder
{
    pub fn new() -> Self
    {
        Self { event_hourglass_rx: None,
               account: None,
               market_event_tx: None,
               data_source: None }
    }

    pub fn event_hourglass_rx(self, value: UnboundedReceiver<HourglassClientEvent>) -> Self
    {
        Self { event_hourglass_rx: Some(value),
               ..self }
    }

    pub fn data_source(self, value: DataSource) -> Self
    {
        Self { data_source: Some(value), ..self }
    }

    pub fn market_event_tx(self, value: UnboundedSender<MarketTrade>) -> Self
    {
        Self { market_event_tx: Some(value), ..self }
    }

    pub fn account(self, value: Arc<Mutex<HourglassAccount>>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<HourglassExchange, ExchangeError>
    {
        Ok(HourglassExchange { client_event_rx: self.event_hourglass_rx.ok_or_else(|| ExchangeError::BuilderIncomplete("event_hourglass_rx".to_string()))?,
                               // market_event_tx: self.market_event_tx.ok_or_else(|| ExecutionError::BuilderIncomplete("market_event_tx".to_string()))?,
                               market_event_tx: self.market_event_tx.ok_or_else(|| ExchangeError::BuilderIncomplete("market_tx".to_string()))?,
                               account: self.account.ok_or_else(|| ExchangeError::BuilderIncomplete("account".to_string()))?,
                               data_source: self.data_source.ok_or_else(|| ExchangeError::BuilderIncomplete("data_source".to_string()))?,
                               clickhouse_client: ClickHouseClient::new(),
                               active_sessions: HashMap::new().into() })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{hourglass::clickhouse_api::queries_operations::ClickHouseClient, test_utils::create_test_account};
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
        let (market_tx, _market_rx) = mpsc::unbounded_channel();
        let clickhouse_client = ClickHouseClient::new();
        let exchange = "binance";
        let instrument = "futures";
        let date = "2024_05_05";
        let cursor = clickhouse_client.cursor_unioned_public_trades(exchange, instrument, date).await.unwrap();

        let (_tx, rx) = mpsc::unbounded_channel();
        let account = create_test_account().await;
        let account = Arc::new(Mutex::new(account)); // Wrap `Account` in `Arc<Mutex<Account>>`
        let exchange = HourglassExchange { client_event_rx: rx,
                                           market_event_tx: market_tx,
                                           account,
                                           data_source: DataSource::Backtest(cursor),
                                           clickhouse_client: ClickHouseClient::new(),
                                           active_sessions: HashMap::new().into() };
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
