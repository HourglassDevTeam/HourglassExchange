use std::{
    collections::HashMap,
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
        Side,
        token::Token,
        trade::Trade,
    },
    error::ExecutionError,
    Exchange,
    ExchangeKind::Simulated,
    simulated_exchange::account::Account,
};

#[derive(Clone, Debug)]
pub struct AccountBalances<Event>
    where Event: Clone + Send + Sync + 'static
{
    pub balance_map: HashMap<Token, Balance>,
    pub account_ref: Option<Arc<RwLock<Account<Event>>>>,
}

impl<Event> PartialEq for AccountBalances<Event> where Event: Clone + Send + Sync + 'static
{
    fn eq(&self, other: &Self) -> bool
    {
        self.balance_map == other.balance_map
        // account_ref 是 Arc<RwLock<>>，一般不会比较其内容
    }
}
// CONSIDER 在哪个环节打上时间戳？
impl<Event> AccountBalances<Event> where Event: Clone + Send + Sync + 'static
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
                let balance = self.balance_mut(&open.instrument.quote).expect("[UniLinkExecution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&open.instrument.base).expect("[UniLinkExecution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.unwrap(),
                       exchange: Exchange::from(Simulated),
                       kind: AccountEventKind::Balance(_updated_balance) }
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> TokenBalance
    {
        match cancelled.side {
            | Side::Buy => {
                let balance = self.balance_mut(&cancelled.instrument.quote)
                                  .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&cancelled.instrument.base)
                                  .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// 当client[`Trade`]发生时，会导致base和quote[`Token`]的[`Balance`]发生变化。
    /// 每个[`Balance`]变化的性质取决于匹配的[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    /// [`Side::Buy`]匹配会导致基础[`Token`] [`Balance`]增加`trade_quantity`，报价[`Token`]减少`trade_quantity * price`。
    /// [`Side::Sell`]匹配会导致基础[`Token`] [`Balance`]减少`trade_quantity`，报价[`Token`]增加`trade_quantity * price`。

    pub async fn update_from_trade(&mut self, trade: &Trade) -> AccountEvent
    {
        let Instrument { base, quote, .. } = &trade.instrument;

        // Calculate the base & quote Balance deltas
        let (base_delta, quote_delta) = match trade.side {
            | Side::Buy => {
                // Base total & available increase by trade.size minus base trade.fees
                let base_increase = trade.size - trade.fees;
                let base_delta = BalanceDelta { total: base_increase,
                                                available: base_increase };

                // Quote total decreases by (trade.size * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                let quote_delta = BalanceDelta { total: -trade.size * trade.price,
                                                 available: 0.0 };

                (base_delta, quote_delta)
            }
            | Side::Sell => {
                // Base total decreases by trade.size
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta { total: -trade.size,
                                                available: 0.0 };

                // Quote total & available increase by (trade.size * price) minus quote fees
                let quote_increase = (trade.size * trade.price) - trade.fees;
                let quote_delta = BalanceDelta { total: quote_increase,
                                                 available: quote_increase };

                (base_delta, quote_delta)
            }
        };

        // Apply BalanceDelta & return updated Balance
        let _base_balance = self.update(base, base_delta);
        let _quote_balance = self.update(quote, quote_delta);

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.unwrap(),
                       exchange: Exchange::from(Simulated),
                       kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), _base_balance), TokenBalance::new(quote.clone(), _quote_balance)]) }
    }

    /// Apply the [`BalanceDelta`] to the [`Balance`] of the specified [`Token`], returning a
    /// `Copy` of the updated [`Balance`].
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl<Event> Deref for AccountBalances<Event> where Event: Clone + Send + Sync + 'static
{
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target
    {
        &self.balance_map
    }
}

impl<Event> DerefMut for AccountBalances<Event> where Event: Clone + Send + Sync + 'static
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.balance_map
    }
}
