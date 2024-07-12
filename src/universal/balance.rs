use cerebro_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};

/// 与[`Symbol`]相关联的[`Balance`]。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TokenBalance {
    pub symbol: Symbol,   // 符号
    pub balance: Balance, // 平衡
}

impl TokenBalance {
    /// 从一个[`Symbol`]和它关联的[`Balance`]构造一个新的[`TokenBalance`]。
    pub fn new<S>(symbol: S, balance: Balance) -> Self
    where
        S: Into<Symbol>,
    {
        Self {
            symbol: symbol.into(),
            balance,
        }
    }
}

/// 总余额和可用余额。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Balance {
    pub total: f64,     // 总额
    pub available: f64, // 可用余额
}

impl Balance {
    /// 构造一个新的[`Balance`]。
    pub fn new(total: f64, available: f64) -> Self {
        Self { total, available }
    }

    /// 计算使用过的余额（`total` - `available`）。
    pub fn used(&self) -> f64 {
        self.total - self.available
    }

    /// 对这个[`Balance`]应用一个[`BalanceDelta`]。
    pub fn apply(&mut self, delta: BalanceDelta) {
        self.total += delta.total;
        self.available += delta.available;
    }
}

/// 可应用于[`Balance`]的变更；
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
