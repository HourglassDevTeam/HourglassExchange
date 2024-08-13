use std::{
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
use std::str::FromStr;

use futures::future::join_all;
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
        instrument::kind::InstrumentKind,
        order::{Cancelled, Open, Order, OrderKind, Pending, RequestCancel, RequestOpen},
        Side,
        token::Token,
    },
    error::ExecutionError,
    ExchangeVariant,
    sandbox::account::account_market_feed::AccountDataStreams,
};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhouseTrade;

pub mod account_states;
pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;

#[derive(Debug)]
pub struct Account<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub exchange_timestamp: AtomicI64,
    pub data: Arc<RwLock<AccountDataStreams<Event>>>,         // 帐户数据
    pub account_event_tx: UnboundedSender<AccountEvent>,      // 帐户事件发送器
    pub market_event_tx: UnboundedSender<MarketEvent<Event>>, // 市场事件发送器
    pub config: Arc<AccountConfig>,                           // 帐户配置
    pub states: Arc<RwLock<AccountState<Event>>>,           // 帐户余额
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
    balances: Option<Arc<RwLock<AccountState<Event>>>>,
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
        self.balances = Some(Arc::new(RwLock::new(value)));
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
                     states: self.balances.ok_or("balances is required")?,                         // 检查并获取balances
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

    // 新方法：更新 exchange_timestamp
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
        let balances = self.states.read().await.fetch_all();
        respond(response_tx, Ok(balances));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError>
    {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::PostOnly | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                                          * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    //
    // // NOTE 为给定的 MarketEvent<ClickhouseTrade> 找到所有候选的Open订单 // TO BE CONFIRMED
    // pub async fn candidate_orders_for_trade_event(&self, market_event: MarketEvent<ClickhouseTrade>) -> Vec<Order<Open>>
    // {
    //     // 读取 market_event 中的 instrument 和 side
    //     let instrument_kind = market_event.instrument;
    //     let side = market_event.kind.side;
    //
    //     // 获取读锁以读取订单数据
    //     let orders = self.orders.read().await;
    //
    //     // 从 instrument_orders_map 中查找对应的 InstrumentOrders
    //     if let Some(instrument_orders) = orders.instrument_orders_map.get(&instrument_kind) {
    //         match side.as_str() {
    //             | "Buy" => {
    //                 // 返回所有买单
    //                 instrument_orders.bids.clone()
    //             }
    //             | "Sell" => {
    //                 // 返回所有卖单
    //                 instrument_orders.asks.clone()
    //             }
    //             | _ => {
    //                 // 处理意外的 side 值
    //                 println!("本系统没听说过这种意外的Side: {}", side);
    //                 vec![]
    //             }
    //         }
    //     }
    //     else {
    //         // 没有找到对应的 InstrumentOrders
    //         println!("未找到本则行情数据对应的未成交订单: {:?}", instrument_kind);
    //         vec![]
    //     }
    // }


    pub async fn match_orders(&mut self, market_event: MarketEvent<ClickhouseTrade>) {
        let instrument_kind = market_event.instrument.kind;

        // 将字符串转换为 `Side` 枚举
        let side = match Side::from_str(&market_event.kind.side.to_string()) {
            Ok(side) => side,
            Err(_) => {
                warn!("无效的 Side: {}", market_event.kind.side);
                return; // 如果 `side` 无效，退出函数
            }
        };

        // 获取当前的佣金费率
        let commission_rates = &self.config.current_commission_rate;

        // 根据 InstrumentKind 和 Side 应用不同的费用
        let fees_percent = match instrument_kind {
            InstrumentKind::Spot => match side {
                Side::Buy => commission_rates.spot_maker,
                Side::Sell => commission_rates.spot_taker,
            },
            InstrumentKind::Perpetual => match side {
                Side::Buy => commission_rates.perpetual_open,
                Side::Sell => commission_rates.perpetual_close,
            },
            _ => {
                warn!("不支持的 InstrumentKind: {:?}", instrument_kind);
                return; // 不支持的 InstrumentKind，退出函数
            }
        };

        // 访问适用于当前 Instrument 的订单
        let mut orders_lock = self.orders.write().await; // 将锁绑定到一个变量上
        let orders = match orders_lock.ins_orders_mut(&market_event.instrument) {
            Ok(orders) => orders,
            Err(error) => {
                warn!(?error, %market_event.instrument, ?market_event.kind, "无法匹配未识别的 Instrument 的订单");
                return; // 未找到对应的 InstrumentOrders，退出函数
            }
        };

        // 根据市场事件类型确定匹配的订单并生成交易
        let trades = match side {
            Side::Buy => orders.match_bids(&market_event, fees_percent),
            Side::Sell => orders.match_asks(&market_event, fees_percent),
        };

        // 处理生成的交易记录
        if !trades.is_empty() {
            let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

            for trade in trades {
                // 更新余额和其他相关信息
                let balance_event = match self.states.write().await.update_from_trade(&trade).await {
                    Ok(event) => event,
                    Err(err) => {
                        warn!("更新余额失败: {:?}", err);
                        continue; // 如果更新失败，跳过这个交易
                    }
                };

                // 发送交易事件给客户端
                if let Err(err) = self.account_event_tx.send(AccountEvent {
                    exchange_timestamp,
                    exchange: ExchangeVariant::SandBox,
                    kind: AccountEventKind::Trade(trade),
                }) {
                    warn!("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Trade 失败: {:?}", err);
                }

                // 发送余额更新事件
                if let Err(err) = self.account_event_tx.send(balance_event) {
                    warn!("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Balance 失败: {:?}", err);
                }
            }
        }
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

    // NOTE 注意size的单位
    pub async fn calculate_required_available_balance<'a>(&'a self, order: &'a Order<Pending>, current_price: f64) -> (&Token, f64)
    {
        match order.instrument.kind {
            | InstrumentKind::Spot => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size),
                | Side::Sell => (&order.instrument.base, order.state.size),
            },
            | InstrumentKind::Perpetual => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size * self.config.leverage_book.get(&order.instrument).unwrap()),
                | Side::Sell => (&order.instrument.base, order.state.size * self.config.leverage_book.get(&order.instrument).unwrap()),
            },
            | InstrumentKind::Future => match order.side {
                | Side::Buy => (&order.instrument.quote, current_price * order.state.size * self.config.leverage_book.get(&order.instrument).unwrap()),
                | Side::Sell => (&order.instrument.base, order.state.size * self.config.leverage_book.get(&order.instrument).unwrap()),
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
        self.states.read().await.has_sufficient_available_balance(token, required_balance)?;

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
        let balance_event = self.states.write().await.update_from_open(&open_order, required_balance).await.unwrap();

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送账户事件给客户端
        self.account_event_tx
            .send(balance_event)
            .expect("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Balance 失败");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: ExchangeVariant::SandBox,
                                 kind: AccountEventKind::OrdersNew(vec![open_order.clone()]) })
            .expect("[UniLink_Execution] : 客户端离线 - 发送 AccountEvent::Trade 失败");

        // 返回已打开的订单
        Ok(open_order)
    }

    pub async fn cancel_orders(&mut self, cancel_requests: Vec<Order<RequestCancel>>, response_tx: Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>)
    {
        let cancel_futures = cancel_requests.into_iter().map(|request| {
                                                            let mut this = self.clone();
                                                            async move { this.try_cancel_order_atomic(request).await }
                                                        });

        // 等待所有的取消操作完成
        let cancel_results = join_all(cancel_futures).await;
        response_tx.send(cancel_results).unwrap_or_else(|_| {
                                            // 如果发送失败，处理错误
                                        });
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
                                  .iter()
                                  .position(|bid| bid.state.id == request.state.id)
                                  .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                orders.bids.remove(index)
            }
            | Side::Sell => {
                // 使用 OrderId 查找 Order<Open>
                let index = orders.asks
                                  .iter()
                                  .position(|ask| ask.state.id == request.state.id)
                                  .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                orders.asks.remove(index)
            }
        };

        // 在可失败操作成功后，更新客户端余额
        let balance_event = {
            let mut balances_guard = self.states.write().await;
            balances_guard.update_from_cancel(&removed)
        };

        // 将 Order<Open> 映射到 Order<Cancelled>
        let cancelled = Order::from(removed);

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送 AccountEvents 给客户端
        self.account_event_tx
            .send(AccountEvent { exchange_timestamp: exchange_timestamp.into(),
                                 exchange: ExchangeVariant::SandBox,
                                 kind: AccountEventKind::OrdersCancelled(vec![cancelled.clone()]) })
            .expect("[TideBroker] : Client is offline - failed to send AccountEvent::Trade");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp: exchange_timestamp.into(),
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
                                                                                              cid: order.cid.clone(),
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

pub fn respond<Response>(response_tx: Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move {
        response_tx.send(response)
                   .expect("[UniLink_Execution] : SandBoxExchange failed to send oneshot response to execution request")
    });
}
