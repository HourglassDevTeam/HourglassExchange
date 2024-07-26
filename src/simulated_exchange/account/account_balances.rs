use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{
    common_skeleton::{
        balance::{Balance, BalanceDelta, TokenBalance},
        datafeed::event::MarketEvent,
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{Open, Order},
        token::Token,
        Side,
    },
    error::ExecutionError,
    simulated_exchange::{account::Account, load_from_clickhouse::queries_operations::ClickhouseTrade},
    ExchangeVariant,
};

#[derive(Clone, Debug)]
pub struct AccountBalances<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord + Ord
{
    pub balance_map: HashMap<Token, Balance>,
    pub account_ref: Arc<RwLock<Account<Event>>>,
}

impl<Event> PartialEq for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn eq(&self, other: &Self) -> bool
    {
        self.balance_map == other.balance_map
        // account_ref 是 Arc<RwLock<>>，一般不会比较其内容
    }
}
// CONSIDER 在哪个环节打上时间戳？
impl<Event> AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError>
    {
        self.get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError>
    {
        self.get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    pub fn set_account(&mut self, account: Arc<RwLock<Account<Event>>>)
    {
        self.account_ref = account;
    }

    /// 获取指定 [`InstrumentKind`] 的手续费。
    pub async fn get_fee(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        let account_read = self.account_ref.read().await;
        let config_read = account_read.config.read().await;
        config_read.fees_book
                   .get(instrument_kind)
                   .cloned()
                   .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for InstrumentKind: {:?}", instrument_kind)))
    }

    // 异步方法来获取 Account 的某个字段
    pub async fn get_exchange_ts(&self) -> Result<i64, ExecutionError>
    {
        let account_read = self.account_ref.read().await;
        Ok(account_read.exchange_timestamp)
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance>
    {
        self.balance_map.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
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

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub async fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> AccountEvent
    {
        let _updated_balance = match open.side {
            | Side::Buy => {
                let balance = self.balance_mut(&open.instrument.quote).expect("[UniLink_Execution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&open.instrument.base).expect("[UniLink_Execution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                       exchange: ExchangeVariant::Simulated,
                       kind: AccountEventKind::Balance(_updated_balance) }
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
    pub async fn update_from_trade(&mut self, market_event: &MarketEvent<ClickhouseTrade>) -> AccountEvent
    {
        // let's start with a destructuring assignment
        let Instrument { base, quote, kind, .. } = &market_event.instrument;

        // 获取手续费
        let fee = self.get_fee(kind).await.unwrap_or(0.0);
        // 将 side 字符串转换为 Side 枚举
        let side = market_event.kind.parse_side();
        // 计算 base 和 quote 余额的变化
        let (base_delta, quote_delta) = match side {
            | Side::Buy => {
                // Base 的总余额和可用余额增加 trade.size 减去 base 的交易费用
                let base_increase = market_event.kind.amount - fee;
                let base_delta = BalanceDelta { total: base_increase,
                                                available: base_increase };

                // Quote 的总余额减少 (trade.size * price)
                // 注意: 可用余额已在买单开单时减少
                let quote_delta = BalanceDelta { total: -market_event.kind.amount * market_event.kind.price,
                                                 available: 0.0 };

                (base_delta, quote_delta)
            }
            | Side::Sell => {
                // Base 的总余额减少 trade.size
                // 注意: 可用余额已在卖单开单时减少
                let base_delta = BalanceDelta { total: -market_event.kind.amount,
                                                available: 0.0 };

                // Quote 的总余额和可用余额增加 (trade.size * price) 减去 quote 的交易费用
                let quote_increase = (market_event.kind.amount * market_event.kind.price) - fee;
                let quote_delta = BalanceDelta { total: quote_increase,
                                                 available: quote_increase };

                (base_delta, quote_delta)
            }
        };

        // 应用 BalanceDelta 并返回更新后的余额
        let _base_balance = self.update(base, base_delta);
        let _quote_balance = self.update(quote, quote_delta);

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                       exchange: ExchangeVariant::Simulated,
                       kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), _base_balance), TokenBalance::new(quote.clone(), _quote_balance),]) }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl<Event> Deref for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target
    {
        &self.balance_map
    }
}

impl<Event> DerefMut for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.balance_map
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common_skeleton::order::OrderKind;

    #[test]
    fn test_check_order_kind_support()
    {
        struct TestCase
        {
            kind: OrderKind,
            expected: Result<(), ExecutionError>,
        }

        let tests = vec![TestCase { // TC0: Market
                                    kind: OrderKind::Market,
                                    expected: Ok(()) },
                         TestCase { // TC1: Limit
                                    kind: OrderKind::Limit,
                                    expected: Ok(()) },
                         TestCase { // TC2: PostOnly
                                    kind: OrderKind::PostOnly,
                                    expected: Ok(()) },
                         TestCase { // TC3: Immediate Or Cancel
                                    kind: OrderKind::ImmediateOrCancel,
                                    expected: Ok(()) },];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = Account::<()>::order_validity_check(test.kind); // Specify the generic type `()`
            match test.expected {
                | Ok(()) => assert!(actual.is_ok(), "TC{} is good", index),
                | Err(_) => assert!(actual.is_err(), "TC{} failed", index),
            }
        }
    }
}
