pub mod kind;

use crate::universal::{instrument::kind::InstrumentKind, symbol::Symbol};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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

impl Instrument {
    // Constructor function to create a new Instrument
    pub fn new<S>(base: S, quote: S, kind: InstrumentKind) -> Self
    where
        S: Into<Symbol>,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}
