use crate::common::datafeed::market_event::MarketEvent;
use futures::future::join_all;
use mpsc::UnboundedSender;
use oneshot::Sender;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::warn;

use account_config::AccountConfig;
use account_orders::AccountOrders;
use account_states::AccountState;

use crate::common::order::identification::client_order_id::ClientOrderId;
use crate::{
    common::{
        balance::TokenBalance,
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{machine_id::generate_machine_id, request_order_id::RequestId},
            order_instructions::OrderInstruction,
            states::{cancelled::Cancelled, open::Open, pending::Pending, request_cancel::RequestCancel, request_open::RequestOpen},
            Order, OrderRole,
        },
        position::AccountPositions,
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::{clickhouse_api::datatype::clickhouse_trade_data::MarketTrade, instrument_orders::InstrumentOrders},
    Exchange,
};

pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;
pub mod account_states;

#[derive(Debug)]
pub struct Account
{
    pub machine_id: u64,
    pub exchange_timestamp: AtomicI64,
    pub account_event_tx: UnboundedSender<AccountEvent>, // 帐户事件发送器
    pub config: Arc<AccountConfig>,                      // 帐户配置
    pub states: Arc<Mutex<AccountState>>,                // 帐户余额
    pub orders: Arc<RwLock<AccountOrders>>,              // 帐户订单集合
}

// 手动实现 Clone trait
impl Clone for Account
{
    fn clone(&self) -> Self
    {
        Account { machine_id: self.machine_id,
                  exchange_timestamp: AtomicI64::new(self.exchange_timestamp.load(Ordering::SeqCst)),
                  account_event_tx: self.account_event_tx.clone(),
                  config: Arc::clone(&self.config),
                  states: Arc::clone(&self.states),
                  orders: Arc::clone(&self.orders) }
    }
}
#[derive(Debug)]
pub struct AccountInitiator
{
    account_event_tx: Option<UnboundedSender<AccountEvent>>,
    config: Option<Arc<AccountConfig>>,
    states: Option<Arc<Mutex<AccountState>>>,
    orders: Option<Arc<RwLock<AccountOrders>>>,
}

impl Default for AccountInitiator
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl AccountInitiator
{
    pub fn new() -> Self
    {
        AccountInitiator { account_event_tx: None,
                           config: None,
                           states: None,
                           orders: None }
    }

    pub fn account_event_tx(mut self, value: UnboundedSender<AccountEvent>) -> Self
    {
        self.account_event_tx = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self
    {
        self.config = Some(Arc::new(value));
        self
    }

    pub fn balances(mut self, value: AccountState) -> Self
    {
        self.states = Some(Arc::new(Mutex::new(value)));
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn build(self) -> Result<Account, String>
    {
        Ok(Account { machine_id: generate_machine_id()?,
                     exchange_timestamp: 0.into(), // NOTE initialisation to 0 might be problematic. Consider compatability of online and local modes.
                     account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?, // 检查并获取account_event_tx
                     config: self.config.ok_or("config is required")?, // 检查并获取config
                     states: self.states.ok_or("balances is required")?, // 检查并获取balances
                     orders: self.orders.ok_or("orders are required")? })
    }
}

#[allow(dead_code)]
impl Account
{
    /// [Initiation] `Account` 模块的初始化函数`initiate`
    pub fn initiate() -> AccountInitiator
    {
        AccountInitiator::new()
    }

    /// [PART1]
    /// `update_exchange_timestamp` 是基本的时间戳更新方法，用于更新 `exchange_timestamp` 值。
    /// `fetch_orders_open` 发送当前所有开放的订单给调用者，用于获取所有未完成的订单。
    /// `fetch_balances` 发送当前所有代币的余额信息，用于获取账户中所有代币的账本数据。
    /// `fetch_positions` 发送当前所有代币的持仓信息，用于获取账户中所有代币的仓位数据。
    /// `generate_request_id` 生成请求id。
    /// `update_request_counter`更新请求计数器。NOTE 在产品上线之前应该增加断线重联沿用counter的功能。并考虑是否需要增加定时重置的功能(要考虑雪花算法的特性)。
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
    /// `process_requests_into_pendings` 处理一组订单请求，将其转换为挂起订单，并在成功后更新状态。
    /// `process_pending_order_into_open_atomically` 尝试以原子操作方式打开一个订单，确保在验证和更新账户余额后安全地打开订单。
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

    pub async fn process_pending_order_into_open_atomically(&mut self, current_price: f64, order: Order<Pending>) -> Result<Order<Open>, ExecutionError>
    {
        Self::validate_order_instruction(order.kind)?;

        // 提前声明所需的变量
        let order_role = {
            let orders_guard = self.orders.read().await; // 使用读锁来判断订单角色
            orders_guard.determine_maker_taker(&order, current_price)?
        };

        // 计算所需的可用余额，尽量避免锁操作
        let (token, required_balance) = self.calculate_required_available_balance(&order, current_price).await;

        // 检查余额是否充足，并在锁定后更新订单
        self.states.lock().await.has_sufficient_available_balance(token, required_balance)?;

        let open_order = {
            let mut orders_guard = self.orders.write().await; // 使用写锁来创建订单
            let open_order = orders_guard.build_order_open(order, order_role).await;
            orders_guard.get_ins_orders_mut(&open_order.instrument)?.add_order_open(open_order.clone());
            open_order
        };

        // 应用订单变更并发送事件
        let balance_event = self.states.lock().await.apply_open_order_changes(&open_order, required_balance).await?;
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        self.account_event_tx
            .send(balance_event)
            .expect("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Balance");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                exchange: Exchange::SandBox,
                kind: AccountEventKind::OrdersNew(vec![open_order.clone()]) })
            .expect("[UniLink_Execution] : Client offline - Failed to send AccountEvent::Trade");

        Ok(open_order)
    }

    pub async fn process_requests_into_pendings(&mut self, order_requests: Vec<Order<RequestOpen>>, response_tx: Sender<Vec<Result<Order<Pending>, ExecutionError>>>)
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
    /// `validate_order_instruction` 验证订单的合法性，确保订单类型是受支持的。
    /// `validate_order_request_open` 验证开单请求的合法性，确保订单类型是受支持的。
    /// `match_orders` 处理市场事件，根据市场事件匹配相应的订单并生成交易。
    /// `get_orders_for_instrument` 获取与特定金融工具相关的订单，用于进一步的订单匹配操作。
    /// `match_orders_by_side` 根据订单的买卖方向（Side）匹配订单并生成交易事件。
    /// `determine_fees_percent` 根据金融工具类型和订单方向确定适用的费用百分比。

    pub fn validate_order_instruction(kind: OrderInstruction) -> Result<(), ExecutionError>
    {
        match kind {
            | OrderInstruction::Market
            | OrderInstruction::Limit
            | OrderInstruction::ImmediateOrCancel
            | OrderInstruction::FillOrKill
            | OrderInstruction::PostOnly
            | OrderInstruction::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                             * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    pub fn validate_order_request_open(&self, order: &Order<RequestOpen>) -> Result<(), ExecutionError> {
        // 检查是否提供了有效的 ClientOrderId
        if let Some(cid) = &order.cid.0 {
            if cid.trim().is_empty() {
                return Err(ExecutionError::InvalidRequestOpen(order.clone()));
            }

            // 使用 validate_id_format 验证 ID 格式
            if !ClientOrderId::validate_id_format(cid) {
                return Err(ExecutionError::InvalidRequestOpen(order.clone()));
            }
        }

        // 检查订单类型是否合法
        Account::validate_order_instruction(order.kind)?;

        // 检查价格是否合法（应为正数）
        if order.state.price <= 0.0 {
            return Err(ExecutionError::InvalidRequestOpen(order.clone()));
        }

        // 检查数量是否合法（应为正数）
        if order.state.size <= 0.0 {
            return Err(ExecutionError::InvalidRequestOpen(order.clone()));
        }

        // 检查基础货币和报价货币是否相同
        if order.instrument.base == order.instrument.quote {
            return Err(ExecutionError::InvalidRequestOpen(order.clone()));
        }

        Ok(())
    }

    pub async fn match_orders(&mut self, market_event: MarketEvent<MarketTrade>)
    {
        let current_price = market_event.kind.price;

        // 获取所有的请求 ID
        let request_ids: Vec<RequestId> = self.orders.read().await.pending_registry.iter().map(|entry| *entry.key()).collect();

        // 遍历订单 ID 来处理每个订单
        for request_id in request_ids {
            let order = {
                // 只在获取订单时持有读锁
                let orders_read = self.orders.read().await;
                orders_read.pending_registry.get(&request_id).map(|entry| entry.value().clone())
            };

            if let Some(order) = order {
                let role = match order.kind {
                    | OrderInstruction::Market | OrderInstruction::ImmediateOrCancel | OrderInstruction::FillOrKill => Ok(OrderRole::Taker),
                    | OrderInstruction::Limit | OrderInstruction::GoodTilCancelled => {
                        // 限价订单的判断逻辑可以在读锁下进行
                        self.orders.read().await.determine_limit_order_role(&order, current_price)
                    }
                    | OrderInstruction::PostOnly => {
                        // 这里仅判断是否应该拒绝订单，而不实际执行拒绝操作
                        let should_reject = {
                            match order.side {
                                | Side::Buy => order.state.price < current_price,
                                | Side::Sell => order.state.price > current_price,
                            }
                        };

                        if should_reject {
                            // 获取写锁并拒绝订单
                            self.orders.write().await.reject_post_only_order(&order)
                        }
                        else {
                            Ok(OrderRole::Maker)
                        }
                    }
                };

                if let Ok(role) = role {
                    // 调用 try_open_order_atomic 替代 build_order_open
                    let open_order_result = self.process_pending_order_into_open_atomically(current_price, order.clone()).await;

                    if let Ok(open_order) = open_order_result {
                        if let Ok(mut orders_write) = self.orders.write().await.get_ins_orders_mut(&open_order.instrument) {
                            orders_write.add_order_open(open_order.clone());

                            let fees_percent = self.determine_fees_percent(&order.instrument.kind, &role);
                            let trades = match orders_write.determine_matching_side(&market_event) {
                                | Some(Side::Buy) => orders_write.match_bids(&market_event, fees_percent.expect("REASON")),
                                | Some(Side::Sell) => orders_write.match_asks(&market_event, fees_percent.expect("REASON")),
                                | None => continue, // 跳过当前订单处理
                            };

                            self.process_trades(trades).await;
                        }
                    }
                }
            }
        }
    }

    fn match_orders_by_side(&self, orders: &mut InstrumentOrders, market_event: &MarketEvent<MarketTrade>, fees_percent: f64, side: &Side) -> Vec<ClientTrade>
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
        // 获取 orders_lock 并在 match 之前完成对它的操作
        let orders_result = {
            let orders_lock = self.orders.write().await;
            orders_lock.get_ins_orders_mut(instrument).map(|orders| orders.to_owned())
        };

        match orders_result {
            | Ok(orders) => Some(orders),
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
                                                                            exchange: Exchange::SandBox,
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
                                                            async move { this.process_cancel_request_into_cancelled_atomic(request).await }
                                                        });

        // 等待所有的取消操作完成
        let cancel_results = join_all(cancel_futures).await;
        response_tx.send(cancel_results).unwrap_or(());
    }
    pub async fn process_cancel_request_into_cancelled_atomic(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExecutionError>
    {
        // 首先使用读锁来查找并验证订单是否存在，同时减少写锁的持有时间
        let removed_order = {
            let orders_guard = self.orders.read().await;
            let mut orders = orders_guard.get_ins_orders_mut(&request.instrument)?;

            // 查找并移除订单，这里使用写锁来修改订单集合
            let removed = match request.side {
                Side::Buy => {
                    let index = orders.bids
                        .par_iter()
                        .position_any(|bid| bid.state.id == request.state.id)
                        .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                    orders.bids.remove(index)
                }
                Side::Sell => {
                    let index = orders.asks
                        .par_iter()
                        .position_any(|ask| ask.state.id == request.state.id)
                        .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                    orders.asks.remove(index)
                }
            };
            removed
        };

        // 处理余额更新（不需要持有订单写锁）
        let balance_event = {
            let mut balances_guard = self.states.lock().await;
            balances_guard.apply_cancel_order_changes(&removed_order)
        };

        // 将 Order<Open> 映射到 Order<Cancelled>
        let cancelled = Order::from(removed_order);

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送 AccountEvents 给客户端（不需要持有订单写锁）
        self.account_event_tx
            .send(AccountEvent {
                exchange_timestamp,
                exchange: Exchange::SandBox,
                kind: AccountEventKind::OrdersCancelled(vec![cancelled.clone()]),
            })
            .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Trade");

        self.account_event_tx
            .send(AccountEvent {
                exchange_timestamp,
                exchange: Exchange::SandBox,
                kind: AccountEventKind::Balance(balance_event),
            })
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
                                                                                              exchange: Exchange::SandBox,
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
