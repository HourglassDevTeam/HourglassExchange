use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::common_skeleton::{instrument::kind::InstrumentKind, token::Token};

pub mod initiator;
pub mod kind;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Instrument
{
    pub base: Token,
    pub quote: Token,
    #[serde(rename = "instrument_kind")]
    pub kind: InstrumentKind,
}

impl Display for Instrument
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "({}_{}, {})", self.base, self.quote, self.kind)
    }
}

impl<S> From<(S, S, InstrumentKind)> for Instrument
where
    S: Into<Token>,
{
    fn from((base, quote, kind): (S, S, InstrumentKind)) -> Self
    {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}

impl Instrument
{
    // Constructor function to create a new Instrument
    pub fn new<S>(base: S, quote: S, kind: InstrumentKind) -> Self
    where
        S: Into<Token>,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}
