use cerebro_integration::model::instrument::symbol::Symbol;
// 引入符号模型
use serde::{Deserialize, Serialize};

// 引入序列化和反序列化库

/// [`Balance`] associated with a [`Symbol`].
/// 与[`Symbol`]相关联的[`Balance`]。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SymbolBalance {
    pub symbol: Symbol,   // 符号
    pub balance: Balance, // 平衡
}

impl SymbolBalance {
    /// Construct a new [`SymbolBalance`] from a [`Symbol`] and it's associated [`Balance`].
    /// 从一个[`Symbol`]和它关联的[`Balance`]构造一个新的[`SymbolBalance`]。
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
    /// Construct a new [`Balance`].
    /// 构造一个新的[`Balance`]。
    pub fn new(total: f64, available: f64) -> Self {
        Self { total, available }
    }

    /// Calculate the used (`total` - `available`) balance.
    /// 计算使用过的余额（`total` - `available`）。
    pub fn used(&self) -> f64 {
        self.total - self.available
    }

    /// Apply a [`BalanceDelta`] to this [`Balance`].
    /// 对这个[`Balance`]应用一个[`BalanceDelta`]。
    pub fn apply(&mut self, delta: BalanceDelta) {
        self.total += delta.total;
        self.available += delta.available;
    }
}

/// Communicates a change to be applied to a [`Balance`];
/// 传达要应用于[`Balance`]的变更；
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_used() {
        // No Balance is used - 未使用任何余额
        let balance = Balance::new(10.0, 10.0);
        assert_eq!(balance.used(), 0.0);

        // All Balance is used - 全部余额被使用
        let balance = Balance::new(10.0, 0.0);
        assert_eq!(balance.used(), balance.total);

        // Half Balance is used - 使用了一半的余额
        let balance = Balance::new(10.0, 5.0);
        assert_eq!(balance.used(), balance.available);
    }

    #[test]
    fn test_balance_apply_balance_delta() {
        struct TestCase {
            balance: Balance,
            input_delta: BalanceDelta,
            expected: Balance,
        }

        let tests = vec![
            TestCase {
                // TC0: Delta applies a negative total delta only - 仅应用负的总额变化
                balance: Balance::new(10.0, 0.0),
                input_delta: BalanceDelta::new(-10.0, 0.0),
                expected: Balance::new(0.0, 0.0),
            },
            TestCase {
                // TC1: Delta applies a negative available delta only - 仅应用负的可用额变化
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, -10.0),
                expected: Balance::new(10.0, 0.0),
            },
            TestCase {
                // TC2: Delta applies a positive available delta only - 仅应用正的可用额变化
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, 10.0),
                expected: Balance::new(10.0, 20.0),
            },
            TestCase {
                // TC3: Delta applies a positive available delta only - 仅应用正的可用额变化
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(0.0, 10.0),
                expected: Balance::new(10.0, 20.0),
            },
            TestCase {
                // TC4: Delta applies a positive total & available delta - 应用正的总额和可用额变化
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(10.0, 10.0),
                expected: Balance::new(20.0, 20.0),
            },
            TestCase {
                // TC5: Delta applies a negative total & available delta - 应用负的总额和可用额变化
                balance: Balance::new(10.0, 10.0),
                input_delta: BalanceDelta::new(-10.0, -10.0),
                expected: Balance::new(0.0, 0.0),
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            test.balance.apply(test.input_delta);
            assert_eq!(test.balance, test.expected, "TC{} failed", index);
        }
    }
}
