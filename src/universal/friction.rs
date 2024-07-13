// NOTE 滑点和摩擦成本的设计放在这里

use crate::universal::token::Token;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
/// 以 [`Token`]（符号）表示的 [`Trade`]（交易）费用。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct TokenFees {
    pub symbol: Token,
    pub fees: f64,
}

impl TokenFees {
    /// 构造一个新的 [`TokenFees`]。
    pub fn new<S>(symbol: S, fees: f64) -> Self
    where
        S: Into<Token>,
    {
        Self { symbol: symbol.into(), fees }
    }
}
