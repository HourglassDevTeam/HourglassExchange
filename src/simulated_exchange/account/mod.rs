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
        event::{AccountEvent, AccountEventKind},
        order::{Cancelled, Open, Order, OrderKind, Pending, RequestCancel, RequestOpen},
        position::AccountPositions,
    },
    error::ExecutionError,
    simulated_exchange::{account::account_market_feed::AccountDataStreams, load_from_clickhouse::queries_operations::ClickhouseTrade},
    ExchangeVariant,
};
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::order::OrderRole;
use crate::common_skeleton::Side;
use crate::common_skeleton::token::Token;

pub mod account_balances;
pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;

#[derive(Clone, Debug)]
pub struct Account<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub exchange_timestamp: i64,                                    // NOTE 日后可以用无锁结构原子锁包裹
    pub data: Arc<RwLock<AccountDataStreams<Event>>>,               // 帐户数据
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,      // 帐户事件发送器
    pub market_event_tx: mpsc::UnboundedSender<MarketEvent<Event>>, // 市场事件发送器
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

    // NOTE 为给定的 MarketEvent<ClickhouseTrade> 找到对应的订单 // TO BE CONFIRMED
    pub async fn find_orders_for_an_trade_event(&self, market_event: MarketEvent<ClickhouseTrade>) -> Vec<Order<Open>>
    {
        // 读取 market_event 中的 instrument 和 side
        let instrument_kind = market_event.instrument;
        let side = market_event.kind.side;

        // 获取读锁以读取订单数据
        let orders = self.orders.read().await;

        // 从 instrument_orders_map 中查找对应的 InstrumentOrders
        if let Some(instrument_orders) = orders.instrument_orders_map.get(&instrument_kind) {
            match side.as_str() {
                | "Buy" => {
                    // 返回所有买单
                    instrument_orders.bids.clone()
                }
                | "Sell" => {
                    // 返回所有卖单
                    instrument_orders.asks.clone()
                }
                | _ => {
                    // 处理意外的 side 值
                    println!("本系统没听说过这种Side: {}", side);
                    vec![]
                }
            }
        }
        else {
            // 没有找到对应的 InstrumentOrders
            println!("未找到本则行情数据对应的未成交订单: {:?}", instrument_kind);
            vec![]
        }
    }

    pub async fn match_orders(&mut self, _market_event: MarketEvent<ClickhouseTrade>)
    {
        // todo()!
    }

    // pub async fn match_orders(&mut self, market_event: MarketEvent<ClickhouseTrade>) {
    //     // NOTE 根据 InstrumentKind 和 Side 来确定 applicable fees
    //     match market_event.kind {
    //         Spot => {
    //             let side = market_event.kind.side;
    //             match side.as_str() {
    //                 "Buy" => {
    //                     let fees_percent = self.config.read().await.current_commission_rate.spot_maker;
    //                     self.orders.read().await.match_bids(&market_event.kind, fees_percent);
    //                 }
    //                 "Sell" => {
    //                     let fees_percent = self.config.read().await.current_commission_rate.spot_taker;
    //                     self.orders.read().await.match_asks(&market_event.kind, fees_percent);
    //                 }
    //                 _ => {
    //                     // Handle unexpected side value
    //                     println!("Unexpected side: {}", side);
    //                 }
    //             }
    //         }
    //         Perpetual => {
    //             let side = market_event.kind.side;
    //             match side.as_str() {
    //                 "Buy" => {
    //                     let fees_percent = self.config.read().await.current_commission_rate.perpetual_open;
    //                     self.orders.read().await.match_bids(&market_event.kind, fees_percent);
    //                 }
    //                 "Sell" => {
    //                     let fees_percent = self.config.read().await.current_commission_rate.perpetual_close;
    //                     self.orders.read().await.match_asks(&market_event.kind, fees_percent);
    //                 }
    //                 _ => {
    //                     // Handle unexpected side value
    //                     println!("Unexpected side: {}", side);
    //                 }
    //             }
    //
    //         }
    //         _ => {
    //             // Handle unexpected InstrumentKind
    //             println!("Unexpected InstrumentKind: {:?}", market_event.kind);
    //         }
    //     }
    // }
    // let fees_percent = self.config.read().await.current_commission_rate.spot_maker;
    //
    // // Access the ClientOrders relating to the Instrument of the PublicTrade
    // let orders = match self.orders.read().await.orders_mut(&market_event.instrument) {
    //     | Ok(orders) => orders,
    //     | Err(error) => {
    //         warn!(
    //             ?error, %market_event.instrument, ?market_event.kind, "cannot match orders with unrecognised Instrument"
    //         );
    //         return;
    //     }
    // };
    //
    // // Match client Order<Open>s to incoming PublicTrade if the liquidity intersects
    // let trades = match orders.has_matching_order(&market_event.kind) {
    //     | Some(Side::Buy) => orders.match_bids(&market_event.kind, fees_percent),
    //     | Some(Side::Sell) => orders.match_asks(&market_event.kind, fees_percent),
    //     | None => return,
    // };
    //
    // // Apply Balance updates for each client Trade and send AccountEvents to client
    // for trade in trades {
    //     // Update Balances
    //     let balances_event = self.balances.update_from_trade(&trade);
    //
    //     self.account_event_tx
    //         .send(balances_event)
    //         .expect("[UniLink_Execution] : Client is offline - failed to send AccountEvent::Balances");
    //
    //     self.account_event_tx
    //         .send(AccountEvent { exchange_timestamp: self.exchange_timestamp,
    //                              exchange: ExchangeVariant::Simulated,
    //                              kind: AccountEventKind::Trade(trade) })
    //         .expect("[UniLink_Execution] : Client is offline - failed to send AccountEvent::Trade");
    // }

    // NOTE a method that generates trade from matched order is missing for the time being.

    pub async fn open_orders(&mut self, order_requests: Vec<Order<RequestOpen>>, response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>, _current_timestamp: i64)
    {
        // 循环处理每个请求并标记为 pending
        let mut open_pending = Vec::new();
        {
            let mut orders = self.orders.write().await;
            for request in &order_requests {
                // 假设 process_request_as_pending 返回 Order<Pending>
                open_pending.push(orders.process_request_as_pending(request.clone()).await);
            }
        }

        // 使用 join_all 处理异步请求
        let open_futures = open_pending.into_iter().map(|pending_order| {
                                                       let mut this = self.clone();
                                                       async move { this.try_open_order_atomic(pending_order).await }
                                                   });

        let open_results = join_all(open_futures).await;

        // 发送结果
        response_tx.send(open_results).unwrap_or_else(|_| {
                                          // 处理发送失败的情况
                                      });
    }

    // NOTE 这里不用检查订单的合法性，而是应该要和行情的时间戳对比。
    pub async fn try_open_order_atomic(&mut self, trade:ClickhouseTrade, order: Order<Pending>) -> Result<Order<Open>, ExecutionError>
    {
        // 验证订单合法性
        Self::order_validity_check(order.kind)?;

        // 计算开仓所需的可用余额
        let (symbol, required_balance) = order.calculate_required_available_balance();

        // 检查可用余额是否充足
        self.balances.read().await.has_sufficient_available_balance(symbol, required_balance)?;

        // 构建 Open<Order>
        let open = {
            // 获取写锁并构建订单
            let mut orders_guard = self.orders.write().await;
            orders_guard.build_order_open(order).await
        };

        {
            // 获取写锁并检索账户中的的 Instrument Orders，添加订单
            let mut orders_guard = self.orders.write().await;
            let orders = orders_guard.orders_mut(&open.instrument)?;
            orders.add_order_open(open.clone());
        }

        // 更新客户余额
        let balance_event = self.balances.write().await.update_from_open(&open, required_balance).await;

        // 发送账户事件给客户端 NOTE 或许需要单独建构变种。
        self.account_event_tx
            .send(balance_event)
            .expect("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Balance 失败");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp: self.exchange_timestamp,
                                 exchange: ExchangeVariant::Simulated,
                                 kind: AccountEventKind::OrdersNew(vec![open.clone()]) })
            .expect("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Trade 失败");

        // 返回已打开的订单
        Ok(open)
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
}

pub fn respond<Response>(response_tx: oneshot::Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move {
        response_tx.send(response)
                   .expect("[UniLink_Execution] : SimulatedExchange failed to send oneshot response to execution request")
    });
}
