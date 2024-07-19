use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind
{
    Spot, // [NOTE] 注意：Spot 指的是即期合约，此处现在缺乏合约细节字段，不适合MarketID的唯一识别。
    #[serde(alias = "Swap", alias = "SWAP", alias = "PERPETUAL")]
    Perpetual, // [NOTE] 注意：Perpetual 指的是永续合约，此处现缺乏合约细节字段，不适合MarketID的唯一识别。
    Future,
    Option,
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
            | InstrumentKind::Option => {
                write!(f, "option")
            }
        }
    }
}
