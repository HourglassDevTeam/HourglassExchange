use crate::common_infrastructure::event::ClientOrderId;
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IndexedParallelIterator;
use futures::{future::join_all, lock::Mutex};
use mpsc::UnboundedSender;
use oneshot::Sender;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::warn;

use account_config::AccountConfig;
use account_orders::AccountOrders;
use account_states::AccountState;

use crate::{
    common_infrastructure::{
        balance::TokenBalance,
        datafeed::event::MarketEvent,
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{Cancelled, Open, Order, OrderKind, OrderRole, Pending, RequestCancel, RequestOpen},
        position::AccountPositions,
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::{account::account_market_feed::AccountDataStreams, clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade, instrument_orders::InstrumentOrders},
    ExchangeVariant,
};

pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;
pub mod account_states;

#[derive(Debug)]
pub struct Account<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub exchange_timestamp: AtomicI64,
    pub data: Arc<RwLock<AccountDataStreams<Event>>>,         // 帐户数据
    pub account_event_tx: UnboundedSender<AccountEvent>,      // 帐户事件发送器
    pub market_event_tx: UnboundedSender<MarketEvent<Event>>, // 市场事件发送器
    pub config: Arc<AccountConfig>,                           // 帐户配置
    pub states: Arc<Mutex<AccountState<Event>>>,              // 帐户余额
    pub orders: Arc<RwLock<AccountOrders>>,
}

// 手动实现 Clone trait
impl<Event> Clone for Account<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn clone(&self) -> Self
    {
        Account { exchange_timestamp: AtomicI64::new(self.exchange_timestamp.load(Ordering::SeqCst)),
                  data: Arc::clone(&self.data),
                  account_event_tx: self.account_event_tx.clone(),
                  market_event_tx: self.market_event_tx.clone(),
                  config: Arc::clone(&self.config),
                  states: Arc::clone(&self.states),
                  orders: Arc::clone(&self.orders) }
    }
}
#[derive(Clone, Debug)]
pub struct AccountInitiator<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    data: Option<Arc<RwLock<AccountDataStreams<Event>>>>,
    account_event_tx: Option<UnboundedSender<AccountEvent>>,
    market_event_tx: Option<UnboundedSender<MarketEvent<Event>>>,
    config: Option<Arc<AccountConfig>>,
    states: Option<Arc<Mutex<AccountState<Event>>>>,
    orders: Option<Arc<RwLock<AccountOrders>>>,
}

impl<Event> Default for AccountInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<Event> AccountInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn new() -> Self
    {
        AccountInitiator { data: None,
                           account_event_tx: None,
                           market_event_tx: None,
                           config: None,
                           states: None,
                           orders: None }
    }

    pub fn data(mut self, value: AccountDataStreams<Event>) -> Self
    {
        self.data = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn account_event_tx(mut self, value: UnboundedSender<AccountEvent>) -> Self
    {
        self.account_event_tx = Some(value);
        self
    }

    pub fn market_event_tx(mut self, value: UnboundedSender<MarketEvent<Event>>) -> Self
    {
        self.market_event_tx = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self
    {
        self.config = Some(Arc::new(value));
        self
    }

    pub fn balances(mut self, value: AccountState<Event>) -> Self
    {
        self.states = Some(Arc::new(Mutex::new(value)));
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn build(self) -> Result<Account<Event>, String>
    {
        Ok(Account { exchange_timestamp: 0.into(),
                     data: self.data.ok_or("datafeed is required")?,                                 // 检查并获取data
                     account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?, // 检查并获取account_event_tx
                     market_event_tx: self.market_event_tx.ok_or("market_event_tx is required")?,    // 检查并获取market_event_tx
                     config: self.config.ok_or("config is required")?,                               // 检查并获取config
                     states: self.states.ok_or("balances is required")?,                             // 检查并获取balances
                     orders: self.orders.ok_or("orders are required")? })
    }
}

#[allow(dead_code)]
impl<Event> Account<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    /// [Initiation] `Account` 模块的初始化函数`initiate`
    pub fn initiate() -> AccountInitiator<Event>
    {
        AccountInitiator::new()
    }

    /// [PART1]
    /// `update_exchange_timestamp` 是基本的时间戳更新方法，用于更新 `exchange_timestamp` 值。
    /// `fetch_orders_open` 发送当前所有开放的订单给调用者，用于获取所有未完成的订单。
    /// `fetch_balances` 发送当前所有代币的余额信息，用于获取账户中所有代币的账本数据。
    /// `fetch_positions` 发送当前所有代币的持仓信息，用于获取账户中所有代币的仓位数据。
    pub fn update_exchange_timestamp(&self, timestamp: i64)
    {
        self.exchange_timestamp.store(timestamp, Ordering::SeqCst);
    }

    pub async fn fetch_orders_open(&self, response_tx: Sender<Result<Vec<Order<Open>>, ExecutionError>>)
    {
        let orders = self.orders.read().await.fetch_all();
        respond(response_tx, Ok(orders)); // 是否要模拟延迟
    }

    pub async fn fetch_balances(&self, response_tx: Sender<Result<Vec<TokenBalance>, ExecutionError>>)
    {
        let balances = self.states.lock().await.fetch_all_balances();
        respond(response_tx, Ok(balances));
    }

    pub async fn fetch_positions(&self, response_tx: Sender<Result<AccountPositions, ExecutionError>>)
    {
        let positions = self.states.lock().await.positions.clone();
        respond(response_tx, Ok(positions));
    }

    /// [PART 2]
    /// `try_open_order_atomic` 尝试以原子操作方式打开一个订单，确保在验证和更新账户余额后安全地打开订单。
    /// `open_requests_into_pendings` 处理一组订单请求，将其转换为挂起订单，并在成功后更新状态。
    /// `calculate_required_available_balance` 计算打开订单所需的可用余额，用于验证账户中是否有足够的资金执行订单。
    // NOTE 注意size的单位
    pub async fn calculate_required_available_balance<'a>(&'a self, order: &'a Order<Pending>, current_price: f64) -> (&Token, f64)
    {
        match order.instrument.kind {
            | InstrumentKind::Spot => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size),
                | Side::Sell => (&order.instrument.base, order.state.size),
            },
            | InstrumentKind::Perpetual => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size * self.config.account_leverage_rate),
                | Side::Sell => (&order.instrument.base, order.state.size * self.config.account_leverage_rate),
            },
            | InstrumentKind::Future => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size * self.config.account_leverage_rate),
                | Side::Sell => (&order.instrument.base, order.state.size * self.config.account_leverage_rate),
            },
            | InstrumentKind::CryptoOption => {
                todo!()
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!()
            }
            | InstrumentKind::CommodityOption => {
                todo!()
            }
            | InstrumentKind::CommodityFuture => {
                todo!()
            }
        }
    }

    pub async fn try_open_order_atomic(&mut self, current_price: f64, order: Order<Pending>) -> Result<Order<Open>, ExecutionError>
    {
        // 验证订单合法性
        Self::order_validity_check(order.kind)?;

        // 获取订单角色（maker 或 taker），现在使用写锁
        let order_role = {
            let mut orders_guard = self.orders.write().await;
            orders_guard.determine_maker_taker(&order, current_price)?
        };

        // 计算所需的可用余额
        let (token, required_balance) = self.calculate_required_available_balance(&order, current_price).await;

        // 检查可用余额是否充足
        self.states.lock().await.has_sufficient_available_balance(token, required_balance)?;

        // 构建 Open<Order> 并获取写锁
        let open_order = {
            let mut orders_guard = self.orders.write().await;

            // 构建 Open<Order>
            let open = orders_guard.build_order_open(order, order_role).await;

            // 添加订单到 Instrument Orders
            orders_guard.ins_orders_mut(&open.instrument)?.add_order_open(open.clone());

            open
        };

        // 更新客户余额
        let balance_event = self.states.lock().await.apply_open_order_changes(&open_order, required_balance).await.unwrap();

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送账户事件给客户端
        self.account_event_tx
            .send(balance_event)
            .expect("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Balance");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: ExchangeVariant::SandBox,
                                 kind: AccountEventKind::OrdersNew(vec![open_order.clone()]) })
            .expect("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Trade");
        // 返回已打开的订单
        Ok(open_order)
    }

    pub async fn open_requests_into_pendings(&mut self, order_requests: Vec<Order<RequestOpen>>, response_tx: Sender<Vec<Result<Order<Pending>, ExecutionError>>>)
    {
        // 创建一个用于存储 Pending 订单的临时向量
        let mut open_pending = Vec::new();

        {
            // 获取写锁并处理每个请求，将其标记为 pending
            let mut orders = self.orders.write().await;
            for request in &order_requests {
                // 假设 process_request_as_pending 返回 Order<Pending>
                // 将每个 Order<Pending> 包装在 Ok 中
                let pending_order = orders.process_request_as_pending(request.clone()).await;
                open_pending.push(Ok(pending_order));
            } // NOTE 在这个大括号内结束时，orders 的写锁会被释放，但 open_pending 仍然有效
        }

        // 这里的 open_pending 仍然是上面声明的那个向量，并未被 drop
        if response_tx.send(open_pending).is_err() {
            eprintln!("[UniLinkExecution] : Failed to send RequestOpen response");
        }
    }

    /// [PART3]
    /// `order_validity_check` 验证订单的合法性，确保订单类型是受支持的。
    /// `match_orders` 处理市场事件，根据市场事件匹配相应的订单并生成交易。
    /// `get_orders_for_instrument` 获取与特定金融工具相关的订单，用于进一步的订单匹配操作。
    /// `match_orders_by_side` 根据订单的买卖方向（Side）匹配订单并生成交易事件。
    /// `determine_fees_percent` 根据金融工具类型和订单方向确定适用的费用百分比。

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError>
    {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::PostOnly | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                                          * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }
    pub async fn match_orders(&mut self, market_event: MarketEvent<ClickhousePublicTrade>) {
        let current_price = market_event.kind.price;

        // 这里不再克隆整个 `pending_registry`，而是逐个处理订单
        let order_ids: Vec<ClientOrderId> = self.orders.read().await.pending_registry.keys().cloned().collect();

        // 遍历订单 ID 来处理每个订单
        for order_id in order_ids {
            // 在需要时获取写锁来调用可变方法
            let mut orders_write = self.orders.write().await;

            // 获取当前的挂单并克隆它
            if let Some(order) = orders_write.pending_registry.get(&order_id).cloned() {
                let role = orders_write.determine_maker_taker(&order, current_price);

                if let Ok(OrderRole::Maker) = role {
                    // 生成 open_order
                    let open_order = orders_write.build_order_open(order.clone(), OrderRole::Maker).await;

                    // 处理 instrument_orders
                    if let Ok(instrument_orders) = orders_write.ins_orders_mut(&order.instrument) {
                        // 将订单加入到相应的订单簿
                        instrument_orders.add_order_open(open_order.clone());

                        // 获取手续费
                        let fees_percent = self.determine_fees_percent(&order.instrument.kind, &OrderRole::Maker);

                        // 匹配订单并生成交易
                        let trades = match instrument_orders.determine_matching_side(&market_event) {
                            Some(Side::Buy) => instrument_orders.match_bids(&market_event, fees_percent.expect("REASON")),
                            Some(Side::Sell) => instrument_orders.match_asks(&market_event, fees_percent.expect("REASON")),
                            None => continue, // 跳过当前订单处理
                        };

                        drop(orders_write); // 释放写锁
                        self.process_trades(trades).await;
                    }
                } else if let Ok(OrderRole::Taker) = role {
                    // 生成 open_order
                    let open_order = orders_write.build_order_open(order.clone(), OrderRole::Taker).await;

                    // 处理 instrument_orders
                    if let Ok(instrument_orders) = orders_write.ins_orders_mut(&order.instrument) {
                        // 将订单加入到相应的订单簿
                        instrument_orders.add_order_open(open_order.clone());

                        // 获取手续费
                        let fees_percent = self.determine_fees_percent(&order.instrument.kind, &OrderRole::Taker);

                        // 匹配订单并生成交易
                        let trades = match instrument_orders.determine_matching_side(&market_event) {
                            Some(Side::Buy) => instrument_orders.match_bids(&market_event, fees_percent.expect("REASON")),
                            Some(Side::Sell) => instrument_orders.match_asks(&market_event, fees_percent.expect("REASON")),
                            None => continue, // 跳过当前订单处理
                        };

                        drop(orders_write); // 释放写锁
                        self.process_trades(trades).await;
                    }
                }
            }
        }
    }


    fn match_orders_by_side(&self, orders: &mut InstrumentOrders, market_event: &MarketEvent<ClickhousePublicTrade>, fees_percent: f64, side: &Side) -> Vec<ClientTrade>
    {
        match side {
            | Side::Buy => orders.match_bids(market_event, fees_percent),
            | Side::Sell => orders.match_asks(market_event, fees_percent),
        }
    }

    fn determine_fees_percent(&self, kind: &InstrumentKind, role: &OrderRole) -> Option<f64>
    {
        let commission_rates = &self.config.fees_book.get(kind)?;

        match kind {
            | InstrumentKind::Spot | InstrumentKind::Perpetual => match role {
                | OrderRole::Maker => Some(commission_rates.maker_fees),
                | OrderRole::Taker => Some(commission_rates.taker_fees),
            },
            | _ => {
                warn!("Unsupported InstrumentKind: {:?}", kind);
                None
            }
        }
    }

    async fn get_orders_for_instrument(&self, instrument: &Instrument) -> Option<InstrumentOrders>
    {
        let mut orders_lock = self.orders.write().await;
        match orders_lock.ins_orders_mut(instrument) {
            | Ok(orders) => Some(orders.to_owned()),
            | Err(error) => {
                warn!(?error, %instrument, "Failed to match orders for unrecognized Instrument");
                None
            }
        }
    }

    async fn process_trades(&self, trades: Vec<ClientTrade>)
    {
        if !trades.is_empty() {
            let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

            for trade in trades {
                let balance_event = match self.states.lock().await.apply_trade_changes(&trade).await {
                    | Ok(event) => event,
                    | Err(err) => {
                        warn!("Failed to update balance: {:?}", err);
                        continue;
                    }
                };

                if let Err(err) = self.account_event_tx.send(AccountEvent { exchange_timestamp,
                                                                            exchange: ExchangeVariant::SandBox,
                                                                            kind: AccountEventKind::Trade(trade) })
                {
                    // 如果发送交易事件失败，记录警告日志
                    warn!("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Trade: {:?}", err);
                }

                if let Err(err) = self.account_event_tx.send(balance_event) {
                    // 如果发送余额事件失败，记录警告日志
                    warn!("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Balance: {:?}", err);
                }
            }
        }
    }

    /// [PART4]
    /// `cancel_orders` 处理一组订单取消请求，异步执行取消操作，并将结果发送回调用者。
    /// `try_cancel_order_atomic` 尝试以原子操作方式取消一个订单，确保在取消订单后更新账户余额，并发送取消事件。
    /// `cancel_orders_all` 取消所有打开的订单，发送取消结果给调用者，并处理可能的错误情况。
    pub async fn cancel_orders(&mut self, cancel_requests: Vec<Order<RequestCancel>>, response_tx: Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>)
    {
        let cancel_futures = cancel_requests.into_iter().map(|request| {
                                                            let mut this = self.clone();
                                                            async move { this.try_cancel_order_atomic(request).await }
                                                        });

        // 等待所有的取消操作完成
        let cancel_results = join_all(cancel_futures).await;
        response_tx.send(cancel_results).unwrap_or(());
    }

    pub async fn try_cancel_order_atomic(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExecutionError>
    {
        // 获取写锁并查找到对应的Instrument Orders，以便修改订单
        let mut orders_guard = self.orders.write().await;
        let orders = orders_guard.ins_orders_mut(&request.instrument)?;

        // 找到并移除与 Order<RequestCancel> 关联的 Order<Open>
        let removed = match request.side {
            | Side::Buy => {
                // 使用 OrderId 查找 Order<Open>
                let index = orders.bids
                                  .par_iter()
                                  .position_any(|bid| bid.state.id == request.state.id)
                                  .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                orders.bids.remove(index)
            }
            | Side::Sell => {
                // 使用 OrderId 查找 Order<Open>
                let index = orders.asks
                                  .par_iter()
                                  .position_any(|ask| ask.state.id == request.state.id)
                                  .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                orders.asks.remove(index)
            }
        };

        // 在可失败操作成功后，更新客户端余额
        let balance_event = {
            let mut balances_guard = self.states.lock().await;
            balances_guard.apply_cancel_order_changes(&removed)
        };

        // 将 Order<Open> 映射到 Order<Cancelled>
        let cancelled = Order::from(removed);

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送 AccountEvents 给客户端
        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: ExchangeVariant::SandBox,
                                 kind: AccountEventKind::OrdersCancelled(vec![cancelled.clone()]) })
            .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Trade");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: ExchangeVariant::SandBox,
                                 kind: AccountEventKind::Balance(balance_event) })
            .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Balance");

        Ok(cancelled)
    }

    pub async fn cancel_orders_all(&mut self, response_tx: Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>)
    {
        // 获取所有打开的订单
        let orders_to_cancel = {
            let orders_guard = self.orders.read().await;
            orders_guard.fetch_all() // 假设已经有 fetch_all 方法返回所有打开的订单
        };

        // 将所有打开的订单转换为取消请求
        let cancel_requests: Vec<Order<RequestCancel>> = orders_to_cancel.into_iter()
                                                                         .map(|order| Order { state: RequestCancel { id: order.state.id },
                                                                                              instrument: order.instrument,
                                                                                              side: order.side,
                                                                                              kind: order.kind,
                                                                                              cid: order.cid,
                                                                                              exchange: ExchangeVariant::SandBox,
                                                                                              client_ts: 0 })
                                                                         .collect();

        // 调用现有的 cancel_orders 方法
        let (tx, rx) = oneshot::channel();
        self.cancel_orders(cancel_requests, tx).await;

        // 等待取消操作完成并返回结果
        match rx.await {
            | Ok(results) => {
                let cancelled_orders: Vec<_> = results.into_iter().collect::<Result<Vec<_>, _>>().expect("Failed to collect cancel results");
                response_tx.send(Ok(cancelled_orders)).unwrap_or_else(|_| {
                                                          eprintln!("[UniLinkExecution] : Failed to send cancel_orders_all response");
                                                      });
            }
            | Err(_) => {
                response_tx.send(Err(ExecutionError::InternalError("Failed to receive cancel results".to_string())))
                           .unwrap_or_else(|_| {
                               eprintln!("[UniLinkExecution] : Failed to send cancel_orders_all error response");
                           });
            }
        }
    }
}

/// [PART5]
/// `respond` 函数:响应处理。
pub fn respond<Response>(response_tx: Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move {
        response_tx.send(response)
                   .expect("[UniLink_Execution] : SandBoxExchange failed to send oneshot response to execution request")
    });
}
