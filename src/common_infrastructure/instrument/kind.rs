use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind
{
    Spot,
    #[serde(alias = "Swap", alias = "SWAP", alias = "PERPETUAL")]
    Perpetual,
    Future,
    CryptoOption,
    CryptoLeveragedToken,
    CommodityOption,
    CommodityFuture,
}

impl Default for InstrumentKind
{
    fn default() -> Self
    {
        Self::Spot
    }
}

impl Display for InstrumentKind
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self {
            | InstrumentKind::Spot => write!(f, "spot"),
            | InstrumentKind::Future => {
                write!(f, "future")
            }
            | InstrumentKind::Perpetual => write!(f, "perpetual"),
            | InstrumentKind::CryptoOption => {
                write!(f, "option")
            }
            | InstrumentKind::CryptoLeveragedToken => {
                write!(f, "margin")
            }
            | InstrumentKind::CommodityFuture => {
                write!(f, "commodity_future")
            }
            | InstrumentKind::CommodityOption => {
                write!(f, "commodity_option")
            }
        }
    }
}
impl From<String> for InstrumentKind
{
    fn from(s: String) -> Self
    {
        match s.as_str() {
            | "Spot" => InstrumentKind::Spot,
            | "Perpetual" => InstrumentKind::Perpetual,
            | "Future" => InstrumentKind::Future,
            | "Option" => InstrumentKind::CryptoOption,
            | "Margin" => InstrumentKind::CryptoLeveragedToken,
            | _ => panic!("Unknown instrument kind: {}", s),
        }
    }
}
