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
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::warn;
use uuid::Uuid;

use crate::{
    common::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, machine_id::generate_machine_id},
            order_instructions::OrderInstruction,
            states::{cancelled::Cancelled, open::Open, request_cancel::RequestCancel, request_open::RequestOpen},
            Order, OrderRole,
        },
        position::{
            future::FuturePosition, leveraged_token::LeveragedTokenPosition, option::OptionPosition, perpetual::PerpetualPosition, AccountPositions, Position,
            PositionDirectionMode, PositionMarginMode,
        },
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::{
        account::account_config::{MarginMode, SandboxMode},
        clickhouse_api::datatype::clickhouse_trade_data::TardisTrade,
        instrument_orders::InstrumentOrders,
    },
    Exchange,
};
use account_config::AccountConfig;
use account_orders::AccountOrders;

pub mod account_config;
pub mod account_latency;
pub mod account_market_feed;
pub mod account_orders;

#[derive(Debug)]
pub struct Account
// where Vault: PositionHandler + BalanceHandler + StatisticHandler<Statistic>,
//       Statistic: Initialiser + PositionSummariser,
{
    pub current_session: Uuid,
    pub machine_id: u64,                                 // 机器ID
    pub exchange_timestamp: AtomicI64,                   // 交易所时间戳
    pub account_event_tx: UnboundedSender<AccountEvent>, // 帐户事件发送器
    pub config: Arc<AccountConfig>,                      // 帐户配置
    pub orders: Arc<RwLock<AccountOrders>>,              // 帐户订单集合
    pub balances: HashMap<Token, Balance>,               // 帐户余额
    pub positions: AccountPositions,                     /* 帐户持仓
                                                          * pub vault: Vault, */
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
                  config: Arc::clone(&self.config),
                  orders: Arc::clone(&self.orders),
                  balances: self.balances.clone(),
                  positions: self.positions.clone() }
    }
}
#[derive(Debug)]
pub struct AccountInitiator
{
    account_event_tx: Option<UnboundedSender<AccountEvent>>,
    config: Option<Arc<AccountConfig>>,
    orders: Option<Arc<RwLock<AccountOrders>>>,
    balances: Option<HashMap<Token, Balance>>,
    positions: Option<AccountPositions>,
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
                           positions: None }
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

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(Arc::new(RwLock::new(value)));
        self
    }

    pub fn balances(mut self, value: HashMap<Token, Balance>) -> Self
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
                     positions: self.positions.ok_or("positions are required")? })
    }
}

impl Account
{
    pub fn initiate() -> AccountInitiator
    {
        AccountInitiator::new()
    }

    /// [PART 1] handle positions and balances.
    pub async fn get_balances(&self) -> Vec<TokenBalance>
    {
        self.balances.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
    }

    pub async fn fetch_balances_and_respond(&self, response_tx: Sender<Result<Vec<TokenBalance>, ExecutionError>>)
    {
        let balances = self.get_balances().await;
        respond(response_tx, Ok(balances));
    }

    pub async fn fetch_positions_and_respond(&self, response_tx: Sender<Result<AccountPositions, ExecutionError>>)
    {
        let positions = self.positions.clone();
        respond(response_tx, Ok(positions));
    }

    /// 获取指定 `Instrument` 的仓位
    pub async fn get_position(&self, instrument: &Instrument) -> Result<Option<Position>, ExecutionError>
    {
        let positions = &self.positions; // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExecutionError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument)));
            }
            | InstrumentKind::Perpetual => {
                let perpetual_positions = &positions.perpetual_pos;
                if let Some(position) = perpetual_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::Perpetual(position.clone())));
                }
            }
            | InstrumentKind::Future => {
                let futures_positions = &positions.futures_pos;
                if let Some(position) = futures_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::Future(position.clone())));
                }
            }
            | InstrumentKind::CryptoOption => {
                let option_positions = &positions.option_pos;
                if let Some(position) = option_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::Option(position.clone())));
                }
            }
            | InstrumentKind::CryptoLeveragedToken => {
                let margin_positions = &positions.margin_pos;
                if let Some(position) = margin_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::LeveragedToken(position.clone())));
                }
            }
            | InstrumentKind::CommodityOption | InstrumentKind::CommodityFuture => {
                todo!("Commodity positions are not yet implemented");
            }
        }

        Ok(None) // 没有找到对应的仓位
    }

    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn get_balance(&self, token: &Token) -> Result<&Balance, ExecutionError>
    {
        self.balances
            .get(token)
            .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn get_balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError>
    {
        self.balances
            .get_mut(token)
            .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for Token: {token}")))
    }

    pub async fn required_available_balance<'a>(&'a self, order: &'a Order<RequestOpen>, current_price: f64) -> (&Token, f64)
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
                todo!("CryptoOption is not supported yet")
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!("CryptoLeveragedToken is not supported yet")
            }
            | InstrumentKind::CommodityOption => {
                todo!("CommodityOption is not supported yet")
            }
            | InstrumentKind::CommodityFuture => {
                todo!("CommodityFuture is not supported yet")
            }
        }
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError>
    {
        let available = self.get_balance(token)?.available;
        if available >= required_balance {
            Ok(())
        }
        else {
            Err(ExecutionError::InsufficientBalance(token.clone()))
        }
    }

    pub fn determine_position_direction_mode(&self) -> Result<PositionDirectionMode, ExecutionError>
    {
        let position_mode = self.config.position_mode.clone();
        Ok(position_mode)
    }

    pub fn determine_position_margin_mode(&self) -> Result<PositionMarginMode, ExecutionError>
    {
        let position_margin_mode = self.config.position_margin_mode.clone();
        Ok(position_margin_mode)
    }

    pub fn determine_margin_mode(&self) -> Result<MarginMode, ExecutionError>
    {
        let margin_mode = self.config.margin_mode.clone();
        Ok(margin_mode)
    }

    /// 更新指定 `Instrument` 的仓位
    pub async fn set_position(&mut self, position: Position) -> Result<(), ExecutionError>
    {
        match position {
            | Position::Perpetual(pos) => self.set_perpetual_position(pos).await,
            | Position::Future(pos) => self.set_future_position(pos).await,
            | Position::Option(pos) => self.set_option_position(pos).await,
            | Position::LeveragedToken(pos) => self.set_leveraged_token_position(pos).await,
        }
    }

    /// 更新 PerpetualPosition 的方法
    ///
    /// 这个方法用于更新账户中的永续合约仓位信息。如果当前账户中已经存在
    /// 对应金融工具（`Instrument`）的仓位，则更新其信息；否则，将新的仓位
    /// 添加到永续合约仓位列表中。
    ///
    /// # 参数
    ///
    /// * `pos` - 需要更新的 `PerpetualPosition` 对象。
    ///
    /// # 返回值
    ///
    /// 如果更新成功，返回 `Ok(())`，否则返回一个 `ExecutionError`。
    async fn set_perpetual_position(&mut self, pos: PerpetualPosition) -> Result<(), ExecutionError>
    {
        // 获取账户的锁，确保在更新仓位信息时没有并发访问的问题
        let positions = &mut self.positions;

        // 获取永续合约仓位的可变引用
        let perpetual_positions = &mut positions.perpetual_pos;

        // 查找是否存在与传入 `pos` 相同的 `instrument`
        if let Some(existing_pos) = perpetual_positions.iter_mut().find(|p| p.meta.instrument == pos.meta.instrument) {
            // 如果找到了相同的 `instrument`，则更新现有仓位信息
            *existing_pos = pos;
        }
        else {
            // 如果没有找到相同的 `instrument`，将新的仓位添加到永续合约仓位列表中
            perpetual_positions.push(pos);
        }

        Ok(())
    }

    /// 更新 FuturePosition 的方法（占位符）
    async fn set_future_position(&mut self, _pos: FuturePosition) -> Result<(), ExecutionError>
    {
        todo!("[UniLinkExecution] : Updating Future positions is not yet implemented")
    }

    /// 更新 OptionPosition 的方法（占位符）
    async fn set_option_position(&mut self, _pos: OptionPosition) -> Result<(), ExecutionError>
    {
        todo!("[UniLinkExecution] : Updating Option positions is not yet implemented")
    }

    /// 更新 LeveragedTokenPosition 的方法（占位符）
    async fn set_leveraged_token_position(&mut self, _pos: LeveragedTokenPosition) -> Result<(), ExecutionError>
    {
        todo!("[UniLinkExecution] : Updating Leveraged Token positions is not yet implemented")
    }

    /// 检查在 AccountPositions 中是否已经存在该 instrument 的某个仓位
    /// 需要首先从 open 订单中确定 InstrumentKind，因为仓位类型各不相同
    pub async fn any_position_open(&self, open: &Order<Open>) -> Result<bool, ExecutionError>
    {
        let positions_lock = &self.positions; // 获取锁

        // 直接调用 AccountPositions 中的 has_position 方法
        if positions_lock.has_position(&open.instrument) {
            return Ok(true);
        }

        Ok(false)
    }

    async fn check_position_direction_conflict(&self, instrument: &Instrument, side: Side) -> Result<(), ExecutionError>
    {
        let positions_lock = &self.positions;

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExecutionError::NotImplemented("Spot position conflict check not implemented".into()));
            }
            | InstrumentKind::CommodityOption => {
                return Err(ExecutionError::NotImplemented("CommodityOption position conflict check not implemented".into()));
            }
            | InstrumentKind::CommodityFuture => {
                return Err(ExecutionError::NotImplemented("CommodityFuture position conflict check not implemented".into()));
            }
            | InstrumentKind::Perpetual => {
                let perpetual_positions = &positions_lock.perpetual_pos;
                for pos in perpetual_positions {
                    if pos.meta.instrument == *instrument && pos.meta.side != side {
                        return Err(ExecutionError::InvalidDirection);
                    }
                }
            }
            | InstrumentKind::Future => {
                let futures_positions = &positions_lock.futures_pos;
                for pos in futures_positions {
                    if pos.meta.instrument == *instrument && pos.meta.side != side {
                        return Err(ExecutionError::InvalidDirection);
                    }
                }
            }
            | InstrumentKind::CryptoOption => {
                let option_positions = &positions_lock.option_pos;
                for pos in option_positions {
                    if pos.meta.instrument == *instrument && pos.meta.side != side {
                        return Err(ExecutionError::InvalidDirection);
                    }
                }
            }
            | InstrumentKind::CryptoLeveragedToken => {
                let margin_positions = &positions_lock.margin_pos;
                for pos in margin_positions {
                    if pos.meta.instrument == *instrument && pos.meta.side != side {
                        return Err(ExecutionError::InvalidDirection);
                    }
                }
            }
        }
        Ok(())
    }

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub async fn apply_open_order_changes(&mut self, open: &Order<Open>, required_balance: f64) -> Result<AccountEvent, ExecutionError>
    {
        // println!("[UniLinkExecution] : Starting apply_open_order_changes: {:?}, with balance: {:?}", open, required_balance);

        // 配置从直接访问 `self.config` 获取
        let (position_mode, position_margin_mode) = (self.config.position_mode.clone(), self.config.position_margin_mode.clone());

        // println!("[UniLinkExecution] : Retrieved position_mode: {:?}, position_margin_mode: {:?}",
        //          position_mode, position_margin_mode);

        // 根据不同的 InstrumentKind 进行处理
        match open.instrument.kind {
            | InstrumentKind::Spot => {
                todo!("[UniLinkExecution] : Spot handling is not implemented yet");
            }
            | InstrumentKind::CryptoOption => {
                todo!("[UniLinkExecution] : Option handling is not implemented yet");
            }
            | InstrumentKind::CommodityFuture => {
                todo!("[UniLinkExecution] : Commodity future handling is not implemented yet");
            }
            | InstrumentKind::CommodityOption => {
                todo!("[UniLinkExecution] : Commodity option handling is not implemented yet");
            }
            | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken => {
                if position_mode == PositionDirectionMode::NetMode {
                    self.check_position_direction_conflict(&open.instrument, open.side).await?;
                }
            }
        }

        // 根据 PositionMarginMode 处理余额更新
        match (open.instrument.kind, position_margin_mode) {
            | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken, PositionMarginMode::Cross) => {
                todo!("Handle Cross Margin");
            }
            | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken, PositionMarginMode::Isolated) => match open.side {
                | Side::Buy => {
                    let delta = BalanceDelta { total: 0.0,
                                               available: -required_balance };
                    self.apply_balance_delta(&open.instrument.quote, delta);
                }
                | Side::Sell => {
                    let delta = BalanceDelta { total: 0.0,
                                               available: -required_balance };
                    self.apply_balance_delta(&open.instrument.base, delta);
                }
            },
            | (_, _) => {
                return Err(ExecutionError::SandBox(format!(
                    "[UniLinkExecution] : Unsupported InstrumentKind or PositionMarginMode for open order: {:?}",
                    open.instrument.kind
                )));
            }
        };

        // 更新后的余额
        let updated_balance = match open.side {
            | Side::Buy => *self.get_balance(&open.instrument.quote)?,
            | Side::Sell => *self.get_balance(&open.instrument.base)?,
        };

        Ok(AccountEvent { exchange_timestamp: self.exchange_timestamp.load(Ordering::SeqCst),
                          exchange: Exchange::SandBox,
                          kind: AccountEventKind::Balance(TokenBalance::new(open.instrument.quote.clone(), updated_balance)) })
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn apply_cancel_order_changes(&mut self, cancelled: &Order<Open>) -> TokenBalance
    {
        match cancelled.side {
            | Side::Buy => {
                let balance = self.get_balance_mut(&cancelled.instrument.quote)
                                  .expect("[UniLinkExecution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.get_balance_mut(&cancelled.instrument.base)
                                  .expect("[UniLinkExecution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// 从交易中更新余额并返回 [`AccountEvent`]
    pub async fn apply_trade_changes(&mut self, trade: &ClientTrade) -> Result<AccountEvent, ExecutionError>
    {
        let Instrument { base, quote, kind, .. } = &trade.instrument;
        let fee = trade.fees; // 直接从 TradeEvent 中获取费用
        let side = trade.side; // 直接使用 TradeEvent 中的 side

        match kind {
            | InstrumentKind::Spot => {
                todo!("Spot handling is not implemented yet");
            }
            | InstrumentKind::CryptoOption => {
                todo!("Option handling is not implemented yet");
            }
            | InstrumentKind::CommodityOption => {
                todo!("CommodityOption handling is not implemented yet")
            }
            | InstrumentKind::CommodityFuture => {
                todo!("CommodityFuture handling is not implemented yet")
            }
            | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken => {
                let (base_delta, quote_delta) = match side {
                    | Side::Buy => {
                        let base_increase = trade.quantity - fee;
                        // Note: available was already decreased by the opening of the Side::Buy order
                        let base_delta = BalanceDelta { total: base_increase,
                                                        available: base_increase };
                        let quote_delta = BalanceDelta { total: -trade.quantity * trade.price,
                                                         available: 0.0 };
                        (base_delta, quote_delta)
                    }
                    | Side::Sell => {
                        // Note: available was already decreased by the opening of the Side::Sell order
                        let base_delta = BalanceDelta { total: -trade.quantity,
                                                        available: 0.0 };
                        let quote_increase = (trade.quantity * trade.price) - fee;
                        let quote_delta = BalanceDelta { total: quote_increase,
                                                         available: quote_increase };
                        (base_delta, quote_delta)
                    }
                };

                let base_balance = self.apply_balance_delta(base, base_delta);
                let quote_balance = self.apply_balance_delta(quote, quote_delta);

                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().expect("[UniLinkExecution] : Failed to get exchange timestamp"),
                                  exchange: Exchange::SandBox,
                                  kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance),]) })
            }
        }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn apply_balance_delta(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.get_balance_mut(token).unwrap();

        let _ = base_balance.apply(delta);

        *base_balance
    }

    /// [PART 2] 杂项方法。
    /// `get_fee` 是获取手续费的方法，用于获取 maker 和 taker 手续费
    /// `get_exchange_ts` 是获取当前时间戳的方法
    /// `update_exchange_timestamp` 是基本的时间戳更新方法，用于更新 `exchange_timestamp` 值。
    /// `generate_request_id` 生成请求id。

    pub async fn get_fee(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExecutionError>
    {
        // 直接访问 account 的 config 字段
        let commission_rates = self.config
                                   .fees_book
                                   .get(instrument_kind)
                                   .cloned()
                                   .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for InstrumentKind: {:?}", instrument_kind)))?;

        match role {
            | OrderRole::Maker => Ok(commission_rates.maker_fees),
            | OrderRole::Taker => Ok(commission_rates.taker_fees),
        }
    }

    pub fn get_exchange_ts(&self) -> Result<i64, ExecutionError>
    {
        // 直接访问 account 的 exchange_timestamp 字段
        let exchange_ts = self.exchange_timestamp.load(Ordering::SeqCst);
        Ok(exchange_ts)
    }

    pub fn update_exchange_ts(&self, timestamp: i64)
    {
        let adjusted_timestamp = match self.config.execution_mode {
            | SandboxMode::Backtest => timestamp,                                                            // 在回测模式下使用传入的时间戳
            | SandboxMode::Online => SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64, // 在实时模式下使用当前时间
        };
        self.exchange_timestamp.store(adjusted_timestamp, Ordering::SeqCst);
    }

    /// 处理交易数据的方法
    pub async fn handle_trade_data(&mut self, trade: TardisTrade) -> Result<(), ExecutionError>
    {
        // 更新时间戳
        self.update_exchange_ts(trade.timestamp);
        // self.process_trade(trade).await?;
        Ok(())
    }

    /// [PART 3]
    /// `fetch_orders_open_and_respond` 从 `open_orders` 读取所有订单，并将其作为 `response_tx` 发送。
    /// `open_orders` 执行开单操作。
    /// `atomic_open` 尝试以原子操作方式打开一个订单，确保在验证和更新账户余额后安全地打开订单。
    /// `required_available_balance` 计算打开订单所需的可用余额，用于验证账户中是否有足够的资金执行订单。

    pub async fn fetch_orders_open_and_respond(&self, response_tx: Sender<Result<Vec<Order<Open>>, ExecutionError>>)
    {
        let orders = self.orders.read().await.fetch_all();
        respond(response_tx, Ok(orders));
    }

    pub async fn open_orders(&mut self,
                             open_requests: Vec<Order<RequestOpen>>,
                             response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)
                             -> Result<(), ExecutionError>
    {
        let mut open_results = Vec::new();

        for request in open_requests {
            let processed_request = match self.config.execution_mode {
                | SandboxMode::Backtest => self.orders.write().await.process_backtest_requestopen_with_a_simulated_latency(request).await,
                | _ => request, // 实时模式下直接使用原始请求
            };

            let current_price = match processed_request.side {
                | Side::Buy => {
                    let token = &processed_request.instrument.base;
                    let balance = self.get_balance(token)?;
                    balance.current_price
                }
                | Side::Sell => {
                    let token = &processed_request.instrument.quote;
                    let balance = self.get_balance(token)?;
                    balance.current_price
                }
            };

            let open_result = self.atomic_open(current_price, processed_request).await;
            open_results.push(open_result);
        }

        if let Err(e) = response_tx.send(open_results) {
            return Err(ExecutionError::SandBox(format!("Failed to send open order results: {:?}", e)));
        }

        Ok(())
    }

    pub async fn atomic_open(&mut self, current_price: f64, order: Order<RequestOpen>) -> Result<Order<Open>, ExecutionError>
    {
        Self::validate_order_instruction(order.kind)?;
        println!("atomic_open: {:?}", order);
        // 提前声明所需的变量
        let order_role = {
            let orders_guard = self.orders.read().await; // 使用读锁来判断订单角色
            orders_guard.determine_maker_taker(&order, current_price)?
        };
        println!("order_role: {:?}", order_role);
        // 计算所需的可用余额，尽量避免锁操作
        let (token, required_balance) = self.required_available_balance(&order, current_price).await;
        println!("required_balance: {:?}", required_balance);
        // 检查余额是否充足，并在锁定后更新订单
        self.has_sufficient_available_balance(token, required_balance)?;

        let open_order = {
            let mut orders_guard = self.orders.write().await; // 使用写锁来创建订单
            let open_order = orders_guard.build_order_open(order, order_role).await;
            orders_guard.get_ins_orders_mut(&open_order.instrument)?.add_order_open(open_order.clone());
            open_order
        };
        println!("open_order: {:?}", open_order);
        // 应用订单变更并发送事件 NOTE test3 failed because of this line.
        let balance_event = self.apply_open_order_changes(&open_order, required_balance).await?;
        println!("balance_event: {:?}",balance_event);
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);
        println!("exchange_timestamp: {:?}",exchange_timestamp);
        self.account_event_tx
            .send(balance_event)
            .expect("[UniLinkExecution] : Client offline - Failed to send AccountEvent::Balance");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: Exchange::SandBox,
                                 kind: AccountEventKind::OrdersNew(vec![open_order.clone()]) })
            .expect("[UniLinkExecution] : Client offline - Failed to send AccountEvent::Trade");
        Ok(open_order)
    }

    /// [PART 4]
    /// `validate_order_instruction` 验证订单的合法性，确保订单类型是受支持的。
    /// `validate_order_request_open` 验证开单请求的合法性，确保订单类型是受支持的。
    /// `match_orders` 处理市场事件，根据市场事件匹配相应的订单并生成交易。
    /// `get_orders_for_instrument` 获取与特定金融工具相关的订单，用于进一步的订单匹配操作。
    /// `determine_fees_percent` 根据金融工具类型和订单方向确定适用的费用百分比。

    pub fn validate_order_instruction(kind: OrderInstruction) -> Result<(), ExecutionError>
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

    pub fn validate_order_request_open(order: &Order<RequestOpen>) -> Result<(), ExecutionError>
    {
        // 检查是否提供了有效的 ClientOrderId
        if let Some(cid) = &order.cid.0 {
            if cid.trim().is_empty() {
                return Err(ExecutionError::InvalidRequestOpen("ClientOrderId is empty".into()));
            }

            // 使用 validate_id_format 验证 ID 格式
            if !ClientOrderId::validate_id_format(cid) {
                return Err(ExecutionError::InvalidRequestOpen(format!("Invalid ClientOrderId format: {}", cid)));
            }
        }

        // 检查订单类型是否合法
        Account::validate_order_instruction(order.kind)?;

        // 检查价格是否合法（应为正数）
        if order.state.price <= 0.0 {
            return Err(ExecutionError::InvalidRequestOpen(format!("Invalid price: {}", order.state.price)));
        }

        // 检查数量是否合法（应为正数）
        if order.state.size <= 0.0 {
            return Err(ExecutionError::InvalidRequestOpen(format!("Invalid size: {}", order.state.size)));
        }

        // 检查基础货币和报价货币是否相同
        if order.instrument.base == order.instrument.quote {
            return Err(ExecutionError::InvalidRequestOpen(format!("Base and Quote tokens must be different: {}", order.instrument.base)));
        }

        Ok(())
    }

    pub fn validate_order_request_cancel(order: &Order<RequestCancel>) -> Result<(), ExecutionError>
    {
        // 检查是否提供了有效的 OrderId
        if order.state.id.value() == 0 {
            return Err(ExecutionError::InvalidRequestCancel("OrderId is missing or invalid".into()));
        }

        // 检查基础货币和报价货币是否相同
        if order.instrument.base == order.instrument.quote {
            return Err(ExecutionError::InvalidRequestCancel("Base and Quote tokens must be different".into()));
        }

        Ok(())
    }

    pub async fn match_orders(&mut self, market_trade: &TardisTrade, instrument_kind: InstrumentKind) -> Vec<ClientTrade>
    {
        let mut trades = Vec::new();
        // parse base from MarketTrade's symbol(which is formatted as base_quote)
        let base = Token::from(market_trade.parse_base().unwrap());
        let quote =Token::from(market_trade.parse_quote().unwrap());
        let instrument = Instrument {base,quote,kind: instrument_kind };

        if let Some(mut instrument_orders) = self.get_orders_for_instrument(&instrument).await {
            if let Some(matching_side) = instrument_orders.determine_matching_side(&market_trade) {
                match matching_side {
                    | Side::Buy => {
                        trades.append(&mut instrument_orders.match_bids(&market_trade,
                                                                        self.determine_fees_percent(&instrument_kind, &OrderRole::Taker)
                                                                            .expect("Missing fees percent")));
                    }
                    | Side::Sell => {
                        trades.append(&mut instrument_orders.match_asks(&market_trade,
                                                                        self.determine_fees_percent(&instrument_kind, &OrderRole::Taker)
                                                                            .expect("Missing fees percent")));
                    }
                }
            }
        }
        else {
            warn!("No orders found for the given instrument in the market event.");
        };

        self.process_trades(trades.clone()).await;

        trades
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

    async fn process_trades(&mut self, trades: Vec<ClientTrade>)
    {
        if !trades.is_empty() {
            let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

            for trade in trades {
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
                    warn!("[UniLinkExecution] : Client offline - Failed to send AccountEvent::Trade: {:?}", err);
                }

                if let Err(err) = self.account_event_tx.send(balance_event) {
                    // 如果发送余额事件失败，记录警告日志
                    warn!("[UniLinkExecution] : Client offline - Failed to send AccountEvent::Balance: {:?}", err);
                }
            }
        }
    }

    /// [PART 5]
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
        Self::validate_order_request_cancel(&request)?;
        // 首先使用读锁来查找并验证订单是否存在，同时减少写锁的持有时间
        let removed_order = {
            let orders_guard = self.orders.read().await;
            let mut orders = orders_guard.get_ins_orders_mut(&request.instrument)?;
            println!("[process_cancel_request_into_cancelled_atomic]: found orders: {:?}", *orders);
            println!("[process_cancel_request_into_cancelled_atomic]: now trying to cancel {:?}", request);
            // 查找并移除订单，这里使用写锁来修改订单集合
            match request.side {
                | Side::Buy => {
                    let index = orders.bids
                                      .par_iter()
                                      .position_any(|bid| bid.state.id == request.state.id)
                                      .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                    orders.bids.remove(index)
                }
                | Side::Sell => {
                    let index = orders.asks
                                      .par_iter()
                                      .position_any(|ask| ask.state.id == request.state.id)
                                      .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                    orders.asks.remove(index)
                }
            }
        };

        // 处理余额更新（不需要持有订单写锁）
        let balance_event = self.apply_cancel_order_changes(&removed_order);

        // 将 Order<Open> 映射到 Order<Cancelled>
        let cancelled = Order::from(removed_order);

        // 获取当前的 exchange_timestamp
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 发送 AccountEvents 给客户端（不需要持有订单写锁）
        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: Exchange::SandBox,
                                 kind: AccountEventKind::OrdersCancelled(vec![cancelled.clone()]) })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Trade");

        self.account_event_tx
            .send(AccountEvent { exchange_timestamp,
                                 exchange: Exchange::SandBox,
                                 kind: AccountEventKind::Balance(balance_event) })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Balance");

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
                                                                                              timestamp: 0 })
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
                   .expect("[UniLinkExecution] : SandBoxExchange failed to send oneshot response to execution request")
    });
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::order::{identification::OrderId, states::request_open::RequestOpen},
        test_utils::{create_test_account, create_test_future_position_with_side, create_test_perpetual_position},
    };

    #[tokio::test]
    async fn test_validate_order_request_open()
    {
        let order = Order { kind: OrderInstruction::Market,
                            exchange: Exchange::SandBox,
                            instrument: Instrument { base: Token::from("BTC"),
                                                     quote: Token::from("USD"),
                                                     kind: InstrumentKind::Spot },
                            timestamp: 1625247600000,
                            cid: ClientOrderId(Some("validCID123".into())),
                            side: Side::Buy,
                            state: RequestOpen { price: 50000.0,
                                                 size: 1.0,
                                                 reduce_only: false } };

        assert!(Account::validate_order_request_open(&order).is_ok());

        let invalid_order = Order { cid: ClientOrderId(Some("".into())), // Invalid ClientOrderId
                                    ..order.clone() };
        assert!(Account::validate_order_request_open(&invalid_order).is_err());
    }

    #[tokio::test]
    async fn test_validate_order_request_cancel()
    {
        let cancel_order = Order { kind: OrderInstruction::Market,
                                   exchange: Exchange::SandBox,
                                   instrument: Instrument { base: Token::from("BTC"),
                                                            quote: Token::from("USD"),
                                                            kind: InstrumentKind::Spot },
                                   timestamp: 1625247600000,
                                   cid: ClientOrderId(Some("validCID123".into())),
                                   side: Side::Buy,
                                   state: RequestCancel { id: OrderId(12345) } };

        assert!(Account::validate_order_request_cancel(&cancel_order).is_ok());

        let invalid_cancel_order = Order { state: RequestCancel { id: OrderId(0) }, // Invalid OrderId
                                           ..cancel_order.clone() };
        assert!(Account::validate_order_request_cancel(&invalid_cancel_order).is_err());
    }

    #[tokio::test]
    async fn test_get_balance()
    {
        let account = create_test_account().await;

        let token = Token::from("TEST_BASE");
        let balance = account.get_balance(&token).unwrap();
        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 10.0);
    }

    #[tokio::test]
    async fn test_get_balance_mut()
    {
        let mut account = create_test_account().await;

        let token = Token::from("TEST_BASE");
        let balance = account.get_balance_mut(&token).unwrap();
        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 10.0);
    }

    #[tokio::test]
    async fn test_get_fee()
    {
        let account = create_test_account();
        let fee = account.await.get_fee(&InstrumentKind::Perpetual, OrderRole::Maker).await.unwrap();
        assert_eq!(fee, 0.001);
    }

    #[tokio::test]
    async fn test_determine_position_mode()
    {
        let account = create_test_account().await;
        let position_mode = account.determine_position_direction_mode().unwrap();
        assert_eq!(position_mode, PositionDirectionMode::NetMode);
    }

    #[tokio::test]
    async fn test_determine_margin_mode()
    {
        let account = create_test_account().await;
        let margin_mode = account.determine_margin_mode().unwrap();
        assert_eq!(margin_mode, MarginMode::SingleCurrencyMargin);
        assert_ne!(margin_mode, MarginMode::MultiCurrencyMargin);
    }

    #[tokio::test]
    async fn test_determine_position_margin_mode()
    {
        let account = create_test_account().await;
        let position_margin_mode = account.determine_position_margin_mode().unwrap();
        assert_eq!(position_margin_mode, PositionMarginMode::Isolated);
        assert_ne!(position_margin_mode, PositionMarginMode::Cross);
    }

    #[tokio::test]
    async fn test_set_position()
    {
        let mut account = create_test_account().await;
        let instrument = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual));
        let perpetual_position = create_test_perpetual_position(instrument.clone());
        account.set_position(Position::Perpetual(perpetual_position)).await.unwrap();
        let position_result = account.get_position(&instrument).await;
        assert!(position_result.is_ok());
        assert!(position_result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_fetch_all_balances()
    {
        let account = create_test_account().await;

        let all_balances = account.get_balances().await;

        assert_eq!(all_balances.len(), 2, "Expected 2 balances but got {}", all_balances.len());

        assert!(all_balances.iter().any(|b| b.token == Token::from("TEST_BASE")), "Expected TEST_BASE balance not found");
        assert!(all_balances.iter().any(|b| b.token == Token::from("TEST_QUOTE")), "Expected TEST_QUOTE balance not found");
    }
    #[tokio::test]
    async fn test_get_position_none()
    {
        let account = create_test_account().await;

        let instrument = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual));
        let position = account.get_position(&instrument).await.unwrap();
        assert!(position.is_none());
    }

    #[tokio::test]
    async fn test_has_sufficient_available_balance()
    {
        let account = create_test_account().await;

        let token = Token::from("TEST_BASE");
        let result = account.has_sufficient_available_balance(&token, 5.0);
        assert!(result.is_ok());

        let result = account.has_sufficient_available_balance(&token, 15.0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_balance_delta()
    {
        let mut account = create_test_account().await;

        let token = Token::from("TEST_BASE");
        let delta = BalanceDelta::new(0.0, -10.0);

        let balance = account.apply_balance_delta(&token, delta);

        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 0.0);
    }

    #[tokio::test]
    async fn test_apply_open_order_changes()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual));
        let open_order_request = Order { kind: OrderInstruction::Limit,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         timestamp: 1625247600000,
                                         cid: ClientOrderId(Some("validCID123".into())),
                                         side: Side::Buy,
                                         state: RequestOpen { price: 1.0,
                                                              size: 2.0,
                                                              reduce_only: false } };

        // 将订单状态从 RequestOpen 转换为 Open
        let open_order = Order { kind: open_order_request.kind,
                                 exchange: open_order_request.exchange,
                                 instrument: open_order_request.instrument.clone(),
                                 timestamp: open_order_request.timestamp,
                                 cid: open_order_request.cid.clone(),
                                 side: open_order_request.side,
                                 state: Open { id: OrderId::new(0, 0, 0), // 使用一个新的 OrderId
                                               price: open_order_request.state.price,
                                               size: open_order_request.state.size,
                                               filled_quantity: 0.0,
                                               order_role: OrderRole::Maker } };

        let required_balance = 2.0; // 模拟需要的余额

        let result = account.apply_open_order_changes(&open_order, required_balance).await;
        assert!(result.is_ok());

        let balance = account.get_balance(&Token::from("TEST_QUOTE")).unwrap();
        assert_eq!(balance.available, 9998.0); // 原始余额是 10,000.0，减去 2.0 后应该是 9998.0
    }

    #[tokio::test]
    async fn test_check_position_direction_conflict()
    {
        let mut account = create_test_account().await;

        // 情况1：没有冲突的情况下调用
        let instrument = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual));
        let result = account.check_position_direction_conflict(&instrument, Side::Buy).await;
        assert!(result.is_ok());

        // 情况2：模拟存在冲突的Perpetual仓位，注意这里 `side` 是 `Sell`
        let perpetual_position = create_test_perpetual_position(instrument.clone());
        account.positions.perpetual_pos.push(perpetual_position);

        let result = account.check_position_direction_conflict(&instrument, Side::Sell).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);

        // 情况3：模拟不存在冲突的Future仓位
        let instrument_future = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Future));
        let result = account.check_position_direction_conflict(&instrument_future, Side::Buy).await;
        assert!(result.is_ok());

        // 情况4：模拟存在冲突的Future仓位，注意这里 `side` 是 `Sell`
        let future_position = create_test_future_position_with_side(instrument_future.clone(), Side::Sell);
        account.positions.futures_pos.push(future_position);

        let result = account.check_position_direction_conflict(&instrument_future, Side::Buy).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);

        // 情况5：其他 InstrumentKind 还没有实现，因此我们只需要检查它们是否返回未实现的错误
        let instrument_spot = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Spot));
        let result = account.check_position_direction_conflict(&instrument_spot, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));

        let instrument_commodity_future = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::CommodityFuture));
        let result = account.check_position_direction_conflict(&instrument_commodity_future, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));

        let instrument_commodity_option = Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::CommodityOption));
        let result = account.check_position_direction_conflict(&instrument_commodity_option, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));
    }
}
