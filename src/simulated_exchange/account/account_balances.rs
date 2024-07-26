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
        event::{AccountEvent, AccountEventKind},
        instrument::Instrument,
        order::{Open, Order},
        token::Token,
        trade::Trade,
        Side,
    },
    error::ExecutionError,
    simulated_exchange::account::Account,
    ExchangeVariant,
};
use crate::common_skeleton::datafeed::event::MarketEvent;
use crate::simulated_exchange::load_from_clickhouse::queries_operations::ClickhouseTrade;

#[derive(Clone, Debug)]
pub struct AccountBalances<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord + Ord
{
    pub balance_map: HashMap<Token, Balance>,
    pub account_ref: Option<Arc<RwLock<Account<Event>>>>,
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
        self.account_ref = Some(account);
    }

    // 异步方法来获取 Account 的某个字段
    pub async fn get_exchange_ts(&self) -> Option<i64>
    {
        if let Some(account) = &self.account_ref {
            let account_read = account.read().await;
            Some(account_read.exchange_timestamp)
        }
        else {
            None
        }
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance>
    {
        self.balance_map.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    /// NOTE 这个方法不应该导致panic,Client要能妥善处理这种状况。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError>
    {
        let available = self.balance(token)?.available;
        match available >= required_balance {
            | true => Ok(()),
            | false => Err(ExecutionError::InsufficientBalance(token.clone())),
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

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.unwrap(),
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


    // /// 从交易中更新余额并返回 [`AccountEvent`]
    // pub async fn update_from_trade(&mut self, market_event: &MarketEvent<ClickhouseTrade>) -> AccountEvent {
    //     let Instrument { base, quote, .. } = &market_event.instrument;
    //
    //     // 计算 base 和 quote 余额的变化
    //     let (base_delta, quote_delta) = match market_event.kind.side {
    //         Side::Buy.to_string() => {
    //             // Base 的总余额和可用余额增加 trade.size 减去 base 的交易费用
    //             let base_increase = market_event.kind.amount - market_event.fees;
    //             let base_delta = BalanceDelta {
    //                 total: base_increase,
    //                 available: base_increase,
    //             };
    //
    //             // Quote 的总余额减少 (trade.size * price)
    //             // 注意: 可用余额已在买单开单时减少
    //             let quote_delta = BalanceDelta {
    //                 total: -market_event.kind.amount * market_event.kind.price,
    //                 available: 0.0,
    //             };
    //
    //             (base_delta, quote_delta)
    //         }
    //         Side::Sell.to_string() => {
    //             // Base 的总余额减少 trade.size
    //             // 注意: 可用余额已在卖单开单时减少
    //             let base_delta = BalanceDelta {
    //                 total: -market_event.kind.amount,
    //                 available: 0.0,
    //             };
    //
    //             // Quote 的总余额和可用余额增加 (trade.size * price) 减去 quote 的交易费用
    //             let quote_increase = (market_event.kind.amount * market_event.kind.price) - market_event.fees;
    //             let quote_delta = BalanceDelta {
    //                 total: quote_increase,
    //                 available: quote_increase,
    //             };
    //
    //             (base_delta, quote_delta)
    //         }
    //     };
    //
    //     // 应用 BalanceDelta 并返回更新后的余额
    //     let _base_balance = self.update(base, base_delta);
    //     let _quote_balance = self.update(quote, quote_delta);
    //
    //     AccountEvent {
    //         exchange_timestamp: self.get_exchange_ts().await.unwrap(),
    //         exchange: ExchangeVariant::Simulated,
    //         kind: AccountEventKind::Balances(vec![
    //             TokenBalance::new(base.clone(), _base_balance),
    //             TokenBalance::new(quote.clone(), _quote_balance),
    //         ]),
    //     }
    // }


    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }}


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
