use serde::{Serialize,Deserialize};



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
