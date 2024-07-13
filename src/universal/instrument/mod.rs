pub mod kind;

use std::fmt::{Display, Formatter};
use crate::universal::symbol::Symbol;
use crate::universal::instrument::kind::InstrumentKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]

pub struct Instrument {
    pub base: Symbol,
    pub quote: Symbol,
    #[serde(rename = "instrument_kind")]
    pub kind: InstrumentKind,
}

impl Display for Instrument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}_{}, {})", self.base, self.quote, self.kind)
    }
}

impl<S> From<(S, S, InstrumentKind)> for Instrument
where
    S: Into<Symbol>,
{
    fn from((base, quote, kind): (S, S, InstrumentKind)) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}

/// 从 `Symbol` 和 `InstrumentKind` 创建 `Instrument` 的方式存在较高的错误风险，
/// 目前仅适用于期货（futures）。
/// [fixme] 这个方法未来也应该支持其他类型的合约

impl Instrument {
    pub fn new<S, F>(base: S, quote: S, kind_fn: F) -> Self
                     where
                         S: Into<Symbol>,
                         F: FnOnce(FutureContract) -> InstrumentKind,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind: kind_fn(FutureContract {
                future_code: "".to_string(),
                multiplier: 1,
            }),
        }
    }
}
