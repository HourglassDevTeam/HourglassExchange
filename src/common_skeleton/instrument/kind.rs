use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    Spot,
    #[serde(alias = "Swap", alias = "SWAP", alias = "PERPETUAL")]
    Perpetual,
    Future,
    Option,
    Margin,
}

impl Default for InstrumentKind {
    fn default() -> Self {
        Self::Spot
    }
}

impl Display for InstrumentKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            | InstrumentKind::Spot => write!(f, "spot"),
            | InstrumentKind::Future => {
                write!(f, "future")
            }
            | InstrumentKind::Perpetual => write!(f, "perpetual"),
            | InstrumentKind::Option => {
                write!(f, "option")
            }
            | InstrumentKind::Margin => {
                write!(f, "margin")
            }
        }
    }
}
