// NOTE 滑点和摩擦成本的设计放在这里

use cerebro_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
/// 以 [`Symbol`]（符号）表示的 [`Trade`]（交易）费用。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SymbolFees {
    pub symbol: Symbol,
    pub fees: f64,
}

impl SymbolFees {
    /// 构造一个新的 [`SymbolFees`]。
    pub fn new<S>(symbol: S, fees: f64) -> Self
    where
        S: Into<Symbol>,
    {
        Self { symbol: symbol.into(), fees }
    }
}
