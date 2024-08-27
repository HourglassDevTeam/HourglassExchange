use serde::{Deserialize, Serialize};

use crate::common::token::Token;

/// 与[`Token`]相关联的[`Balance`]。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TokenBalance
{
    pub token: Token,     // 符号
    pub balance: Balance, // 账单
}

impl TokenBalance
{
    pub fn new(token: impl Into<Token>, balance: Balance) -> Self
    {
        Self { token: token.into(), balance }
    }

    // update balance price's current price on a MarketEvent's new price
    pub fn update_current_price(&mut self, price: f64)
    {
        self.balance.current_price = price;
    }
}

/// 总余额和可用余额。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Balance
{
    pub current_price: f64, // NOTE 当前价格 newly added on 1st Aug 2024
    pub total: f64,         // 总额
    pub available: f64,     // 可用余额
}

impl Balance
{
    /// 构造一个新的[`Balance`]。
    pub fn new(total: f64, available: f64, current_price: f64) -> Self
    {
        Self { total, available, current_price }
    }

    /// 计算使用过的余额（`total` - `available`）。
    pub fn used(&self) -> f64
    {
        self.total - self.available
    }

    /// 对这个[`Balance`]应用一个[`BalanceDelta`]。
    pub fn apply(&mut self, delta: BalanceDelta) -> Result<(), &'static str>
    {
        if self.total + delta.total < 0.0 || self.available + delta.available < 0.0 {
            return Err("[UniLinkExecution] : Insufficient balance to apply the delta.");
        }
        self.total += delta.total;
        self.available += delta.available;
        Ok(())
    }
}

/// 可应用于[`Balance`]的增量变更；
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BalanceDelta
{
    pub total: f64,     // 总额变化
    pub available: f64, // 可用额变化
}

impl BalanceDelta
{
    /// Construct a new [`BalanceDelta`].
    /// 构造一个新的[`BalanceDelta`]。
    pub fn new(total: f64, available: f64) -> Self
    {
        Self { total, available }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn token_balance_new_should_create_token_balance()
    {
        let token = Token::from("BTC");
        let balance = Balance::new(100.0, 50.0, 20000.0);
        let token_balance = TokenBalance::new(token.clone(), balance);
        assert_eq!(token_balance.token, token);
        assert_eq!(token_balance.balance, balance);
    }

    #[test]
    fn token_balance_update_current_price_should_update_price()
    {
        let token = Token::from("BTC");
        let balance = Balance::new(100.0, 50.0, 20000.0);
        let mut token_balance = TokenBalance::new(token, balance);
        token_balance.update_current_price(21000.0);
        assert_eq!(token_balance.balance.current_price, 21000.0);
    }

    #[test]
    fn balance_new_should_create_balance()
    {
        let balance = Balance::new(100.0, 50.0, 20000.0);
        assert_eq!(balance.total, 100.0);
        assert_eq!(balance.available, 50.0);
        assert_eq!(balance.current_price, 20000.0);
    }

    #[test]
    fn balance_used_should_return_used_balance()
    {
        let balance = Balance::new(100.0, 50.0, 20000.0);
        assert_eq!(balance.used(), 50.0);
    }

    #[test]
    fn balance_apply_should_apply_balance_delta()
    {
        let mut balance = Balance::new(100.0, 50.0, 20000.0);
        let delta = BalanceDelta::new(10.0, 5.0);
        let _ = balance.apply(delta);
        assert_eq!(balance.total, 110.0);
        assert_eq!(balance.available, 55.0);
    }

    #[test]
    fn balance_delta_new_should_create_balance_delta()
    {
        let delta = BalanceDelta::new(10.0, 5.0);
        assert_eq!(delta.total, 10.0);
        assert_eq!(delta.available, 5.0);
    }
}
