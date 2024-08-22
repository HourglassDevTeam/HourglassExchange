use crate::common_infrastructure::position;
use crate::common_infrastructure::position::perpetual::PerpetualPosition;
use crate::common_infrastructure::position::{leveraged_token, option};
use crate::{
    common_infrastructure::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{Open, Order},
        position::{AccountPositions, Position, PositionDirectionMode, PositionMarginMode},
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::account::{account_config::MarginMode, Account},
    ExchangeVariant,
};
use future::FuturesPosition;
use leveraged_token::LeveragedTokenPosition;
use option::OptionPosition;
use position::future;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Weak},
};

#[derive(Clone, Debug)]
pub struct AccountState<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord + Ord
{
    pub balances: HashMap<Token, Balance>,
    pub positions: AccountPositions,
    pub account_ref: Weak<Account<Event>>, // NOTE :如果不使用弱引用，可能会导致循环引用和内存泄漏。
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
            .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError>
    {
        self.balances
            .get_mut(token)
            .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for Token: {token}")))
    }

    /// 获取指定 [`InstrumentKind`] 的手续费。
    pub async fn get_fee(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account;
            account_read.config
                        .fees_book
                        .get(instrument_kind)
                        .cloned()
                        .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for InstrumentKind: {:?}", instrument_kind)))
        }
        else {
            Err(ExecutionError::SandBox("Account reference is not set".to_string()))
        }
    }


    pub async fn get_exchange_ts(&self) -> Result<i64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account;
            Ok(account_read.exchange_timestamp.load(Ordering::SeqCst))
        }
        else {
            Err(ExecutionError::SandBox("Account reference is not set".to_string()))
        }
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all_balances(&self) -> Vec<TokenBalance>
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
            let account_read = account;
            Ok(account_read.config.position_mode.clone())
        }
        else {
            Err(ExecutionError::SandBox("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 判断Account的当前保证金模式。
    #[allow(dead_code)]
    async fn determine_margin_mode(&self) -> Result<MarginMode, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account;
            Ok(account_read.config.margin_mode.clone())
        }
        else {
            Err(ExecutionError::SandBox("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 获取指定 `Instrument` 的仓位/
    pub async fn get_position(&self, instrument: &Instrument) -> Result<Option<Position>, ExecutionError>
    {
        let positions = &self.positions; // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => return Err(ExecutionError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument))),
            | InstrumentKind::Perpetual => {
                if let Some(perpetual_positions) = &positions.perpetual_pos {
                    if let Some(position) = perpetual_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(Position::Perpetual(position.clone())));
                    }
                }
            }
            | InstrumentKind::Future => {
                if let Some(futures_positions) = &positions.futures_pos {
                    if let Some(position) = futures_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(Position::Future(position.clone())));
                    }
                }
            }
            | InstrumentKind::CryptoOption => {
                if let Some(option_positions) = &positions.option_pos {
                    if let Some(position) = option_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(Position::Option(position.clone())));
                    }
                }
            }
            | InstrumentKind::CryptoLeveragedToken => {
                if let Some(margin_positions) = &positions.margin_pos {
                    if let Some(position) = margin_positions.iter().find(|pos| pos.meta.instrument == *instrument) {
                        return Ok(Some(Position::LeveragedToken(position.clone())));
                    }
                }
            }
            | InstrumentKind::CommodityOption => {
                todo!() // not quite needed either
            }
            | InstrumentKind::CommodityFuture => {
                todo!() // not quite needed either
            }
        }

        Ok(None) // 没有找到对应的仓位
    }

    /// 更新指定 `Instrument` 的仓位
    pub async fn set_position(&mut self, position: Position) -> Result<(), ExecutionError> {
        match position {
            Position::Perpetual(pos) => {
                self.set_perpetual_position(pos).await
            }
            Position::Future(pos) => {
                self.set_future_position(pos).await
            }
            Position::Option(pos) => {
                self.set_option_position(pos).await
            }
            Position::LeveragedToken(pos) => {
                self.set_leveraged_token_position(pos).await
            }
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
    async fn set_perpetual_position(&mut self, pos: PerpetualPosition) -> Result<(), ExecutionError> {
        // 获取账户的锁，确保在更新仓位信息时没有并发访问的问题
        let positions = &mut self.positions;

        // 检查账户是否已经有永续合约仓位
        if let Some(perpetual_positions) = &mut positions.perpetual_pos {
            // 尝试查找是否存在与传入 `pos` 相同的 `instrument`
            if let Some(existing_pos) = perpetual_positions.iter_mut().find(|p| p.meta.instrument == pos.meta.instrument) {
                // 如果找到了相同的 `instrument`，则更新现有仓位信息
                *existing_pos = pos;
            } else {
                // 如果没有找到相同的 `instrument`，将新的仓位添加到永续合约仓位列表中
                perpetual_positions.push(pos);
            }
        } else {
            // 如果账户中没有永续合约仓位，初始化一个新的包含 `pos` 的列表
            positions.perpetual_pos = Some(vec![pos]);
        }

        Ok(())
    }
    /// 更新 FuturePosition 的方法（占位符）
    async fn set_future_position(&mut self, _pos: FuturesPosition) -> Result<(), ExecutionError> {
        todo!("[UniLink_Execution] : Updating Future positions is not yet implemented")
    }

    /// 更新 OptionPosition 的方法（占位符）
    async fn set_option_position(&mut self, _pos: OptionPosition) -> Result<(), ExecutionError> {
        todo!("[UniLink_Execution] : Updating Option positions is not yet implemented")
    }

    /// 更新 LeveragedTokenPosition 的方法（占位符）
    async fn set_leveraged_token_position(&mut self, _pos: LeveragedTokenPosition) -> Result<(), ExecutionError> {
        todo!("[UniLink_Execution] : Updating Leveraged Token positions is not yet implemented")
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
                todo!()
            }
            | InstrumentKind::CommodityOption => {
                todo!()
            }
            | InstrumentKind::CommodityFuture => {
                todo!()
            }
            | InstrumentKind::Perpetual => {
                if let Some(perpetual_positions) = &positions_lock.perpetual_pos {
                    for pos in perpetual_positions {
                        if pos.meta.instrument == *instrument && pos.meta.side != side {
                            return Err(ExecutionError::InvalidDirection);
                        }
                    }
                }
            }
            | InstrumentKind::Future => {
                if let Some(futures_positions) = &positions_lock.futures_pos {
                    for pos in futures_positions {
                        if pos.meta.instrument == *instrument && pos.meta.side != side {
                            return Err(ExecutionError::InvalidDirection);
                        }
                    }
                }
            }
            | InstrumentKind::CryptoOption => {
                if let Some(option_positions) = &positions_lock.option_pos {
                    for pos in option_positions {
                        if pos.meta.instrument == *instrument && pos.meta.side != side {
                            return Err(ExecutionError::InvalidDirection);
                        }
                    }
                }
            }
            | InstrumentKind::CryptoLeveragedToken => {
                if let Some(margin_positions) = &positions_lock.margin_pos {
                    for pos in margin_positions {
                        if pos.meta.instrument == *instrument && pos.meta.side != side {
                            return Err(ExecutionError::InvalidDirection);
                        }
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
        if let Some(account) = self.account_ref.upgrade() {
            let position_mode = self.determine_position_mode().await?;
            let position_margin_mode = account.config.position_margin_mode.clone();

            // 前置检查 InstrumentKind 和 NetMode 方向
            match open.instrument.kind {
                | InstrumentKind::Spot => {
                    todo!("[UniLink_Execution] : Spot handling is not implemented yet");
                }
                | InstrumentKind::CryptoOption => {
                    todo!("[UniLink_Execution] : Option handling is not implemented yet");
                }
                | InstrumentKind::CommodityFuture => {
                    todo!("[UniLink_Execution] : Commodity future handling is not implemented yet");
                }
                | InstrumentKind::CommodityOption => {
                    todo!("[UniLink_Execution] : Crypto option handling is not implemented yet");
                }
                | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken => {
                    if position_mode == PositionDirectionMode::NetMode {
                        self.check_position_direction_conflict(&open.instrument, open.side).await?;
                    }
                }
            }

            // 更新余额，根据不同的 PositionMarginMode 处理
            match (open.instrument.kind, position_margin_mode) {
                | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken, PositionMarginMode::Cross) => {
                    // FIXME: NOTE this is DEMONSTRATIVE AND PROBLEMATIC and the common pool is yet to be built.
                    // Cross margin: apply the required balance to a common pool
                    todo!()
                }
                | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken, PositionMarginMode::Isolated) => {
                    // Isolated margin: apply changes to the specific position's margin
                    match open.side {
                        | Side::Buy => {
                            let delta = BalanceDelta { total: 0.0,
                                                       available: -required_balance };
                            self.apply_balance_delta(&open.instrument.quote, delta);
                            // position 中增加 deposited_margin
                        }
                        | Side::Sell => {
                            let delta = BalanceDelta { total: 0.0,
                                                       available: -required_balance };
                            self.apply_balance_delta(&open.instrument.base, delta);
                            // position 中增加 deposited_margin
                        }
                    }
                }
                // 其他情况下，继续处理，当前返回错误
                | (_, _) => {
                    return Err(ExecutionError::SandBox(format!(
                        "[UniLink_Execution] : Unsupported InstrumentKind or PositionMarginMode for open order: {:?}",
                        open.instrument.kind
                    )));
                }
            };

            let updated_balance = match open.side {
                | Side::Buy => *self.balance(&open.instrument.quote)?,
                | Side::Sell => *self.balance(&open.instrument.base)?,
            };

            Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                              exchange: ExchangeVariant::SandBox,
                              kind: AccountEventKind::Balance(TokenBalance::new(open.instrument.quote.clone(), updated_balance)) })
        }
        else {
            Err(ExecutionError::SandBox("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn apply_cancel_order_changes(&mut self, cancelled: &Order<Open>) -> TokenBalance
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
    pub async fn apply_trade_changes(&mut self, trade_event: &ClientTrade) -> Result<AccountEvent, ExecutionError>
    {
        let Instrument { base, quote, kind, .. } = &trade_event.instrument;
        let fee = trade_event.fees; // 直接从 TradeEvent 中获取费用
        let side = trade_event.side; // 直接使用 TradeEvent 中的 side

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
                        let base_increase = trade_event.size - fee;
                        // Note: available was already decreased by the opening of the Side::Buy order
                        let base_delta = BalanceDelta { total: base_increase,
                                                        available: base_increase };
                        let quote_delta = BalanceDelta { total: -trade_event.size * trade_event.price,
                                                         available: 0.0 };
                        (base_delta, quote_delta)
                    }
                    | Side::Sell => {
                        // Note: available was already decreased by the opening of the Side::Sell order
                        let base_delta = BalanceDelta { total: -trade_event.size,
                                                        available: 0.0 };
                        let quote_increase = (trade_event.size * trade_event.price) - fee;
                        let quote_delta = BalanceDelta { total: quote_increase,
                                                         available: quote_increase };
                        (base_delta, quote_delta)
                    }
                };

                let base_balance = self.apply_balance_delta(base, base_delta);
                let quote_balance = self.apply_balance_delta(quote, quote_delta);

                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                                  exchange: ExchangeVariant::SandBox,
                                  kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance),]) })
            }
        }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn apply_balance_delta(&mut self, token: &Token, delta: BalanceDelta) -> Balance
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
