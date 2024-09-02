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
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub struct AccountState
{
    pub balances: HashMap<Token, Balance>,
    pub positions: AccountPositions,
    pub account_ref: Weak<Mutex<Account>>, // NOTE :如果不使用弱引用，可能会导致循环引用和内存泄漏。
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
        Self {
            balances,
            positions,
            account_ref: Weak::new(),
        }
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
    pub async fn get_fee(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExecutionError> {
        if let Some(account) = self.account_ref.upgrade() {
            // 直接访问 account 的 config 字段
            let commission_rates = account.lock().await.config
                .fees_book
                .get(instrument_kind)
                .cloned()
                .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange is not configured for InstrumentKind: {:?}", instrument_kind)))?;

            match role {
                OrderRole::Maker => Ok(commission_rates.maker_fees),
                OrderRole::Taker => Ok(commission_rates.taker_fees),
            }
        } else {
            Err(ExecutionError::SandBox("Account reference is not set".to_string()))
        }
    }

    pub async fn get_exchange_ts(&self) -> Result<i64, ExecutionError> {
        if let Some(account) = self.account_ref.upgrade() {
            // 直接访问 account 的 exchange_timestamp 字段
            let exchange_ts = account.lock().await.exchange_timestamp.load(Ordering::SeqCst);
            Ok(exchange_ts)
        } else {
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
        } else {
            Err(ExecutionError::InsufficientBalance(token.clone()))
        }
    }
    #[allow(dead_code)]

    async fn determine_position_mode(&self) -> Result<PositionDirectionMode, ExecutionError> {
        if let Some(account) = self.account_ref.upgrade() {
            // 直接访问 account 的 config 字段
            let position_mode = account.lock().await.config.position_mode.clone();
            Ok(position_mode)
        } else {
            Err(ExecutionError::SandBox("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// 判断Account的当前保证金模式。
    #[allow(dead_code)]
    async fn determine_margin_mode(&self) -> Result<MarginMode, ExecutionError> {
        if let Some(account) = self.account_ref.upgrade() {
            // 直接访问 account 的 config 字段
            Ok(account.lock().await.config.margin_mode.clone())
        } else {
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
        } else {
            // 如果没有找到相同的 `instrument`，将新的仓位添加到永续合约仓位列表中
            perpetual_positions.push(pos);
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
    pub async fn apply_open_order_changes(
        &mut self,
        open: &Order<Open>,
        required_balance: f64,
    ) -> Result<AccountEvent, ExecutionError> {
        println!(
            "[UniLink_Execution] : Starting apply_open_order_changes: {:?}, with balance: {:?}",
            open,
            required_balance
        );

        let timeout_duration = Duration::from_secs(1);
        let result = timeout(timeout_duration, async {
            let account_arc = match self.account_ref.upgrade() {
                Some(account_arc) => {
                    println!("[UniLink_Execution] : Account reference is set");
                    account_arc },
                None => {
                    eprintln!("Failed to upgrade account_ref, it is None.");
                    return Err(ExecutionError::SandBox(
                        "[UniLink_Execution] : Account reference is not set".to_string(),
                    ));
                }
            };

            let config = &account_arc.lock().await.config;

            let (position_mode, position_margin_mode) = (
                config.position_mode.clone(),
                config.position_margin_mode.clone(),
            );

            println!(
                "[UniLink_Execution] : Retrieved position_mode: {:?}, position_margin_mode: {:?}",
                position_mode,
                position_margin_mode
            );

            match open.instrument.kind {
                InstrumentKind::Spot => {
                    todo!("[UniLink_Execution] : Spot handling is not implemented yet");
                }
                InstrumentKind::CryptoOption => {
                    todo!("[UniLink_Execution] : Option handling is not implemented yet");
                }
                InstrumentKind::CommodityFuture => {
                    todo!("[UniLink_Execution] : Commodity future handling is not implemented yet");
                }
                InstrumentKind::CommodityOption => {
                    todo!("[UniLink_Execution] : Commodity option handling is not implemented yet");
                }
                InstrumentKind::Perpetual
                | InstrumentKind::Future
                | InstrumentKind::CryptoLeveragedToken => {
                    if position_mode == PositionDirectionMode::NetMode {
                        self.check_position_direction_conflict(&open.instrument, open.side)
                            .await?;
                    }
                }
            }

            match (
                open.instrument.kind,
                position_margin_mode,
            ) {
                (
                    InstrumentKind::Perpetual
                    | InstrumentKind::Future
                    | InstrumentKind::CryptoLeveragedToken,
                    PositionMarginMode::Cross,
                ) => {
                    todo!("Handle Cross Margin");
                }
                (
                    InstrumentKind::Perpetual
                    | InstrumentKind::Future
                    | InstrumentKind::CryptoLeveragedToken,
                    PositionMarginMode::Isolated,
                ) => match open.side {
                    Side::Buy => {
                        let delta = BalanceDelta {
                            total: 0.0,
                            available: -required_balance,
                        };
                        self.apply_balance_delta(&open.instrument.quote, delta);
                    }
                    Side::Sell => {
                        let delta = BalanceDelta {
                            total: 0.0,
                            available: -required_balance,
                        };
                        self.apply_balance_delta(&open.instrument.base, delta);
                    }
                },
                (_, _) => {
                    return Err(ExecutionError::SandBox(format!(
                        "[UniLink_Execution] : Unsupported InstrumentKind or PositionMarginMode for open order: {:?}",
                        open.instrument.kind
                    )));
                }
            };

            let updated_balance = match open.side {
                Side::Buy => *self.balance(&open.instrument.quote)?,
                Side::Sell => *self.balance(&open.instrument.base)?,
            };

            Ok(AccountEvent {
                exchange_timestamp: self
                    .get_exchange_ts()
                    .await
                    .expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                exchange: Exchange::SandBox,
                kind: AccountEventKind::Balance(TokenBalance::new(
                    open.instrument.quote.clone(),
                    updated_balance,
                )),
            })
        })
            .await;

        result.unwrap_or_else(|_| {
            println!("[UniLink_Execution] : apply_open_order_changes timed out");
            Err(ExecutionError::SandBox(
                "apply_open_order_changes timed out".to_string(),
            ))
        })
    }    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
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
                        let base_delta = BalanceDelta {
                            total: base_increase,
                            available: base_increase,
                        };
                        let quote_delta = BalanceDelta {
                            total: -trade.quantity * trade.price,
                            available: 0.0,
                        };
                        (base_delta, quote_delta)
                    }
                    | Side::Sell => {
                        // Note: available was already decreased by the opening of the Side::Sell order
                        let base_delta = BalanceDelta {
                            total: -trade.quantity,
                            available: 0.0,
                        };
                        let quote_increase = (trade.quantity * trade.price) - fee;
                        let quote_delta = BalanceDelta {
                            total: quote_increase,
                            available: quote_increase,
                        };
                        (base_delta, quote_delta)
                    }
                };

                let base_balance = self.apply_balance_delta(base, base_delta);
                let quote_balance = self.apply_balance_delta(quote, quote_delta);

                Ok(AccountEvent {
                    exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                    exchange: Exchange::SandBox,
                    kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance), ]),
                })
            }
        }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn apply_balance_delta(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.balance_mut(token).unwrap();

        let _ = base_balance.apply(delta);

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
            instrument::kind::InstrumentKind,
            order::OrderRole,
            position::AccountPositions,
            token::Token,
        },
        sandbox::account::account_config::{CommissionRates, MarginMode},
        test_utils::*,
    };
    use std::{collections::HashMap, sync::Arc};

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
        let token1 = Token::from("TEST_BASE");
        let token2 = Token::from("TEST_QUOTE");

        // Create a mock balance map and populate it
        let mut balances = HashMap::new();
        balances.insert(token1.clone(), Balance::new(100.0, 50.0, 1.0));
        balances.insert(token2.clone(), Balance::new(200.0, 150.0, 1.0));

        // Create a mock positions structure with all positions set to None
        let positions = AccountPositions {
            margin_pos: Vec::new(),
            perpetual_pos: Vec::new(),
            futures_pos: Vec::new(),
            option_pos: Vec::new(),
        };

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
    async fn test_get_fee() {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates {
            maker_fees: 0.001,
            taker_fees: 0.002,
        };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 创建 Account 并包装在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        {
            // 更新 account_state 的 account_ref
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account_arc);
        } // 释放锁

        // 直接调用 get_fee 方法，不需要再锁定 account_arc
        let fee_result = account_state.lock().await.get_fee(&InstrumentKind::Perpetual, OrderRole::Maker).await;

        if let Err(e) = &fee_result {
            println!("Error: {:?}", e);
        }

        assert!(fee_result.is_ok());
        assert_eq!(fee_result.unwrap(), 0.001); // 确保你检查的是插入的 perpetual_open 费率
    }

    #[tokio::test]
    async fn test_determine_position_mode() {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates {
            maker_fees: 0.0,
            taker_fees: 0.0,
        };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 创建 Account 并包装在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account_arc);
        }

        // 调用 determine_position_mode 方法
        let position_mode_result = account_state.lock().await.determine_position_mode().await;

        assert!(position_mode_result.is_ok());
        assert_eq!(position_mode_result.unwrap(), PositionDirectionMode::NetMode);
    }
    #[tokio::test]
    async fn test_determine_margin_mode() {
        let account_state = create_test_account_state().await;

        // 创建一个新的 AccountConfig 并手动设置 fees_book
        let mut config = create_test_account_config();

        // 设置 CommissionRates 并插入到 fees_book 中
        let commission_rates = CommissionRates {
            maker_fees: 0.0,
            taker_fees: 0.0,
        };
        config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

        // 创建 Account 并将其包装在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account_arc);
        }

        let margin_mode_result = account_state.lock().await.determine_margin_mode().await;

        assert!(margin_mode_result.is_ok());
        assert_eq!(margin_mode_result.unwrap(), MarginMode::SingleCurrencyMargin);
    }

    #[tokio::test]
    async fn test_set_position() {
        let account_state = create_test_account_state().await;
        let instrument = create_test_instrument(InstrumentKind::Perpetual);
        let perpetual_position = create_test_perpetual_position(instrument.clone());

        // 创建 Account 并将其包装在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        // 更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account_arc);
        }

        // 将 Position 设置到 account_state 中
        account_state
            .lock()
            .await
            .set_position(Position::Perpetual(perpetual_position.clone()))
            .await
            .unwrap();

        // 确保使用相同的 Instrument
        let position_result = account_state.lock().await.get_position(&instrument).await;

        assert!(position_result.is_ok());
        assert!(position_result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_upgrade_account_ref() {
        // 创建 AccountState 实例
        let account_state = create_test_account_state().await;

        // 创建 Account 实例，并将其包裹在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        // 手动更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = Arc::downgrade(&account_arc);
        }

        // 测试 account_ref 是否能够成功升级
        {
            let state = account_state.lock().await;
            match state.account_ref.upgrade() {
                Some(upgraded_account) => {
                    let upgraded_account = upgraded_account.lock().await;
                    println!("Successfully upgraded account_ref!");
                    // 验证升级后的 account_ref 是否指向正确的 Account 实例
                    assert_eq!(upgraded_account.machine_id, account_arc.lock().await.machine_id);
                    println!("machine_id is correct and matches the original account.");
                }
                None => {
                    println!("Failed to upgrade account_ref, it is None.");
                    panic!("account_ref upgrade failed!");
                }
            }
        }
    }
    #[tokio::test]
    async fn test_downgrade_and_upgrade_account_ref() {
        // 创建 AccountState 实例
        let account_state = create_test_account_state().await;

        // 创建 Account 实例，并将其包裹在 Arc<Mutex<Account>> 中
        let account_arc = Arc::new(Mutex::new(create_test_account().await));

        // 手动将 account_arc 降级为 Weak
        let weak_account = Arc::downgrade(&account_arc);

        // 测试升级降级后的 Weak 是否能够成功升级
        match weak_account.upgrade() {
            Some(upgraded_account) => {
                let upgraded_account = upgraded_account.lock().await;
                println!("Successfully upgraded from Weak to Arc!");
                // 验证升级后的 account_ref 是否指向正确的 Account 实例
                assert_eq!(upgraded_account.machine_id, account_arc.lock().await.machine_id);
                println!("machine_id is correct and matches the original account.");
            }
            None => {
                println!("Failed to upgrade from Weak, it is None.");
                panic!("account_ref upgrade failed!");
            }
        }

        // 手动更新 account_state 的 account_ref
        {
            let mut account_state_locked = account_state.lock().await;
            account_state_locked.account_ref = weak_account.clone();
        }

        // 再次测试 account_ref 是否能够成功升级
        {
            let state = account_state.lock().await;
            match state.account_ref.upgrade() {
                Some(upgraded_account) => {
                    let upgraded_account = upgraded_account.lock().await;
                    println!("Successfully upgraded account_ref from AccountState!");
                    // 验证升级后的 account_ref 是否指向正确的 Account 实例
                    assert_eq!(upgraded_account.machine_id, account_arc.lock().await.machine_id);
                    println!("machine_id is correct and matches the original account.");
                }
                None => {
                    println!("Failed to upgrade account_ref from AccountState, it is None.");
                    panic!("account_ref upgrade from AccountState failed!");
                }
            }
        }
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
            account_state.lock().await.positions.perpetual_pos = vec![create_test_perpetual_position(instrument.clone()), ];

            let result = account_state.lock().await.check_position_direction_conflict(&instrument, Side::Sell).await;
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);

            // 情况3：模拟不存在冲突的Future仓位
            let instrument_future = create_test_instrument(InstrumentKind::Future);
            let result = account_state.lock().await.check_position_direction_conflict(&instrument_future, Side::Buy).await;
            assert!(result.is_ok());

            // 情况4：模拟存在冲突的Future仓位，注意这里 `side` 是 `Sell`
            account_state.lock().await.positions.futures_pos = vec![create_test_future_position_with_side(instrument_future.clone(), Side::Sell), ];

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
