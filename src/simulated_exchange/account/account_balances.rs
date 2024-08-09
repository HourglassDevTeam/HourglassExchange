use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Arc, Mutex, Weak},
};

use tokio::sync::RwLock;

use crate::{
    common_skeleton::{
        balance::{Balance, BalanceDelta, TokenBalance},
        datafeed::event::MarketEvent,
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{Open, Order},
        position::{AccountPositions, PositionDirectionMode, PositionKind, PositionMarginMode},
        token::Token,
        Side,
    },
    error::ExecutionError,
    simulated_exchange::{
        account::{account_config::MarginMode, Account},
        clickhouse_api::queries_operations::ClickhouseTrade,
    },
    ExchangeVariant,
};

#[derive(Clone, Debug)]
pub struct AccountState<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord + Ord
{
    pub balances: HashMap<Token, Balance>,
    pub positions: Arc<Mutex<AccountPositions>>,
    pub account_ref: Weak<RwLock<Account<Event>>>, // NOTE :如果不使用弱引用，可能会导致循环引用和内存泄漏。
}

impl<Event> PartialEq for AccountState<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn eq(&self, other: &Self) -> bool
    {
        self.balances == other.balances
        // account_ref 是Weak<RwLock<>>，一般不会比较其内容
    }
}

impl<Event> AccountState<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError>
    {
        self.balances
            .get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError>
    {
        self.balances
            .get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// Sets the account reference. 设置 Account 实例的引用 FIXME to be moved to a AccountStateInitiator (TBD).
    // pub fn set_account(&mut self, account: Arc<RwLock<Account<Event>>>) {
    //     self.account_ref = Arc::downgrade(&account);
    // }

    /// 获取指定 [`InstrumentKind`] 的手续费。
    pub async fn get_fee(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            account_read.config
                        .fees_book
                        .get(instrument_kind)
                        .cloned()
                        .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for InstrumentKind: {:?}", instrument_kind)))
        }
        else {
            Err(ExecutionError::Simulated("Account reference is not set".to_string()))
        }
    }

    // 异步方法来获取 Exchange 的 timestamp.
    pub async fn get_exchange_ts(&self) -> Result<i64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            Ok(account_read.exchange_timestamp.load(Ordering::SeqCst))
        }
        else {
            Err(ExecutionError::Simulated("Account reference is not set".to_string()))
        }
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance>
    {
        self.balances.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError>
    {
        let available = self.balance(token)?.available;
        if available >= required_balance {
            Ok(())
        }
        else {
            Err(ExecutionError::InsufficientBalance(token.clone()))
        }
    }

    /// 判断Account的当前持仓模式。
    #[allow(dead_code)]
    async fn determine_position_mode(&self) -> Result<PositionDirectionMode, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            Ok(account_read.config.position_mode.clone())
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 判断Account的当前保证金模式。
    #[allow(dead_code)]
    async fn determine_margin_mode(&self) -> Result<MarginMode, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            Ok(account_read.config.margin_mode.clone())
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 获取指定 `Instrument` 的仓位
    pub async fn get_position(&self, instrument: &Instrument) -> Result<Option<PositionKind>, ExecutionError>
    {
        let positions = self.positions.lock().unwrap(); // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => return Err(ExecutionError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument))),
            | InstrumentKind::Perpetual => {
                if let Some(perpetual_positions) = &positions.perpetual_pos {
                    if let Some(position) = perpetual_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(PositionKind::Perpetual(position.clone())));
                    }
                }
            }
            | InstrumentKind::Future => {
                if let Some(futures_positions) = &positions.futures_pos {
                    if let Some(position) = futures_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(PositionKind::Future(position.clone())));
                    }
                }
            }
            | InstrumentKind::Option => {
                if let Some(option_positions) = &positions.option_pos {
                    if let Some(position) = option_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(PositionKind::Option(position.clone())));
                    }
                }
            }
            | InstrumentKind::Margin => {
                if let Some(margin_positions) = &positions.margin_pos {
                    if let Some(position) = margin_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(PositionKind::LeveragedToken(position.clone())));
                    }
                }
            }
        }

        Ok(None) // 没有找到对应的仓位
    }

    /// 更新指定 `Instrument` 的仓位
    pub async fn update_position(&mut self, position: PositionKind) -> Result<(), ExecutionError>
    {
        let mut positions = self.positions.lock().unwrap(); // 获取锁

        match position {
            | PositionKind::Perpetual(pos) => {
                // 检查是否存在当前账户的 `perpetual_pos`，即是否有任何永续合约仓位
                if let Some(perpetual_positions) = &mut positions.perpetual_pos {
                    // 尝试在现有的永续合约仓位中找到与传入的 `pos` 相同的 `instrument`（金融工具）
                    if let Some(existing_pos) = perpetual_positions.iter_mut().find(|p| p.meta.instrument == pos.meta.instrument) {
                        // 如果找到了相同的 `instrument`，则更新现有仓位为传入的 `pos`
                        *existing_pos = pos;
                    }
                    else {
                        // 如果没有找到相同的 `instrument`，则将新的仓位 `pos` 添加到永续合约仓位列表中
                        perpetual_positions.push(pos);
                    }
                }
                else {
                    // 如果 `perpetual_pos` 为空，则初始化一个新的包含 `pos` 的永续合约仓位列表
                    positions.perpetual_pos = Some(vec![pos]);
                }
                Ok(())
            }
            | PositionKind::Future(_) => {
                // TODO: Implement the update logic for Future positions
                todo!("[UniLink_Execution] : Updating Future positions is not yet implemented")
            }
            | PositionKind::Option(_) => {
                // TODO: Implement the update logic for Option positions
                todo!("[UniLink_Execution] : Updating Option positions is not yet implemented")
            }
            | PositionKind::LeveragedToken(_) => {
                // TODO: Implement the update logic for Margin positions
                todo!("[UniLink_Execution] : Updating Margin positions is not yet implemented")
            }
        }
    }

    /// Check if there is already some position of this instrument in the AccountPositions
    /// need to determine InstrumentKind from the open order first as position types vary
    pub async fn any_position_open(&self, open: &Order<Open>) -> Result<bool, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let balances_read = account_read.balances.read().await; // 创建一个中间变量
            let positions_lock = balances_read.positions.lock(); // 获取锁

            for positions in positions_lock.iter() {
                if positions.has_position(&open.instrument) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    async fn check_position_direction_conflict(&self, instrument: &Instrument, side: Side) -> Result<(), ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let balances_read = account_read.balances.read().await; // 创建一个中间变量
            let positions_lock = balances_read.positions.lock(); // 获取锁

            for positions in positions_lock.iter() {
                match instrument.kind {
                    | InstrumentKind::Spot => {
                        todo!() // not quite needed either
                    }
                    | InstrumentKind::Perpetual => {
                        if let Some(perpetual_positions) = &positions.perpetual_pos {
                            for pos in perpetual_positions {
                                if pos.meta.instrument == *instrument && pos.meta.side != side {
                                    return Err(ExecutionError::InvalidDirection);
                                }
                            }
                        }
                    }
                    | InstrumentKind::Future => {
                        if let Some(futures_positions) = &positions.futures_pos {
                            for pos in futures_positions {
                                if pos.meta.instrument == *instrument && pos.meta.side != side {
                                    return Err(ExecutionError::InvalidDirection);
                                }
                            }
                        }
                    }
                    | InstrumentKind::Option => {
                        if let Some(option_positions) = &positions.option_pos {
                            for pos in option_positions {
                                if pos.meta.instrument == *instrument && pos.meta.side != side {
                                    return Err(ExecutionError::InvalidDirection);
                                }
                            }
                        }
                    }
                    | InstrumentKind::Margin => {
                        if let Some(margin_positions) = &positions.margin_pos {
                            for pos in margin_positions {
                                if pos.meta.instrument == *instrument && pos.meta.side != side {
                                    return Err(ExecutionError::InvalidDirection);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub async fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> Result<AccountEvent, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let position_mode = self.determine_position_mode().await?;
            let position_margin_mode = account.read().await.config.position_margin_mode.clone();

            // 前置检查 InstrumentKind 和 NetMode 方向
            match open.instrument.kind {
                | InstrumentKind::Spot => {
                    todo!("[UniLink_Execution] : Spot handling is not implemented yet");
                }
                | InstrumentKind::Option => {
                    todo!("[UniLink_Execution] : Option handling is not implemented yet");
                }
                | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::Margin => {
                    if position_mode == PositionDirectionMode::NetMode {
                        self.check_position_direction_conflict(&open.instrument, open.side).await?;
                    }
                }
            }

            // 更新余额，根据不同的 PositionMarginMode 处理
            match (open.instrument.kind.clone(), position_margin_mode) {
                | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::Margin, PositionMarginMode::Cross) => {
                    // FIXME: NOTE this is DEMONSTRATIVE AND PROBLEMATIC and the common pool is yet to be built.
                    // Cross margin: apply the required balance to a common pool
                    todo!()
                }
                | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::Margin, PositionMarginMode::Isolated) => {
                    // Isolated margin: apply changes to the specific position's margin
                    match open.side {
                        | Side::Buy => {
                            let delta = BalanceDelta { total: 0.0,
                                                       available: -required_balance };
                            self.update(&open.instrument.quote, delta);
                            // position 中增加 deposited_margin
                        }
                        | Side::Sell => {
                            let delta = BalanceDelta { total: 0.0,
                                                       available: -required_balance };
                            self.update(&open.instrument.base, delta);
                            // position 中增加 deposited_margin
                        }
                    }
                }
                // 其他情况下，继续处理，当前返回错误
                | (_, _) => {
                    return Err(ExecutionError::Simulated(format!(
                        "[UniLink_Execution] : Unsupported InstrumentKind or PositionMarginMode for open order: {:?}",
                        open.instrument.kind
                    )));
                }
            };

            let updated_balance = match open.side {
                | Side::Buy => self.balance(&open.instrument.quote)?.clone(),
                | Side::Sell => self.balance(&open.instrument.base)?.clone(),
            };

            Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp").into(),
                              exchange: ExchangeVariant::Simulated,
                              kind: AccountEventKind::Balance(TokenBalance::new(open.instrument.quote.clone(), updated_balance)) })
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> TokenBalance
    {
        match cancelled.side {
            | Side::Buy => {
                let balance = self.balance_mut(&cancelled.instrument.quote)
                                  .expect("[UniLink_Execution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&cancelled.instrument.base)
                                  .expect("[UniLink_Execution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// 从交易中更新余额并返回 [`AccountEvent`]
    /// NOTE 注意[ClickhouseTrade]行情数据和此处所需Trade是否兼容。
    /// NOTE this is currently buggy!
    pub async fn update_from_trade(&mut self, market_event: &MarketEvent<ClickhouseTrade>) -> Result<AccountEvent, ExecutionError>
    {
        let Instrument { base, quote, kind, .. } = &market_event.instrument;
        let fee = self.get_fee(kind).await.unwrap_or(0.0);
        let side = market_event.kind.parse_side();

        match kind {
            | InstrumentKind::Spot => {
                todo!("Spot handling is not implemented yet");
            }
            | InstrumentKind::Option => {
                todo!("Option handling is not implemented yet");
            }
            | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::Margin => {
                let (base_delta, quote_delta) = match side {
                    | Side::Buy => {
                        let base_increase = market_event.kind.amount - fee;
                        // Note: available was already decreased by the opening of the Side::Buy order
                        let base_delta = BalanceDelta { total: base_increase,
                                                        available: base_increase };
                        let quote_delta = BalanceDelta { total: -market_event.kind.amount * market_event.kind.price,
                                                         available: 0.0 };
                        (base_delta, quote_delta)
                    }
                    | Side::Sell => {
                        // Note: available was already decreased by the opening of the Side::Sell order
                        let base_delta = BalanceDelta { total: -market_event.kind.amount,
                                                        available: 0.0 };
                        let quote_increase = (market_event.kind.amount * market_event.kind.price) - fee;
                        let quote_delta = BalanceDelta { total: quote_increase,
                                                         available: quote_increase };
                        (base_delta, quote_delta)
                    }
                };

                let base_balance = self.update(base, base_delta);
                let quote_balance = self.update(quote, quote_delta);

                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp").into(),
                                  exchange: ExchangeVariant::Simulated,
                                  kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance)]) })
            }
        }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl<Event> Deref for AccountState<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target
    {
        &self.balances
    }
}

impl<Event> DerefMut for AccountState<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.balances
    }
}
