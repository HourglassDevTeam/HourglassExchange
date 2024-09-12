use crate::{
    common::{
        account_positions::{
            AccountPositions, PositionDirectionMode,
        },
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, machine_id::generate_machine_id},
            order_instructions::OrderInstruction,
            states::{cancelled::Cancelled, open::Open, request_cancel::RequestCancel},
            Order, OrderRole,
        },
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExchangeError,
    sandbox::{
        account::{account_config::SandboxMode, traits::position_handler::PositionHandler},
        clickhouse_api::datatype::{clickhouse_trade_data::MarketTrade, single_level_order_book::SingleLevelOrderBook},
    },
    Exchange,
};
use account_config::AccountConfig;
use account_orders::AccountOrders;
use chrono::Utc;
use dashmap::{
    mapref::one::{RefMut as DashMapRefMut},
    DashMap,
};
use futures::future::join_all;
use mpsc::UnboundedSender;
use oneshot::Sender;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator};
use std::collections::HashMap;
/// FIXME respond function is not used in some of the functions.
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::warn;
use uuid::Uuid;
use crate::common::account_positions::exited_positions::AccountExitedPositions;
use crate::common::order::states::request_open::RequestOpen;
 use crate::sandbox::account::traits::balance_handler::BalanceHandler;
pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;
pub mod traits;

#[derive(Debug)]
pub struct Account
// where Vault: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
//       Statistic: Initialiser + PositionSummariser,
{
    pub current_session: Uuid,
    pub machine_id: u64,                                                                // 机器ID
    pub exchange_timestamp: AtomicI64,                                                  // 交易所时间戳
    pub account_event_tx: UnboundedSender<AccountEvent>,                                // 帐户事件发送器
    pub config: AccountConfig,                                                          // 帐户配置
    pub orders: Arc<RwLock<AccountOrders>>,                                             // 帐户订单集合
    pub single_level_order_book: Arc<Mutex<HashMap<Instrument, SingleLevelOrderBook>>>, // 将最新的价格存到订单簿里面去
    pub balances: DashMap<Token, Balance>,                                              // 每个币种的细分余额
    pub positions: AccountPositions,                                                    // 帐户持仓
    pub exited_positions: AccountExitedPositions,                                       // pub vault: Vault,
}

// 手动实现 Clone trait
impl Clone for Account
{
    fn clone(&self) -> Self
    {
        Account { current_session: Uuid::new_v4(),
                  machine_id: self.machine_id,
                  exchange_timestamp: AtomicI64::new(self.exchange_timestamp.load(Ordering::SeqCst)),
                  account_event_tx: self.account_event_tx.clone(),
                  config: self.config.clone(),
                  orders: Arc::clone(&self.orders),
                  single_level_order_book: Arc::new(Mutex::new(HashMap::new())),
                  balances: self.balances.clone(),
                  positions: self.positions.clone(),
                  exited_positions: self.exited_positions.clone() }
    }
}
#[derive(Debug)]
pub struct AccountInitiator
{
    account_event_tx: Option<UnboundedSender<AccountEvent>>,
    config: Option<AccountConfig>,
    orders: Option<Arc<RwLock<AccountOrders>>>,
    balances: Option<DashMap<Token, Balance>>,
    positions: Option<AccountPositions>,
    closed_positions: Option<AccountExitedPositions>,
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
                           orders: None,
                           balances: None,
                           positions: None,
                           closed_positions: None }
    }

    pub fn account_event_tx(mut self, value: UnboundedSender<AccountEvent>) -> Self
    {
        self.account_event_tx = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self
    {
        self.config = Some(value);
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn balances(mut self, value: DashMap<Token, Balance>) -> Self
    {
        self.balances = Some(value);
        self
    }

    pub fn positions(mut self, value: AccountPositions) -> Self
    {
        self.positions = Some(value);
        self
    }

    pub fn build(self) -> Result<Account, String>
    {
        Ok(Account { current_session: Uuid::new_v4(),
                     machine_id: generate_machine_id()?,
                     exchange_timestamp: 0.into(),
                     account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?,
                     config: self.config.ok_or("config is required")?,
                     orders: self.orders.ok_or("orders are required")?,
                     balances: self.balances.ok_or("balances are required")?,
                     positions: self.positions.ok_or("positions are required")?,
                     single_level_order_book: Arc::new(Mutex::new(HashMap::new())),
                     exited_positions: self.closed_positions.ok_or("closed_positions sink are required")? })
    }
}

impl Account
{
    /// [PART 1] - [账户初始化与配置]
    pub fn initiate() -> AccountInitiator
    {
        AccountInitiator::new()
    }

    /// 初始化账户中要使用的币种，初始余额设为 0。
    ///
    /// # 参数
    ///
    /// * `tokens` - 一个包含要初始化的 `Token` 名称的 `Vec<String>`。
    pub fn initialize_tokens(&mut self, tokens: Vec<String>) -> Result<(), ExchangeError>
    {
        for token_str in tokens {
            let token = Token(token_str);
            self.balances.entry(token.clone()).or_insert_with(|| Balance { time: Utc::now(),
                                                                           current_price: Some(1.0), // 假设初始价格为 1.0，具体根据实际情况调整
                                                                           total: 0.0,
                                                                           available: 0.0 });
        }
        Ok(())
    }

    /// 为指定的 `Token` 充值指定数量的稳定币。
    ///
    /// 如果该 `Token` 已经存在于 `balances` 中，则更新其余额；如果不存在，则创建一个新的 `Balance` 条目。
    ///
    /// # 参数
    ///
    /// * `token` - 需要充值的 `Token`。
    /// * `amount` - 充值的数额。
    ///
    /// # 返回值
    ///
    /// 返回更新后的 `TokenBalance`。
    fn deposit_coin(&mut self, token: Token, amount: f64) -> Result<TokenBalance, ExchangeError>
    {
        let mut balance = self.balances.entry(token.clone()).or_insert_with(|| {
                                                                Balance { time: Utc::now(),
                                                                          current_price: Some(1.0), // 假设稳定币价格为1.0
                                                                          total: 0.0,
                                                                          available: 0.0 }
                                                            });

        balance.total += amount;
        balance.available += amount;

        Ok(TokenBalance::new(token, *balance))
    }

    /// 为多个指定的 `Token` 充值指定数量的稳定币。
    ///
    /// 如果这些 `Token` 中有已经存在于 `balances` 中的，则更新其余额；如果不存在，则创建新的 `Balance` 条目。
    ///
    /// # 参数
    ///
    /// * `deposits` - 包含多个 `Token` 和对应充值金额的元组的集合。
    ///
    /// # 返回值
    ///
    /// 返回更新后的 `TokenBalance` 列表。
    fn deposit_multiple_coins(&mut self, deposits: Vec<(Token, f64)>) -> Result<Vec<TokenBalance>, ExchangeError>
    {
        let mut updated_balances = Vec::new();

        for (token, amount) in deposits {
            let balance = self.deposit_coin(token, amount)?;
            updated_balances.push(balance);
        }

        Ok(updated_balances)
    }

    /// 为账户充值 `u本位` 稳定币（USDT）。 并返回充值结果。
    pub async fn deposit_multiple_coins_and_respond(&mut self, deposits: Vec<(Token, f64)>, response_tx: Sender<Result<Vec<TokenBalance>, ExchangeError>>)
    {
        let result = self.deposit_multiple_coins(deposits);
        respond(response_tx, result);
    }

    /// 为账户充值 `u本位` 稳定币（USDT）。
    ///
    /// # 参数
    ///
    /// * `amount` - 充值的数额。
    ///
    /// # 返回值
    ///
    /// 返回更新后的 `TokenBalance`。
    pub fn deposit_usdt(&mut self, amount: f64) -> Result<TokenBalance, ExchangeError>
    {
        let usdt_token = Token("USDT".into());
        self.deposit_coin(usdt_token, amount)
    }

    /// NOTE : BETA功能，待测试。
    /// 为账户充值 `b本位` 稳定币（BTC）。
    ///
    /// # 参数
    /// * `amount` - 充值的数额。
    ///
    /// # 返回值
    ///
    /// 返回更新后的 `TokenBalance`。
    pub fn deposit_bitcoin(&mut self, amount: f64) -> Result<TokenBalance, ExchangeError>
    {
        let btc_token = Token("BTC".into());
        self.deposit_coin(btc_token, amount)
    }

    /// NOTE : BETA功能，待测试。
    /// 用 `u本位` (USDT) 买 `b本位` (BTC)。
    ///
    /// # 参数
    ///
    /// * `usdt_amount` - 用于购买的 USDT 数额。
    /// * `btc_price` - 当前 BTC 的价格（USDT/BTC）。
    ///
    /// # 返回值
    ///
    /// 返回更新后的 `TokenBalance` 列表，其中包含更新后的 BTC 和 USDT 余额。
    pub fn topup_bitcoin_with_usdt(&mut self, usdt_amount: f64, btc_price: f64) -> Result<Vec<TokenBalance>, ExchangeError>
    {
        let usdt_token = Token("USDT".into());
        let btc_token = Token("BTC".into());

        // 检查是否有足够的 USDT 余额
        self.has_sufficient_available_balance(&usdt_token, usdt_amount)?;

        // 计算购买的 BTC 数量
        let btc_amount = usdt_amount / btc_price;

        // 更新 USDT 余额
        let usdt_delta = BalanceDelta { total: -usdt_amount,
                                        available: -usdt_amount };
        let updated_usdt_balance = self.apply_balance_delta(&usdt_token, usdt_delta);

        // 更新 BTC 余额
        let btc_delta = BalanceDelta { total: btc_amount,
                                       available: btc_amount };
        let updated_btc_balance = self.apply_balance_delta(&btc_token, btc_delta);

        Ok(vec![TokenBalance::new(usdt_token, updated_usdt_balance), TokenBalance::new(btc_token, updated_btc_balance),])
    }

    /// [PART 2] - [订单管理].
    pub async fn fetch_orders_open_and_respond(&self, response_tx: Sender<Result<Vec<Order<Open>>, ExchangeError>>)
    {
        let orders = self.orders.read().await.fetch_all();
        respond(response_tx, Ok(orders));
    }

    /// 处理多个开仓订单请求，并执行相应操作。
    ///
    /// 对于每个开仓请求，该函数根据配置的 `PositionDirectionMode` 来判断是否允许方向冲突。如果是 `NetMode`，则会检查订单方向与当前持仓的方向是否冲突。
    /// 如果订单标记为 `reduce only`，则不会进行方向冲突检查，但仍需判断订单方向与现有持仓方向是否一致。如果 `reduce only` 订单的方向与现有持仓方向相同，将拒绝该订单。
    ///
    /// # 参数
    ///
    /// * `open_requests` - 一个包含多个 `Order<RequestOpen>` 的向量，表示待处理的开仓请求。
    /// * `response_tx` - 一个 `oneshot::Sender`，用于异步发送订单处理结果。
    ///
    /// # 逻辑
    ///
    /// 1. 首先检查订单的 `reduce only` 状态：
    ///    - 如果是 `reduce only`，则跳过方向冲突检查，但如果订单方向与当前持仓方向相同，则拒绝该订单。
    /// 2. 如果是 `NetMode` 且订单不是 `reduce only`，则调用 `check_position_direction_conflict` 检查当前持仓方向是否与订单冲突。
    /// 3. 计算订单的当前价格，并尝试原子性开仓操作。
    /// 4. 将每个订单的处理结果发送到 `response_tx`。
    ///
    /// # 错误处理
    ///
    /// - 如果 `reduce only` 订单的方向与现有持仓方向相同，则拒绝该订单，并继续处理下一个订单。
    /// - 如果在 `NetMode` 下存在方向冲突，则跳过该订单并继续处理下一个订单。
    pub async fn open_orders(&mut self, open_requests: Vec<Order<RequestOpen>>, response_tx: Sender<Vec<Result<Order<Open>, ExchangeError>>>) -> Result<(), ExchangeError>
    {
        let mut open_results = Vec::new();

        // 获取当前的 position_direction_mode 并提前判断是否需要进行方向冲突检查
        let is_netmode = self.config.global_position_direction_mode == PositionDirectionMode::Net;

        for request in open_requests {
            // 如果是 NetMode，检查方向冲突
            if is_netmode {
                if let Err(err) = self.check_direction_conflict(&request).await {
                    open_results.push(Err(err));
                    continue; // 跳过这个订单
                }
            }

            // 处理订单请求，根据模式（回测或实时）选择处理方式
            let processed_request = match self.config.execution_mode {
                | SandboxMode::Backtest => self.orders.write().await.process_backtest_requestopen_with_a_simulated_latency(request).await,
                | _ => request, // 实时模式下直接使用原始请求
            };

            // 尝试开仓，处理结果
            let open_result = self.atomic_open(processed_request).await;
            open_results.push(open_result);
        }

        // 发送处理结果
        if let Err(e) = response_tx.send(open_results) {
            return Err(ExchangeError::SandBox(format!("Failed to send open order results: {:?}", e)));
        }

        Ok(())
    }

    // 辅助函数，用于检查仓位方向冲突
    async fn check_direction_conflict(&self, request: &Order<RequestOpen>) -> Result<(), ExchangeError>
    {
        if request.state.reduce_only {
            // 获取当前仓位
            let (long_pos, short_pos) = self.get_position_both_ways(&request.instrument).await?;

            // 检查 reduce_only 订单是否有相同方向的仓位
            match request.side {
                | Side::Buy => {
                    if long_pos.is_some() {
                        return Err(ExchangeError::InvalidDirection);
                    }
                }
                | Side::Sell => {
                    if short_pos.is_some() {
                        return Err(ExchangeError::InvalidDirection);
                    }
                }
            }
        }
        else {
            // 检查非 reduce_only 订单的方向冲突
            self.check_position_direction_conflict(&request.instrument, request.side, request.state.reduce_only).await?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    // 辅助函数，用于获取当前市场价格 // NOTE 要处理不同的InstrumentKind,现在是不对的
    async fn get_current_price(&self, order: &Order<RequestOpen>) -> Result<f64, ExchangeError>
    {
        match order.instrument.kind {
            | InstrumentKind::Spot => match order.side {
                | Side::Buy => {
                    let token = &order.instrument.base;
                    let balance = self.get_balance(token)?;
                    Ok(balance.current_price.expect("Price for Spot Buy is missing"))
                }
                | Side::Sell => {
                    let token = &order.instrument.quote;
                    let balance = self.get_balance(token)?;
                    Ok(balance.current_price.expect("Price for Spot Sell is missing"))
                }
            },
            // 对于其他种类的 instrument，暂时未处理
            | _ => {
                todo!("Handling for other InstrumentKind is not yet implemented.");
            }
        }
    }

    pub async fn atomic_open(&mut self, order: Order<RequestOpen>) -> Result<Order<Open>, ExchangeError>
    {
        // 验证订单的基本合法性
        Self::validate_order_instruction(order.instruction)?;

        println!("[attempt_atomic_open] : successfully validated order instruction");

        // 将锁的作用域限制在这个块内， 通过和订单簿比较价格来判断是潜在的 Taker 还是 Maker。
        let order_role = {
            let mut order_books_lock = self.single_level_order_book.lock().await;
            let order_book = order_books_lock.get_mut(&order.instrument).unwrap(); // 引用的生命周期延长

            let orders_guard = self.orders.read().await;
            // 将订单簿传递给 determine_maker_taker
            orders_guard.determine_maker_taker(&order, order_book)?
        };

        // 锁已经在此处释放，后续操作可以安全地借用 `self`
        let (token, required_balance) = self.required_available_balance(&order).await?;
        println!("[attempt_atomic_open] required balance is quoted in {}: {}", token, required_balance);
        self.has_sufficient_available_balance(token, required_balance)?;

        let open_order = {
            let mut orders_guard = self.orders.write().await;
            let open_order = orders_guard.build_order_open(order, order_role).await;
            orders_guard.get_ins_orders_mut(&open_order.instrument)?.add_order_open(open_order.clone());
            open_order
        };

        let balance_event = self.apply_open_order_changes(&open_order, required_balance).await?;
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 使用 `send_account_event` 发送余额和订单事件
        self.send_account_event(balance_event)?;
        let order_event = AccountEvent { exchange_timestamp,
                                         exchange: Exchange::SandBox,
                                         kind: AccountEventKind::OrdersOpen(vec![open_order.clone()]) };

        self.send_account_event(order_event)?;
        Ok(open_order)
    }

    /// NOTE 现货等一些金融工具是否不支持这些订单指令？？？？
    pub fn validate_order_instruction(kind: OrderInstruction) -> Result<(), ExchangeError>
    {
        match kind {
            | OrderInstruction::Market
            | OrderInstruction::Limit
            | OrderInstruction::ImmediateOrCancel
            | OrderInstruction::FillOrKill
            | OrderInstruction::PostOnly
            | OrderInstruction::GoodTilCancelled
            | OrderInstruction::Cancel => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                   * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    pub fn validate_order_request_open(order: &Order<RequestOpen>) -> Result<(), ExchangeError>
    {
        // 检查是否提供了有效的 ClientOrderId
        if let Some(cid) = &order.cid {
            if cid.0.trim().is_empty() {
                return Err(ExchangeError::InvalidRequestOpen("ClientOrderId is empty".into()));
            }

            // 使用 validate_id_format 验证 CID 格式
            if !ClientOrderId::validate_id_format(&cid.0) {
                return Err(ExchangeError::InvalidRequestOpen(format!("Invalid ClientOrderId format: {}", cid.0)));
            }
        }
        // 检查订单类型是否合法
        Account::validate_order_instruction(order.instruction)?;

        // 检查价格是否合法（应为正数）
        if order.state.price <= 0.0 {
            return Err(ExchangeError::InvalidRequestOpen(format!("Invalid price: {}", order.state.price)));
        }

        // 检查数量是否合法（应为正数）
        if order.state.size <= 0.0 {
            return Err(ExchangeError::InvalidRequestOpen(format!("Invalid size: {}", order.state.size)));
        }

        // 检查基础货币和报价货币是否相同
        if order.instrument.base == order.instrument.quote {
            return Err(ExchangeError::InvalidRequestOpen(format!("Base and Quote tokens must be different: {}", order.instrument.base)));
        }

        Ok(())
    }

    pub fn validate_order_request_cancel(order: &Order<RequestCancel>) -> Result<(), ExchangeError>
    {
        // 检查是否提供了有效的 OrderId 或 ClientOrderId
        if order.state.id.is_none() && order.cid.is_none() {
            return Err(ExchangeError::InvalidRequestCancel("Both OrderId and ClientOrderId are missing".into()));
        }

        // 如果提供了 OrderId，则检查其是否有效
        if let Some(id) = &order.state.id {
            if id.value() == 0 {
                return Err(ExchangeError::InvalidRequestCancel("OrderId is missing or invalid".into()));
            }
        }

        // 如果提供了 ClientOrderId，则验证其格式是否有效
        if let Some(cid) = &order.cid {
            // 使用 `validate_id_format` 方法验证 ClientOrderId 格式
            if !ClientOrderId::validate_id_format(&cid.0) {
                return Err(ExchangeError::InvalidRequestCancel(format!("Invalid ClientOrderId format: {}", cid.0)));
            }
        }

        // 检查基础货币和报价货币是否相同
        if order.instrument.base == order.instrument.quote {
            return Err(ExchangeError::InvalidRequestCancel("Base and Quote tokens must be different".into()));
        }

        Ok(())
    }

    pub async fn cancel_orders(&mut self, cancel_requests: Vec<Order<RequestCancel>>, response_tx: Sender<Vec<Result<Order<Cancelled>, ExchangeError>>>)
    {
        let cancel_futures = cancel_requests.into_iter().map(|request| {
                                                            let mut this = self.clone();
                                                            async move { this.atomic_cancel(request).await }
                                                        });

        // 等待所有的取消操作完成
        let cancel_results = join_all(cancel_futures).await;
        response_tx.send(cancel_results).unwrap_or(());
    }

    /// 原子性取消订单并更新相关的账户状态。
    ///
    /// 该方法尝试以原子操作的方式取消一个指定的订单，确保在取消订单后更新账户余额，并发送取消事件和余额更新事件。
    ///
    /// # 参数
    ///
    /// * `request` - 一个 `Order<RequestCancel>` 实例，表示客户端发送的订单取消请求。
    ///
    /// # 逻辑
    ///
    /// 1. 验证取消请求的合法性（例如是否提供了有效的 `OrderId` 或 `ClientOrderId`）。
    /// 2. 使用读锁查找订单是否存在，确保最小化锁的持有时间。
    /// 3. 根据订单方向（买或卖），查找并移除订单。
    /// 4. 在移除订单后，更新相关余额并生成余额事件。
    /// 5. 将 `Order<Open>` 转换为 `Order<Cancelled>`，并生成取消事件。
    /// 6. 发送账户事件，包括取消订单事件和余额更新事件。
    ///
    /// # 返回值
    ///
    /// * 成功取消订单后，返回 `Order<Cancelled>`。
    /// * 如果订单不存在，返回 `ExchangeError::OrderNotFound` 错误。
    ///
    /// # 错误处理
    ///
    /// * 如果订单验证失败或订单不存在，返回相应的 `ExchangeError`。
    /// * 如果事件发送失败（如客户端离线），记录警告日志。
    ///
    /// # 锁机制
    ///
    /// * 在查找和移除订单时，使用读锁以减少写锁的持有时间，避免阻塞其他操作。
    pub async fn atomic_cancel(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExchangeError>
    {
        // 首先验证取消请求的合法性
        Self::validate_order_request_cancel(&request)?;

        // 使用读锁来获取订单，减少锁的持有时间
        let removed_order = {
            let orders_guard = self.orders.read().await;
            let mut orders = orders_guard.get_ins_orders_mut(&request.instrument)?;

            // 根据订单方向（买/卖）处理相应的订单集
            match request.side {
                | Side::Buy => {
                    let index = Self::find_matching_order(&orders.bids, &request)?;
                    orders.bids.remove(index)
                }
                | Side::Sell => {
                    let index = Self::find_matching_order(&orders.asks, &request)?;
                    orders.asks.remove(index)
                }
            }
        };

        // 处理取消订单后的余额更新
        let balance_event = match self.apply_cancel_order_changes(&removed_order) {
            | Ok(event) => event,
            | Err(e) => return Err(e), // 如果更新余额时发生错误，返回错误
        };

        // 将订单从 `Order<Open>` 转换为 `Order<Cancelled>`
        let cancelled_order = Order::from(removed_order);

        // 获取当前的交易所时间戳
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送订单取消事件
        let orders_cancelled_event = AccountEvent { exchange_timestamp,
                                                    exchange: Exchange::SandBox,
                                                    kind: AccountEventKind::OrdersCancelled(vec![cancelled_order.clone()]) };

        // 发送账户事件
        self.send_account_event(orders_cancelled_event)?;
        self.send_account_event(balance_event)?;

        Ok(cancelled_order)
    }

    pub async fn cancel_orders_all(&mut self, response_tx: Sender<Result<Vec<Order<Cancelled>>, ExchangeError>>)
    {
        // 获取所有打开的订单
        let orders_to_cancel = {
            let orders_guard = self.orders.read().await;
            orders_guard.fetch_all() // 假设已经有 fetch_all 方法返回所有打开的订单
        };

        // 将所有打开的订单转换为取消请求
        let cancel_requests: Vec<Order<RequestCancel>> = orders_to_cancel.into_iter()
                                                                         .map(|order| Order { state: RequestCancel { id: Some(order.state.id) },
                                                                                              instrument: order.instrument,
                                                                                              side: order.side,
                                                                                              instruction: order.instruction,
                                                                                              cid: order.cid,
                                                                                              exchange: Exchange::SandBox,
                                                                                              timestamp: self.exchange_timestamp.load(Ordering::SeqCst) })
                                                                         .collect();

        // 调用现有的 cancel_orders 方法
        let (tx, rx) = oneshot::channel();
        self.cancel_orders(cancel_requests, tx).await;

        // 等待取消操作完成并返回结果
        match rx.await {
            | Ok(results) => {
                let cancelled_orders: Vec<_> = results.into_iter().collect::<Result<Vec<_>, _>>().expect("Failed to collect cancel results");
                response_tx.send(Ok(cancelled_orders)).unwrap_or_else(|_| {
                                                          eprintln!("[UniLinkEx] : Failed to send cancel_orders_all response");
                                                      });
            }
            | Err(_) => {
                response_tx.send(Err(ExchangeError::InternalError("Failed to receive cancel results".to_string()))).unwrap_or_else(|_| {
                                                                                                                       eprintln!("[UniLinkEx] : Failed to send cancel_orders_all error response");
                                                                                                                   });
            }
        }
    }

    /// [PART 5] - [交易处理]

    /// 更新交易所时间辍
    fn update_exchange_ts(&self, timestamp: i64)
    {
        let adjusted_timestamp = match self.config.execution_mode {
            | SandboxMode::Backtest => timestamp,                                                            // 在回测模式下使用传入的时间戳
            | SandboxMode::Online => SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64, // 在实时模式下使用当前时间
        };
        self.exchange_timestamp.store(adjusted_timestamp, Ordering::SeqCst);
    }

    /// 创建或更新一个单级别的订单簿（SingleLevelOrderBook），基于传入的市场交易（MarketTrade）。
    ///
    /// # 参数
    /// - `trade`: 引用 `MarketTrade` 类型的交易信息，用于从中提取 `instrument` 并更新对应的订单簿。
    ///
    /// # 实现步骤
    /// 1. 从 `trade` 中解析出 `instrument`，如果解析失败，程序会 panic（`unwrap`）。
    /// 2. 获取 `single_level_order_book` 的互斥锁以安全地访问共享资源。
    /// 3. 使用 `instrument` 作为键，查找对应的单级别订单簿，如果没有则创建一个新的订单簿。
    /// 4. 使用 `trade` 更新该 `instrument` 对应的订单簿。
    ///
    /// # 备注
    /// - `SingleLevelOrderBook::from(trade)` 是一个基于 `trade` 初始化订单簿的工厂方法。
    /// - 该函数异步锁定了 `single_level_order_book`，并且通过 `.await` 实现对共享数据的安全访问。
    async fn create_or_single_level_orderbook_from_market_trade(&mut self, trade: &MarketTrade)
    {
        let instrument = trade.parse_instrument().unwrap();
        let mut orderbook = self.single_level_order_book.lock().await;

        orderbook.entry(instrument)
                 .or_insert_with(|| SingleLevelOrderBook::from(trade)) // 传递引用 &trade
                 .update_from_trade(&trade);
    }

    /// 处理交易数据的方法
    pub async fn handle_trade_data(&mut self, trade: &MarketTrade) -> Result<(), ExchangeError>
    {
        // 更新时间戳
        self.update_exchange_ts(trade.timestamp);
        // 更新单层OrderBook，注意 这个做法仅仅适用于回测。
        self.create_or_single_level_orderbook_from_market_trade(trade).await;
        // 用交易所记录的用户第开放订单去匹配 market_rade 以实现模拟的目的
        self.match_orders(&trade).await?;
        Ok(())
    }

    /// 处理市场交易事件并尝试匹配订单。
    ///
    /// 该函数根据市场交易事件尝试匹配账户中的订单，并生成相应的交易。它会根据市场事件的方向（买或卖）
    /// 查找最佳报价，并使用预先计算的 `OrderRole` 来确定订单的费用比例。匹配成功的订单会生成相应的交易记录。
    ///
    /// # 参数
    ///
    /// - `market_trade`: 一个 [`MarketTrade`] 实例，表示来自市场的交易事件。
    ///
    /// # 返回值
    ///
    /// 返回一个包含所有匹配到的 [`ClientTrade`] 实例的向量。
    ///
    /// # 逻辑
    ///
    /// 1. 从市场交易事件中解析出基础货币和报价货币，并确定金融工具种类。
    /// 2. 查找与该金融工具相关的挂单（`InstrumentOrders`）。
    /// 3. 根据市场事件的方向（买或卖）尝试匹配相应的挂单（买单匹配卖单，卖单匹配买单）。
    /// 4. 使用订单的 `OrderRole` 来计算手续费，并生成交易记录。
    /// 5. 处理并返回生成的交易记录。
    ///
    /// # 注意
    /// 该函数假设市场交易事件的符号格式为 `base_quote`，并从中解析出基础货币和报价货币。
    /// 如果找不到与市场事件相关的挂单，函数会记录警告并返回一个空的交易向量。
    pub async fn match_orders(&mut self, market_trade: &MarketTrade) -> Result<Vec<ClientTrade>, ExchangeError>
    {
        // println!("[match_orders]: market_trade: {:?}", market_trade);
        let mut trades = Vec::new();

        // 从市场交易事件的符号中解析基础货币和报价货币，并确定金融工具种类
        let base = Token::from(market_trade.parse_base().ok_or_else(|| ExchangeError::SandBox("Unknown base.".to_string()))?);
        let quote = Token::from(market_trade.parse_quote().ok_or_else(|| ExchangeError::SandBox("Unknown quote.".to_string()))?);
        let kind = market_trade.parse_kind();
        // println!("[match_orders]: kind is {}", kind);
        let instrument = Instrument { base, quote, kind };
        // println!("[match_orders]: instrument is {}", instrument);

        // 查找与指定金融工具相关的挂单
        if let Ok(mut instrument_orders) = self.orders.read().await.get_ins_orders_mut(&instrument) {
            // 确定市场事件匹配的挂单方向（买或卖）
            if let Some(matching_side) = instrument_orders.determine_matching_side(market_trade) {
                // println!("[match_orders]: matching side is {}, will look up in corresponding open orders", matching_side);
                match matching_side {
                    | Side::Buy => {
                        // 从最佳买单中提取 `OrderRole` 以获取正确的手续费比例
                        if let Some(best_bid) = instrument_orders.bids.last() {
                            let order_role = best_bid.state.order_role;
                            // println!("[match_orders]: order_role: {:?}", order_role);
                            let fees_percent = self.fees_percent(&kind, order_role).await.map_err(|_| ExchangeError::SandBox("Missing fees.".to_string()))?;

                            // 使用计算出的手续费比例匹配买单
                            trades.append(&mut instrument_orders.match_bids(market_trade, fees_percent));
                        }
                    }
                    | Side::Sell => {
                        // 从最佳卖单中提取 `OrderRole` 以获取正确的手续费比例
                        if let Some(best_ask) = instrument_orders.asks.last() {
                            let order_role = best_ask.state.order_role;
                            // println!("[match_orders]: order_role: {:?}", order_role);
                            let fees_percent = self.fees_percent(&kind, order_role).await.map_err(|_| ExchangeError::SandBox("Missing fees.".to_string()))?;

                            // 使用计算出的手续费比例匹配卖单
                            trades.append(&mut instrument_orders.match_asks(market_trade, fees_percent));
                        }
                    }
                }
            }
        }
        else {
            // 记录日志并继续，不返回错误
            warn!("未找到与市场事件相关的挂单，跳过处理。");
        }

        // println!("[match_orders]: generated client trades are: {:?}", trades);
        self.process_trades(trades.clone()).await;

        Ok(trades)
    }

    /// 根据金融工具类型和订单角色返回相应的手续费百分比。 NOTE 需要扩展并支持现货和期货。
    ///
    /// # 参数
    ///
    /// * `kind` - 表示金融工具的种类，如 `Spot` 或 `Perpetual`。
    /// * `role` - 表示订单的角色，如 `Maker` 或 `Taker`。
    ///
    /// # 返回值
    ///
    /// * `Option<f64>` - 返回适用于指定金融工具类型和订单角色的手续费百分比。
    ///     - `Some(f64)` - 如果手续费配置存在，则返回对应的 `maker_fees` 或 `taker_fees`。
    ///     - `None` - 如果手续费配置不存在或金融工具类型不受支持，返回 `None`。
    ///
    /// # 注意事项
    ///
    /// * 目前只支持 `Spot` 和 `Perpetual` 类型的金融工具。
    /// * 如果传入的 `InstrumentKind` 不受支持，函数会记录一个警告并返回 `None`。
    pub(crate) async fn fees_percent(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExchangeError>
    {
        // 直接访问 account 的 config 字段
        let commission_rates = self.config
                                   .fees_book
                                   .get(instrument_kind)
                                   .cloned()
                                   .ok_or_else(|| ExchangeError::SandBox(format!("SandBoxExchange is not configured for InstrumentKind: {:?}", instrument_kind)))?;

        match role {
            | OrderRole::Maker => Ok(commission_rates.maker_fees),
            | OrderRole::Taker => Ok(commission_rates.taker_fees),
        }
    }

    /// 处理客户端交易列表并更新账户余额及交易事件。
    ///
    /// 该方法接收多个 `ClientTrade` 实例，并依次处理每笔交易：
    ///
    /// 1. 更新账户的相关余额信息。
    /// 2. 发送交易事件 `AccountEventKind::Trade`。
    /// 3. 发送余额更新事件 `AccountEventKind::Balance`。
    ///
    /// # 参数
    ///
    /// * `client_trades` - 一个包含多个 `ClientTrade` 实例的向量，表示客户端生成的交易记录。
    ///
    /// # 错误处理
    ///
    /// * 如果在应用交易变化时发生错误，会记录警告日志并继续处理下一笔交易。
    /// * 如果发送交易事件或余额事件失败，也会记录警告日志。
    ///
    /// # 注意事项
    ///
    /// * 当 `client_trades` 为空时，该方法不会执行任何操作。
    async fn process_trades(&mut self, client_trades: Vec<ClientTrade>)
    {
        if !client_trades.is_empty() {
            let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

            for trade in client_trades {
                // 直接调用 `self.apply_trade_changes` 来处理余额更新
                let balance_event = match self.apply_trade_changes(&trade).await {
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
                    warn!("[UniLinkEx] : Client offline - Failed to send AccountEvent::Trade: {:?}", err);
                }

                if let Err(err) = self.account_event_tx.send(balance_event) {
                    // 如果发送余额事件失败，记录警告日志
                    warn!("[UniLinkEx] : Client offline - Failed to send AccountEvent::Balance: {:?}", err);
                }
            }
        }
    }

    /// [PART 6] - [Miscellaneous]

    pub(crate) fn get_exchange_ts(&self) -> Result<i64, ExchangeError>
    {
        // 直接访问 account 的 exchange_timestamp 字段
        let exchange_ts = self.exchange_timestamp.load(Ordering::SeqCst);
        Ok(exchange_ts)
    }

    /// 查找匹配的订单，根据 `OrderId` 和 `ClientOrderId` 匹配。
    fn find_matching_order(orders: &[Order<Open>], request: &Order<RequestCancel>) -> Result<usize, ExchangeError>
    {
        orders.par_iter()
              .position_any(|order| Self::order_ids_check(order, request))
              .ok_or_else(|| ExchangeError::OrderNotFound { client_order_id: request.cid.clone(),
                                                            order_id: request.state.id.clone() })
    }

    /// 判断订单是否匹配，根据 `OrderId` 或 `ClientOrderId` 进行匹配。
    fn order_ids_check(order: &Order<Open>, request: &Order<RequestCancel>) -> bool
    {
        let id_match = match &request.state.id {
            | Some(req_id) => &order.state.id == req_id, // 直接比较 `OrderId`
            | None => false,
        };

        let cid_match = match (&order.cid, &request.cid) {
            | (Some(order_cid), Some(req_cid)) => order_cid == req_cid, // 比较 `ClientOrderId`
            | _ => false,
        };

        // 如果有 `OrderId` 或 `ClientOrderId` 匹配，说明订单匹配
        id_match || cid_match
    }

    /// 发送账户事件给客户端。
    pub(crate) fn send_account_event(&self, account_event: AccountEvent) -> Result<(), ExchangeError>
    {
        self.account_event_tx.send(account_event).map_err(|_| ExchangeError::ReponseSenderError)
    }
}


pub fn respond<Response>(response_tx: Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move { response_tx.send(response).expect("[UniLinkEx] : SandBoxExchange failed to send oneshot response to execution request") });
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{
            order::{identification::OrderId, states::request_open::RequestOpen},
            trade::ClientTradeId,
        },
        test_utils::create_test_account,
    };

    #[tokio::test]
    async fn test_validate_order_request_open()
    {
        let order = Order { instruction: OrderInstruction::Market,
                            exchange: Exchange::SandBox,
                            instrument: Instrument { base: Token::from("BTC"),
                                                     quote: Token::from("USD"),
                                                     kind: InstrumentKind::Spot },
                            timestamp: 1625247600000,
                            cid: Some(ClientOrderId("validCID123".into())),
                            side: Side::Buy,
                            state: RequestOpen { price: 50000.0,
                                                 size: 1.0,
                                                 reduce_only: false } };

        assert!(Account::validate_order_request_open(&order).is_ok());

        let invalid_order = Order { cid: Some(ClientOrderId("ars3214321431234rafsftdarstdars".into())), // Invalid ClientOrderId
                                    ..order.clone() };
        assert!(Account::validate_order_request_open(&invalid_order).is_err());
    }

    #[tokio::test]
    async fn test_validate_order_request_cancel()
    {
        let cancel_order = Order { instruction: OrderInstruction::Market,
                                   exchange: Exchange::SandBox,
                                   instrument: Instrument { base: Token::from("BTC"),
                                                            quote: Token::from("USD"),
                                                            kind: InstrumentKind::Spot },
                                   timestamp: 1625247600000,
                                   cid: Some(ClientOrderId("validCID123".into())),
                                   side: Side::Buy,
                                   state: RequestCancel { id: Some(OrderId::new(17213412341233948, generate_machine_id().unwrap(), 23)) } };

        assert!(Account::validate_order_request_cancel(&cancel_order).is_ok());

        let invalid_cancel_order = Order { state: RequestCancel { id: Some(OrderId(0)) }, // Invalid OrderId
                                           ..cancel_order.clone() };
        assert!(Account::validate_order_request_cancel(&invalid_cancel_order).is_err());
    }

    #[tokio::test]
    async fn test_handle_trade_data()
    {
        let mut account = create_test_account().await;

        let trade = MarketTrade { exchange: "binance-futures".to_string(),
                                  symbol: "BTC_USDT".to_string(),
                                  timestamp: 1625247600000,
                                  price: 100.0,
                                  side: Side::Buy.to_string(),
                                  amount: 0.0 };

        // 处理交易数据
        let result = account.handle_trade_data(&trade).await;
        assert!(result.is_ok());

        // 验证时间戳是否已更新
        assert_eq!(account.get_exchange_ts().unwrap(), 1625247600000);
    }

    #[tokio::test]
    async fn test_deposit_b_base()
    {
        let mut account = create_test_account().await;
        let btc_amount = 0.5;

        let balance = account.deposit_bitcoin(btc_amount).unwrap();

        assert_eq!(balance.token, Token("BTC".into()));
        assert_eq!(balance.balance.total, btc_amount);
        assert_eq!(balance.balance.available, btc_amount);
    }

    #[tokio::test]
    async fn test_initialize_tokens()
    {
        let mut account = create_test_account().await;

        // 初始化一些币种
        let tokens = vec!["大爷币".into(), "二爷币".into(), "姑奶奶币".into()];
        account.initialize_tokens(tokens.clone()).unwrap();

        // 检查这些币种是否被正确初始化，且初始余额为 0
        for token_str in tokens {
            let token = Token(token_str);
            let balance = account.get_balance(&token).unwrap();
            assert_eq!(balance.total, 0.0);
            assert_eq!(balance.available, 0.0);
        }
    }

    #[tokio::test]
    async fn test_fail_to_cancel_limit_order_due_to_invalid_order_id()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        let invalid_cancel_request = Order { instruction: OrderInstruction::Cancel,
                                             exchange: Exchange::SandBox,
                                             instrument: instrument.clone(),
                                             timestamp: 1625247600000,
                                             cid: Some(ClientOrderId("validCID123".into())),
                                             side: Side::Buy,
                                             state: RequestCancel { id: Some(OrderId(99999)) } /* 无效的OrderId */ };

        let result = account.atomic_cancel(invalid_cancel_request.clone()).await;
        // println!("Result: {:?}", result);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExchangeError::OrderNotFound { client_order_id: invalid_cancel_request.cid.clone(),
                                                                       order_id: Some(OrderId(99999)) });
    }

    #[tokio::test]
    async fn test_deposit_u_base()
    {
        let mut account = create_test_account().await;
        let usdt_amount = 100.0;

        {
            // 充值前查询 USDT 余额
            let initial_balance = account.get_balance(&Token::from("USDT")).unwrap();
            // println!("Initial USDT balance: {:?}", *initial_balance);
            assert_eq!(initial_balance.total, 10_000.0);
        } // `initial_balance` 的作用域在此结束，释放了不可变借用

        // 进行充值操作
        let balance = account.deposit_usdt(usdt_amount).unwrap();

        // 充值后再次查询 USDT 余额
        let updated_balance = account.get_balance(&Token::from("USDT")).unwrap();
        // println!("Updated USDT balance: {:?}", *updated_balance);

        // 验证余额更新
        assert_eq!(balance.token, Token("USDT".into()));
        assert_eq!(updated_balance.total, 10_000.0 + usdt_amount);
        assert_eq!(updated_balance.available, 10_000.0 + usdt_amount);
    }

    #[tokio::test]
    async fn test_buy_b_with_u()
    {
        let mut account = create_test_account().await;
        let usdt_amount = 100.0;
        let btc_price = 50_000.0;

        // 首先充值 USDT
        account.deposit_usdt(usdt_amount).unwrap();

        // 为 BTC 手动初始化一个余额（尽管余额为 0，但可以避免配置报错）
        account.deposit_bitcoin(0.0).unwrap();

        // 购买前查询 USDT 和 BTC 余额，提取实际值以避免生命周期问题
        let usdt_initial_balance = account.get_balance(&Token::from("USDT")).as_deref().unwrap().clone();
        let btc_initial_balance = account.get_balance(&Token::from("BTC")).as_deref().unwrap().clone();

        println!("Initial USDT balance: {:?}", usdt_initial_balance);
        println!("Initial BTC balance: {:?}", btc_initial_balance);

        assert_eq!(usdt_initial_balance.total, 10_000.0 + usdt_amount);
        assert_eq!(btc_initial_balance.total, 0.0);

        // 用 USDT 购买 BTC
        account.topup_bitcoin_with_usdt(usdt_amount, btc_price).unwrap();

        // 购买后查询 USDT 和 BTC 余额
        let usdt_balance = account.get_balance(&Token::from("USDT")).unwrap();
        let btc_balance = account.get_balance(&Token::from("BTC")).unwrap();

        println!("Updated USDT balance: {:?}", usdt_balance);
        println!("Updated BTC balance: {:?}", btc_balance);

        // 购买后，USDT 余额应为 10_000 - 100，BTC 余额应为 0.002
        assert_eq!(usdt_balance.total, 10_000.0);
        assert_eq!(btc_balance.total, usdt_amount / btc_price);
    }

    #[tokio::test]
    async fn test_match_market_event_with_open_order_sell_with_insufficient_balance()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 查询 USDT 和 ETH 余额并 clone，以避免借用冲突
        let initial_usdt_balance = account.get_balance(&Token::from("USDT")).unwrap().clone();
        let initial_eth_balance = account.get_balance(&Token::from("ETH")).unwrap().clone();

        println!("[test_match_market_event_with_open_order_sell] : Initial ETH balance: {:?}", initial_eth_balance);
        println!("[test_match_market_event_with_open_order_sell] : Initial USDT balance: {:?}", initial_usdt_balance);

        // 创建一个待开卖单订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID456".into())),
                                 side: Side::Sell,
                                 state: RequestOpen { reduce_only: false,
                                                      price: 16406.0,
                                                      size: 2.0 } };

        // 将订单添加到账户
        let result = account.atomic_open(open_order.clone()).await;
        assert_eq!(result.is_ok(), false);
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 16605.0,
                                         side: Side::Buy.to_string(),
                                         amount: 2.0 };

        // 匹配订单并生成交易事件
        let _ = account.match_orders(&market_event).await.unwrap();

        // 检查余额是否已更新
        let base_balance = account.get_balance(&instrument.base).unwrap();
        let quote_balance = account.get_balance(&instrument.quote).unwrap();

        assert_eq!(base_balance.total, 10.0);
        assert_eq!(base_balance.available, 10.0);
        assert_eq!(quote_balance.available, 10000.0); // 根本不能成交。若以不应该变。
        assert_eq!(quote_balance.total, 10000.0); // 根本不能成交。若以不应该变。
    }

    #[tokio::test]
    async fn test_match_market_event_with_open_order_sell_with_sufficient_balance()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
        account.deposit_usdt(50000.0).unwrap(); // 往测试里面充 50000 USDT.
                                                // 查询 USDT 和 ETH 余额并 clone，以避免借用冲突
        let initial_usdt_balance = account.get_balance(&Token::from("USDT")).unwrap().clone();
        let initial_eth_balance = account.get_balance(&Token::from("ETH")).unwrap().clone();

        println!("[test_match_market_event_with_open_order_sell] : Initial ETH balance: {:?}", initial_eth_balance);
        println!("[test_match_market_event_with_open_order_sell] : Initial USDT balance: {:?}", initial_usdt_balance);

        // 创建一个待开卖单订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID456".into())),
                                 side: Side::Sell,
                                 state: RequestOpen { reduce_only: false,
                                                      price: 16406.0,
                                                      size: 2.0 } };

        // 将订单添加到账户
        let result = account.atomic_open(open_order.clone()).await;
        assert_eq!(result.is_ok(), false);
        // let result = account.atomic_open(open_order).await;
        // assert_eq!(result.is_ok(), true);
        // // 创建一个市场事件，该事件与 open订单完全匹配
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 16605.0, // 前面已经确认是Maker单，成交计算的价格应该按照这里的 16605
                                         side: Side::Buy.to_string(),
                                         amount: 2.0 };

        // 匹配订单并生成交易事件
        let trades = account.match_orders(&market_event).await.unwrap();
        println!("trades:{:?}", trades);

        // 检查余额是否已更新 注意合约交易中base_balance不应该被改变
        let base_balance = account.get_balance(&instrument.base).unwrap();
        assert_eq!(base_balance.total, 10.0);
        assert_eq!(base_balance.available, 10.0);
        let quote_balance = account.get_balance(&instrument.quote).unwrap();
        assert_eq!(quote_balance.available, 27155.188); // Maker 价格
        assert_eq!(quote_balance.total, 59967.188); // NOTE this is correct remaining total
    }

    #[tokio::test]
    async fn test_get_open_orders_should_be_empty_after_matching()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 创建并添加订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID123".into())),
                                 side: Side::Buy,
                                 state: Open { id: OrderId::new(0, 0, 0),

                                               price: 100.0,
                                               size: 2.0,
                                               filled_quantity: 0.0,
                                               order_role: OrderRole::Maker } };
        account.orders.write().await.get_ins_orders_mut(&instrument).unwrap().add_order_open(open_order.clone());

        // 匹配一个完全匹配的市场事件
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 100.0,
                                         side: Side::Sell.to_string(),
                                         amount: 2.0 };
        let _ = account.match_orders(&market_event).await;

        // 获取未完成的订单
        let orders = account.orders.read().await.fetch_all();
        assert!(orders.is_empty(), "Expected no open orders after full match, but found some.");
    }

    #[tokio::test]
    async fn test_fail_to_open_limit_order_due_to_insufficient_funds()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 设置一个资金不足的场景，减少USDT的余额
        {
            let mut quote_balance = account.get_balance_mut(&instrument.quote).unwrap();
            quote_balance.available = 1.0; // 模拟 USDT 余额不足
        }

        // 创建一个待开买单订单
        let open_order_request = Order { instruction: OrderInstruction::Limit,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         timestamp: 1625247600000,
                                         cid: Some(ClientOrderId("validCID123".into())),
                                         side: Side::Buy,
                                         state: RequestOpen { price: 16499.0,
                                                              size: 5.0,
                                                              reduce_only: false } };

        let result = account.atomic_open(open_order_request).await;

        // 断言开单失败，且返回的错误是余额不足
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExchangeError::InsufficientBalance(instrument.quote));
    }

    #[tokio::test]
    async fn test_unsupported_instrument_kind()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("RRR".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Spot /* Spot Position is either not developed or not supported. */ },
                                  side: Side::Sell,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 执行管理仓位逻辑，应该返回错误
        let result = account.update_position_from_client_trade(trade.clone()).await;
        println!("result: {:?}", result);
        assert!(result.is_err());
    }
}
