use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct Token(String);

/// 与[`Symbol`]相关联的[`Balance`]。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TokenBalance {
    pub symbol: Token,    // 符号
    pub balance: Balance, // 平衡
}

impl TokenBalance {
    /// 从一个[`Symbol`]和它关联的[`Balance`]构造一个新的[`TokenBalance`]。
    pub fn new<S>(symbol: S, balance: Balance) -> Self
    where
        S: Into<Token>,
    {
        Self {
            symbol: symbol.into(),
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
