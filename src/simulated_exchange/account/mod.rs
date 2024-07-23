use std::{fmt::Debug, sync::Arc};

use futures::future::join_all;
use tokio::sync::{mpsc, oneshot, RwLock};

use account_balances::AccountBalances;
use account_config::AccountConfig;
use account_orders::AccountOrders;

use crate::{
    common_skeleton::{
        balance::TokenBalance,
        datafeed::event::MarketEvent,
        event::AccountEvent,
        order::{Cancelled, Open, Order, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
    },
    error::ExecutionError,
    simulated_exchange::{
        account::{
            account_latency::{fluctuate_latency, AccountLatency},
            account_market_feed::AccountDataStreams,
        },
        load_from_clickhouse::queries_operations::ClickhouseTrade,
    },
};

pub mod account_balances;
pub mod account_config;
mod account_latency;
pub mod account_market_feed;
pub mod account_orders;
pub mod slippage_registry;

#[derive(Clone, Debug)]
pub struct Account<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub exchange_timestamp: i64,                                    // NOTE 日后可以用无锁结构原子锁包裹
    pub data: Arc<RwLock<AccountDataStreams<Event>>>,               // 帐户数据
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,      // 帐户事件发送器
    pub market_event_tx: mpsc::UnboundedSender<MarketEvent<Event>>, // 市场事件发送器
    pub latency: Arc<RwLock<AccountLatency>>,                       // 帐户延迟
    pub config: Arc<RwLock<AccountConfig>>,                         // 帐户配置
    pub balances: Arc<RwLock<AccountBalances<Event>>>,              // 帐户余额
    pub positions: Arc<RwLock<Vec<AccountPositions>>>,              // 帐户头寸
    pub orders: Arc<RwLock<AccountOrders>>,
}

#[derive(Clone, Debug)]
pub struct AccountInitiator<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    data: Option<Arc<RwLock<AccountDataStreams<Event>>>>,
    account_event_tx: Option<mpsc::UnboundedSender<AccountEvent>>,
    market_event_tx: Option<mpsc::UnboundedSender<MarketEvent<Event>>>,
    latency: Option<Arc<RwLock<AccountLatency>>>,
    config: Option<Arc<RwLock<AccountConfig>>>,
    balances: Option<Arc<RwLock<AccountBalances<Event>>>>,
    positions: Option<Arc<RwLock<Vec<AccountPositions>>>>,
    orders: Option<Arc<RwLock<AccountOrders>>>,
}

impl<Event> AccountInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn new() -> Self
    {
        AccountInitiator { data: None,
                           account_event_tx: None,
                           market_event_tx: None,
                           latency: None,
                           config: None,
                           balances: None,
                           positions: None,
                           orders: None }
    }

    pub fn data(mut self, value: AccountDataStreams<Event>) -> Self
    {
        self.data = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn account_event_tx(mut self, value: mpsc::UnboundedSender<AccountEvent>) -> Self
    {
        self.account_event_tx = Some(value);
        self
    }

    pub fn market_event_tx(mut self, value: mpsc::UnboundedSender<MarketEvent<Event>>) -> Self
    {
        self.market_event_tx = Some(value);
        self
    }

    pub fn latency(mut self, value: AccountLatency) -> Self
    {
        self.latency = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self
    {
        self.config = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn balances(mut self, value: AccountBalances<Event>) -> Self
    {
        self.balances = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn positions(mut self, value: Vec<AccountPositions>) -> Self
    {
        self.positions = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn build(self) -> Result<Account<Event>, String>
    {
        Ok(Account { exchange_timestamp: 0,
                     data: self.data.ok_or("datafeed is required")?,                                 // 检查并获取data
                     account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?, // 检查并获取account_event_tx
                     market_event_tx: self.market_event_tx.ok_or("market_event_tx is required")?,    // 检查并获取market_event_tx
                     latency: self.latency.ok_or("latency is required")?,                            // 检查并获取latency
                     config: self.config.ok_or("config is required")?,                               // 检查并获取config
                     balances: self.balances.ok_or("balances is required")?,                         // 检查并获取balances
                     positions: self.positions.ok_or("positions are required")?,                     // 检查并获取positions
                     orders: self.orders.ok_or("orders are required")? })
    }
}

// NOTE 未完成
#[allow(dead_code)]
impl<Event> Account<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn initiate() -> AccountInitiator<Event>
    {
        AccountInitiator::new()
    }

    pub async fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>)
    {
        let orders = self.orders.read().await.fetch_all();
        respond(response_tx, Ok(orders)); // 是否要模拟延迟
    }

    pub async fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>)
    {
        let balances = self.balances.read().await.fetch_all();
        respond(response_tx, Ok(balances));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError>
    {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::PostOnly | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                                          * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    pub async fn fetch_positions(&self, response_tx: oneshot::Sender<Result<Vec<AccountPositions>, ExecutionError>>)
    {
        let positions = self.positions.read().await.clone();
        respond(response_tx, Ok(positions));
    }

    // pub async fn match_in_trade_distribution_loop(&mut self, market_event: MarketEvent<ClickhouseTrade>)
    // {
    //     todo!()
    // }

    // pub async fn match_orders(&mut self, _instrument: Instrument, _trade: ClickhouseTrade)
    // {
    //     let fees_percent = self.config.read().await.current_commission_rate.spot_maker;
    //
    //     // Access the ClientOrders relating to the Instrument of the PublicTrade
    //     let orders = match self.orders.orders_mut(&instrument) {
    //         | Ok(orders) => orders,
    //         | Err(error) => {
    //             warn!(
    //                 ?error, %instrument, ?trade, "cannot match orders with unrecognised Instrument"
    //             );
    //             return;
    //         }
    //     };
    //
    //     // Match client Order<Open>s to incoming PublicTrade if the liquidity intersects
    //     let trades = match orders.has_matching_order(&trade) {
    //         | Some(Side::Buy) => orders.match_bids(&trade, fees_percent),
    //         | Some(Side::Sell) => orders.match_asks(&trade, fees_percent),
    //         | None => return,
    //     };
    //
    //     // Apply Balance updates for each client Trade and send AccountEvents to client
    //     for trade in trades {
    //         // Update Balances
    //         let balances_event = self.balances.update_from_trade(&trade);
    //
    //         self.event_account_tx
    //             .send(balances_event)
    //             .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Balances");
    //
    //         self.event_account_tx
    //             .send(AccountEvent {
    //                 received_time: Utc::now(),
    //                 exchange: Exchange::from(ExchangeKind::Simulated),
    //                 kind: AccountEventKind::Trade(trade),
    //             })
    //             .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Trade");
    //     }
    // }

    pub async fn open_orders(&mut self, open_requests: Vec<Order<RequestOpen>>, response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>, current_timestamp: i64)
    {
        let open_futures = open_requests.into_iter().map(|request| {
                                                        let mut this = self.clone();
                                                        async move { this.try_open_order_atomic(request, current_timestamp).await }
                                                    });

        let open_results = join_all(open_futures).await;
        response_tx.send(open_results).unwrap_or_else(|_| {
                                          // Handle the error if sending fails
                                      });
    }

    pub async fn try_open_order_atomic(&mut self, request: Order<RequestOpen>, _current_timestamp: i64) -> Result<Order<Open>, ExecutionError>
    {
        Self::order_validity_check(request.kind).unwrap();
        todo!()
    }

    pub async fn cancel_orders(&mut self,
                               cancel_requests: Vec<Order<RequestCancel>>,
                               response_tx: oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
                               current_timestamp: i64)
    {
        let cancel_futures = cancel_requests.into_iter().map(|request| {
                                                            let mut this = self.clone();
                                                            async move { this.try_cancel_order_atomic(request, current_timestamp).await }
                                                        });

        let cancel_results = join_all(cancel_futures).await;
        response_tx.send(cancel_results).unwrap_or_else(|_| {
                                            // Handle the error if sending fails
                                        });
    }

    pub async fn try_cancel_order_atomic(&mut self, _request: Order<RequestCancel>, _current_timestamp: i64) -> Result<Order<Cancelled>, ExecutionError>
    {
        todo!()
    }

    pub async fn cancel_orders_all(&mut self, _response_tx: oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>, _current_timestamp: i64)
    {
        todo!()
    }

    pub async fn update_latency(&mut self, current_time: i64)
    {
        let mut latency = self.latency.write().await;
        fluctuate_latency(&mut *latency, current_time);
    }
}

pub fn respond<Response>(response_tx: oneshot::Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move {
        response_tx.send(response)
                   .expect("[UniLinkExecution] : SimulatedExchange failed to send oneshot response to execution request")
    });
}
