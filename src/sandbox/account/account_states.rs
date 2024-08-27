use crate::{
    common::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{states::open::Open, Order, OrderRole},
        position,
        position::{leveraged_token, option, perpetual::PerpetualPosition, AccountPositions, Position, PositionDirectionMode, PositionMarginMode},
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::account::{account_config::MarginMode, Account},
    Exchange,
};
use future::FuturePosition;
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
pub struct AccountState
{
    pub balances: HashMap<Token, Balance>,
    pub positions: AccountPositions,
    pub account_ref: Weak<Account>, // NOTE :如果不使用弱引用，可能会导致循环引用和内存泄漏。
}

impl PartialEq for AccountState
{
    fn eq(&self, other: &Self) -> bool
    {
        self.balances == other.balances
    }
}

impl AccountState
{
    pub fn new(balances: HashMap<Token, Balance>, positions: AccountPositions) -> Self
    {
        Self { balances,
               positions,
               account_ref: Weak::new() }
    }

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

    pub async fn get_fee(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account;
            let commission_rates = account_read.config
                                               .fees_book
                                               .get(instrument_kind)
                                               .cloned()
                                               .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for InstrumentKind: {:?}", instrument_kind)))?;

            match role {
                | OrderRole::Maker => Ok(commission_rates.maker_fees),
                | OrderRole::Taker => Ok(commission_rates.taker_fees),
            }
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

    /// 获取指定 `Instrument` 的仓位
    pub async fn get_position(&self, instrument: &Instrument) -> Result<Option<Position>, ExecutionError>
    {
        let positions = &self.positions; // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExecutionError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument)));
            }
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

        // 检查账户是否已经有永续合约仓位
        if let Some(perpetual_positions) = &mut positions.perpetual_pos {
            // 尝试查找是否存在与传入 `pos` 相同的 `instrument`
            if let Some(existing_pos) = perpetual_positions.iter_mut().find(|p| p.meta.instrument == pos.meta.instrument) {
                // 如果找到了相同的 `instrument`，则更新现有仓位信息
                *existing_pos = pos;
            }
            else {
                // 如果没有找到相同的 `instrument`，将新的仓位添加到永续合约仓位列表中
                perpetual_positions.push(pos);
            }
        }
        else {
            // 如果账户中没有永续合约仓位，初始化一个新的包含 `pos` 的列表
            positions.perpetual_pos = Some(vec![pos]);
        }

        Ok(())
    }

    /// 更新 FuturePosition 的方法（占位符）
    async fn set_future_position(&mut self, _pos: FuturePosition) -> Result<(), ExecutionError>
    {
        todo!("[UniLink_Execution] : Updating Future positions is not yet implemented")
    }

    /// 更新 OptionPosition 的方法（占位符）
    async fn set_option_position(&mut self, _pos: OptionPosition) -> Result<(), ExecutionError>
    {
        todo!("[UniLink_Execution] : Updating Option positions is not yet implemented")
    }

    /// 更新 LeveragedTokenPosition 的方法（占位符）
    async fn set_leveraged_token_position(&mut self, _pos: LeveragedTokenPosition) -> Result<(), ExecutionError>
    {
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
                return Err(ExecutionError::NotImplemented("Spot position conflict check not implemented".into()));
            }
            | InstrumentKind::CommodityOption => {
                return Err(ExecutionError::NotImplemented("CommodityOption position conflict check not implemented".into()));
            }
            | InstrumentKind::CommodityFuture => {
                return Err(ExecutionError::NotImplemented("CommodityFuture position conflict check not implemented".into()));
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
                    todo!("Handle Cross Margin")
                }
                | (InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken, PositionMarginMode::Isolated) => {
                    // Isolated margin: apply changes to the specific position's margin
                    match open.side {
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
                              exchange: Exchange::SandBox,
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

                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                                  exchange: Exchange::SandBox,
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

impl Deref for AccountState
{
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target
    {
        &self.balances
    }
}

impl DerefMut for AccountState
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.balances
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{
            balance::Balance,
            event::ClientOrderId,
            friction::{Fees, FutureFees, PerpetualFees},
            instrument::{kind::InstrumentKind, Instrument},
            order::{order_instructions::OrderInstruction, OrderId, OrderRole},
            position::{future::FuturePositionConfig, perpetual::PerpetualPositionConfig, position_meta::PositionMeta, AccountPositions},
            token::Token,
        },
        sandbox::account::{
            account_config::{AccountConfig, CommissionLevel, CommissionRates, MarginMode},
            account_latency::{AccountLatency, FluctuationMode},
            account_orders::AccountOrders,
            Account,
        },
    };
    use std::{
        collections::HashMap,
        sync::{atomic::AtomicI64, Arc, Weak},
    };
    use tokio::sync::Mutex; // 确保使用 tokio 的 Mutex
    use tokio::sync::{mpsc, RwLock};
    use uuid::Uuid;

    fn create_test_instrument(kind: InstrumentKind) -> Instrument
    {
        Instrument { base: Token::from("BTC"),
                     quote: Token::from("USDT"),
                     kind }
    }
    fn create_test_account_config() -> AccountConfig
    {
        let leverage_rate = 1.0;

        AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
                        position_mode: PositionDirectionMode::NetMode,
                        position_margin_mode: PositionMarginMode::Isolated,
                        commission_level: CommissionLevel::Lv1,
                        // current_commission_rate: CommissionRates { maker_fees: 0.001,
                        //                                            taker_fees: 0.0015 },
                        funding_rate: 0.0,
                        account_leverage_rate: leverage_rate,
                        fees_book: HashMap::new() }
    }
    async fn create_test_account_state() -> Arc<Mutex<AccountState>>
    {
        let balances = HashMap::new();
        let positions = AccountPositions { margin_pos: None,
                                           perpetual_pos: None,
                                           futures_pos: None,
                                           option_pos: None };

        let account_config = create_test_account_config();

        // 初始化 AccountState，但不要立即设置 account_ref
        let account_state = AccountState { balances: balances.clone(),
                                           positions: positions.clone(),
                                           account_ref: Weak::new() };

        // 创建 Account 实例
        let account_state_arc = Arc::new(Mutex::new(account_state.clone()));

        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(0),
                                         // data: Arc::new(RwLock::new(AccountDataStreams::default())),
                                         account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
                                         config: Arc::new(account_config),
                                         states: account_state_arc.clone(), // 使用克隆后的 Arc<Mutex<...>>
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });

        // 更新 account_ref，使其指向 Account
        {
            let mut account_state_locked = account_state_arc.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }

        // 返回 AccountState 被 Arc<Mutex<...>> 包装
        account_state_arc
    }

    fn create_test_perpetual_position(instrument: Instrument) -> PerpetualPosition
    {
        PerpetualPosition { meta: PositionMeta { position_id: "test_position".to_string(),
                                                 enter_ts: 0,
                                                 update_ts: 0,
                                                 exit_balance: TokenBalance { token: instrument.base.clone(),
                                                                              balance: Balance { current_price: 0.0,
                                                                                                 total: 0.0,
                                                                                                 available: 0.0 } },
                                                 exchange: Exchange::SandBox,
                                                 instrument,
                                                 side: Side::Buy,
                                                 current_size: 1.0,
                                                 current_fees_total: Fees::Perpetual(PerpetualFees { maker_fee: 0.0,
                                                                                                     taker_fee: 0.0,
                                                                                                     funding_fee: 0.0 }),
                                                 current_avg_price_gross: 0.0,
                                                 current_symbol_price: 0.0,
                                                 current_avg_price: 0.0,
                                                 unrealised_pnl: 0.0,
                                                 realised_pnl: 0.0 },
                            pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                                  leverage: 1.0,
                                                                  position_mode: PositionDirectionMode::LongShortMode },
                            liquidation_price: 0.0,
                            margin: 0.0 }
    }

    fn create_test_future_position_with_side(instrument: Instrument, side: Side) -> FuturePosition
    {
        FuturePosition { meta: PositionMeta { position_id: Uuid::new_v4().to_string(),
                                              enter_ts: 0,
                                              update_ts: 0,
                                              exit_balance: TokenBalance { token: instrument.base.clone(),
                                                                           balance: Balance { current_price: 0.0,
                                                                                              total: 0.0,
                                                                                              available: 0.0 } },
                                              exchange: Exchange::SandBox,
                                              instrument: instrument.clone(),
                                              side,
                                              current_size: 0.0,
                                              current_fees_total: Fees::Future(FutureFees { maker_fee: 0.0,
                                                                                            taker_fee: 0.0,
                                                                                            funding_fee: 0.0 }),
                                              current_avg_price_gross: 0.0,
                                              current_symbol_price: 0.0,
                                              current_avg_price: 0.0,
                                              unrealised_pnl: 0.0,
                                              realised_pnl: 0.0 },
                         pos_config: FuturePositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                            leverage: 1.0,
                                                            position_mode: PositionDirectionMode::LongShortMode },
                         liquidation_price: 0.0,
                         margin: 0.0,
                         funding_fee: 0.0 }
    }

    #[tokio::test]
    async fn test_balance()
    {
        let token = Token::from("SOL");
        let mut balances = HashMap::new();
        balances.insert(token.clone(), Balance::new(100.0, 50.0, 1.0));

        let positions = AccountPositions::init(); // 假设有一个用于初始化 AccountPositions 的方法
        let account_state = AccountState::new(balances, positions);

        let balance = account_state.balance(&token).unwrap();
        assert_eq!(balance.total, 100.0);
        assert_eq!(balance.available, 50.0);
    }

    #[tokio::test]
    async fn test_balance_mut()
    {
        let token = Token::from("TEST");
        let mut balances = HashMap::new();
        balances.insert(token.clone(), Balance::new(100.0, 50.0, 1.0));

        let positions = AccountPositions::init();
        let mut account_state = AccountState::new(balances, positions);

        let balance_mut = account_state.balance_mut(&token).unwrap();
        balance_mut.available -= 10.0;
        assert_eq!(balance_mut.available, 40.0);

        let balance = account_state.balance(&token).unwrap();
        assert_eq!(balance.available, 40.0);
    }

    #[tokio::test]
    async fn test_has_sufficient_available_balance()
    {
        let token = Token::from("TEST");
        let mut balances = HashMap::new();
        balances.insert(token.clone(), Balance::new(100.0, 50.0, 1.0));

        let positions = AccountPositions::init();
        let account_state = AccountState::new(balances, positions);

        assert!(account_state.has_sufficient_available_balance(&token, 40.0).is_ok());
        assert!(account_state.has_sufficient_available_balance(&token, 60.0).is_err());
    }

    #[tokio::test]
    async fn test_apply_balance_delta()
    {
        let token = Token::from("SOL");
        let mut balances = HashMap::new();
        balances.insert(token.clone(), Balance::new(100.0, 50.0, 1.0));

        let positions = AccountPositions::init();
        let mut account_state = AccountState::new(balances, positions);

        let delta = BalanceDelta::new(0.0, -10.0);
        let balance = account_state.apply_balance_delta(&token, delta);

        assert_eq!(balance.total, 100.0);
        assert_eq!(balance.available, 40.0);
    }

    #[tokio::test]
    async fn test_fetch_all_balances()
    {
        // Define tokens for testing
        let token1 = Token::from("TEST1");
        let token2 = Token::from("TEST2");

        // Create a mock balance map and populate it
        let mut balances = HashMap::new();
        balances.insert(token1.clone(), Balance::new(100.0, 50.0, 1.0));
        balances.insert(token2.clone(), Balance::new(200.0, 150.0, 1.0));

        // Create a mock positions structure with all positions set to None
        let positions = AccountPositions { margin_pos: None,
                                           perpetual_pos: None,
                                           futures_pos: None,
                                           option_pos: None };

        // Instantiate the account state with the balances and positions
        let account_state = AccountState::new(balances, positions);

        // Fetch all balances from the account state
        let all_balances = account_state.fetch_all_balances();

        // Verify that the correct number of balances is returned
        assert_eq!(all_balances.len(), 2, "Expected 2 balances but got {}", all_balances.len());

        // Check that the balances include the expected tokens using rayon for parallel iteration
        assert!(all_balances.iter().any(|b| b.token == token1), "Expected token1 balance not found");
        assert!(all_balances.iter().any(|b| b.token == token2), "Expected token2 balance not found");
    }
    #[tokio::test]
    async fn test_get_fee()
    {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates { maker_fees: 0.001,
                                                 taker_fees: 0.002 };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 更新 account_state 的 account_ref，使其指向新的 AccountConfig
        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(0),
                                         // data: Arc::new(RwLock::new(AccountDataStreams::default())),
                                         account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
                                         config: Arc::new(config),
                                         states: account_state.clone(),
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }

        // 解锁并调用 get_fee 方法
        let fee_result = account_state.lock().await.get_fee(&InstrumentKind::Perpetual, OrderRole::Maker).await;

        if let Err(e) = &fee_result {
            println!("Error: {:?}", e);
        }

        assert!(fee_result.is_ok());
        assert_eq!(fee_result.unwrap(), 0.001); // 确保你检查的是插入的 perpetual_open 费率
    }

    #[tokio::test]
    async fn test_get_exchange_ts()
    {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates { maker_fees: 0.0, taker_fees: 0.0 };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 更新 account_state 的 account_ref，使其指向新的 AccountConfig
        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(123456789), // 设置一个非零的初始时间戳值
                                         account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
                                         config: Arc::new(config),
                                         states: account_state.clone(),
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }

        // 获取 exchange timestamp
        let exchange_ts_result = account_state.lock().await.get_exchange_ts().await;

        // 检查结果
        assert!(exchange_ts_result.is_ok());
        assert_eq!(exchange_ts_result.unwrap(), 123456789); // 确保测试的初始值与预期一致
    }

    #[tokio::test]
    async fn test_determine_position_mode()
    {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates { maker_fees: 0.0, taker_fees: 0.0 };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 更新 account_state 的 account_ref，使其指向新的 AccountConfig
        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(123456789), // 设置一个非零的初始时间戳值
                                         // data: Arc::new(RwLock::new(AccountDataStreams::default())),
                                         account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
                                         config: Arc::new(config),
                                         states: account_state.clone(),
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }

        let position_mode_result = account_state.lock().await.determine_position_mode().await;

        assert!(position_mode_result.is_ok());
        assert_eq!(position_mode_result.unwrap(), PositionDirectionMode::NetMode);
    }

    #[tokio::test]
    async fn test_determine_margin_mode()
    {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates { maker_fees: 0.0, taker_fees: 0.0 };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 更新 account_state 的 account_ref，使其指向新的 AccountConfig
        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(123456789), // 设置一个非零的初始时间戳值
                                         // data: Arc::new(RwLock::new(AccountDataStreams::default())),
                                         account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
                                         config: Arc::new(config),
                                         states: account_state.clone(),
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });
        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }
        let margin_mode_result = account_state.lock().await.determine_margin_mode().await;

        assert!(margin_mode_result.is_ok());
        assert_eq!(margin_mode_result.unwrap(), MarginMode::SingleCurrencyMargin);
    }

    #[tokio::test]

    async fn test_set_position()
    {
        let account_state = create_test_account_state().await;

        let config = create_test_account_config();

        let perpetual_position = PerpetualPosition { meta: PositionMeta { position_id: "".to_string(),
                                                                          enter_ts: 0,
                                                                          update_ts: 0,
                                                                          exit_balance: TokenBalance { token: "SOL".into(),
                                                                                                       balance: Balance { current_price: 0.0,
                                                                                                                          total: 0.0,
                                                                                                                          available: 0.0 } },
                                                                          exchange: Exchange::SandBox,
                                                                          instrument: create_test_instrument(InstrumentKind::Perpetual),
                                                                          side: Side::Buy,
                                                                          current_size: 0.0,
                                                                          current_fees_total: Fees::Perpetual(PerpetualFees { maker_fee: 0.0,
                                                                                                                              taker_fee: 0.0,
                                                                                                                              funding_fee: 0.0 }),
                                                                          current_avg_price_gross: 0.0,
                                                                          current_symbol_price: 0.0,
                                                                          current_avg_price: 0.0,
                                                                          unrealised_pnl: 0.0,
                                                                          realised_pnl: 0.0 },
                                                     pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                                                           leverage: 0.0,
                                                                                           position_mode: PositionDirectionMode::LongShortMode },
                                                     liquidation_price: 0.0,
                                                     margin: 0.0 };

        let account = Arc::new(Account { exchange_timestamp: AtomicI64::new(123456789),
                                         // data: Arc::new(RwLock::new(AccountDataStreams::default())),
                                         account_event_tx: mpsc::unbounded_channel().0,
                                         config: Arc::new(config),
                                         states: account_state.clone(),
                                         orders: Arc::new(RwLock::new(AccountOrders::new(vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                  maximum: 0,
                                                                                                                  minimum: 0,
                                                                                                                  current_value: 0 }).await)) });

        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account);
        }

        account_state.lock().await.set_position(Position::Perpetual(perpetual_position.clone())).await.unwrap();

        let instrument = perpetual_position.meta.instrument.clone(); // 确保使用相同的 Instrument
        let position_result = account_state.lock().await.get_position(&instrument).await;

        // if let Ok(Some(position)) = &position_result {
        //     println!("Position found: {:?}", position);
        // }
        // else {
        //     println!("Position not found or error occurred.");
        // }

        assert!(position_result.is_ok());
        assert!(position_result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_any_position_open()
    {
        let account_state = create_test_account_state().await;
        let instrument = create_test_instrument(InstrumentKind::Perpetual);

        // 正确地创建一个 ClientOrderId
        let client_order_id = ClientOrderId(Uuid::new_v4());

        // 模拟一个 Open 订单
        let open_order = Order::<Open> { kind: OrderInstruction::Market,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         client_ts: 123456789,
                                         client_order_id: client_order_id,
                                         side: Side::Buy,
                                         state: Open { id: OrderId::from("test_order"),
                                                       price: 100.0,
                                                       size: 1.0,
                                                       filled_quantity: 0.0,
                                                       order_role: OrderRole::Maker,
                                                       received_ts: 123456789 } };

        // 在没有任何仓位的情况下调用
        let result = account_state.lock().await.any_position_open(&open_order).await;

        assert_eq!(result.expect("Failed to check position open status"), false);

        // 模拟已有仓位的情况
        account_state.lock().await.positions.perpetual_pos = Some(vec![create_test_perpetual_position(instrument.clone())]);

        let result = account_state.lock().await.any_position_open(&open_order).await;

        assert_eq!(result.expect("Failed to check position open status"), true);
    }

    #[tokio::test]
    async fn test_check_position_direction_conflict()
    {
        let account_state = create_test_account_state().await;
        let instrument = create_test_instrument(InstrumentKind::Perpetual);

        // 情况1：没有冲突的情况下调用
        let result = account_state.lock().await.check_position_direction_conflict(&instrument, Side::Buy).await;
        assert!(result.is_ok());

        // 情况2：模拟存在冲突的Perpetual仓位，注意这里 `side` 是 `Sell`
        account_state.lock().await.positions.perpetual_pos = Some(vec![create_test_perpetual_position(instrument.clone()),]);

        let result = account_state.lock().await.check_position_direction_conflict(&instrument, Side::Sell).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);

        // 情况3：模拟不存在冲突的Future仓位
        let instrument_future = create_test_instrument(InstrumentKind::Future);
        let result = account_state.lock().await.check_position_direction_conflict(&instrument_future, Side::Buy).await;
        assert!(result.is_ok());

        // 情况4：模拟存在冲突的Future仓位，注意这里 `side` 是 `Sell`
        account_state.lock().await.positions.futures_pos = Some(vec![create_test_future_position_with_side(instrument_future.clone(), Side::Sell),]);

        let result = account_state.lock().await.check_position_direction_conflict(&instrument_future, Side::Buy).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);

        // 情况5：其他 InstrumentKind 还没有实现，因此我们只需要检查它们是否返回未实现的错误
        let instrument_spot = create_test_instrument(InstrumentKind::Spot);
        let result = account_state.lock().await.check_position_direction_conflict(&instrument_spot, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));

        let instrument_commodity_future = create_test_instrument(InstrumentKind::CommodityFuture);
        let result = account_state.lock().await.check_position_direction_conflict(&instrument_commodity_future, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));

        let instrument_commodity_option = create_test_instrument(InstrumentKind::CommodityOption);
        let result = account_state.lock().await.check_position_direction_conflict(&instrument_commodity_option, Side::Buy).await;
        assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));
    }
}
