use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::ExecutionError;
use crate::Exchange;
use crate::ExchangeKind::Simulated;
use crate::universal::balance::{Balance, BalanceDelta, TokenBalance};
use crate::universal::event::{AccountEvent, AccountEventKind};
use crate::universal::instrument::Instrument;
use crate::universal::order::{Open, Order};
use crate::universal::Side;
use crate::universal::token::Token;
use crate::universal::trade::Trade;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct AccountBalances(pub HashMap<Token, Balance>);

impl AccountBalances {
    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError> {
        self.get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError> {
        self.get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance> {
        self.0
            .clone()
            .into_iter()
            .map(|(token, balance)| TokenBalance::new(token, balance))
            .collect()
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    /// NOTE 这个方法不应该导致panic,Client要能妥善处理这种状况。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError> {
        let available = self.balance(token)?.available;
        match available >= required_balance {
            true => Ok(()),
            false => Err(ExecutionError::InsufficientBalance(token.clone())),
        }
    }

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> AccountEvent {
        let updated_balance = match open.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&open.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&open.instrument.base)
                    .expect("[UniLinkExecution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent {
            exchange_ts: todo!(),
            exchange: Exchange::from(Simulated),
            kind: AccountEventKind::Balance(updated_balance),
        }
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> TokenBalance {
        match cancelled.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&cancelled.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&cancelled.instrument.base)
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

    pub fn update_from_trade(&mut self, trade: &Trade) -> AccountEvent {
        let Instrument { base, quote, .. } = &trade.instrument;

        // Calculate the base & quote Balance deltas
        let (base_delta, quote_delta) = match trade.side {
            Side::Buy => {
                // Base total & available increase by trade.size minus base trade.fees
                let base_increase = trade.size - trade.fees;
                let base_delta = BalanceDelta {
                    total: base_increase,
                    available: base_increase,
                };

                // Quote total decreases by (trade.size * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                let quote_delta = BalanceDelta {
                    total: -trade.size * trade.price,
                    available: 0.0,
                };

                (base_delta, quote_delta)
            }
            Side::Sell => {
                // Base total decreases by trade.size
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta {
                    total: -trade.size,
                    available: 0.0,
                };

                // Quote total & available increase by (trade.size * price) minus quote fees
                let quote_increase = (trade.size * trade.price) - trade.fees;
                let quote_delta = BalanceDelta {
                    total: quote_increase,
                    available: quote_increase,
                };

                (base_delta, quote_delta)
            }
        };

        // Apply BalanceDelta & return updated Balance
        let base_balance = self.update(base, base_delta);
        let quote_balance = self.update(quote, quote_delta);

        AccountEvent {
            exchange_ts: todo!(),
            exchange: Exchange::from(Simulated),
            kind: AccountEventKind::Balances(vec![
                TokenBalance::new(base.clone(), base_balance),
                TokenBalance::new(quote.clone(), quote_balance),
            ]),
        }
    }

    /// Apply the [`BalanceDelta`] to the [`Balance`] of the specified [`Token`], returning a
    /// `Copy` of the updated [`Balance`].
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl std::ops::Deref for AccountBalances {
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AccountBalances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
