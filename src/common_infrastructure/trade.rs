use serde::{Deserialize, Serialize};

// 引入相关模块和结构体。
use crate::common_infrastructure::{instrument::Instrument, Side};

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct ClientTrade
{
    pub id: TradeId,
    pub instrument: Instrument,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    // pub count: i64,
    pub fees: f64,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct TradeId(pub i64);

impl<S> From<S> for TradeId where S: Into<i64>
{
    fn from(id: S) -> Self
    {
        Self(id.into())
    }
}
