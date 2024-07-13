use serde::{Deserialize, Serialize};
use crate::universal::token::Token;


/// 与[`Token`]相关联的[`Balance`]。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TokenBalance {
    pub token: Token,     // 符号
    pub balance: Balance, // 平衡
}

impl TokenBalance {
    pub fn new<S>(token: S, balance: Balance) -> Self
    where
        S: Into<Token>,
    {
        Self {
            token: token.into(),
            balance,
        }
    }
}

/// 总余额和可用余额。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BalanceDelta {
    pub total: f64,     // 总额变化
    pub available: f64, // 可用额变化
}

impl BalanceDelta {
    /// Construct a new [`BalanceDelta`].
    /// 构造一个新的[`BalanceDelta`]。
    pub fn new(total: f64, available: f64) -> Self {
        Self { total, available }
    }
}
