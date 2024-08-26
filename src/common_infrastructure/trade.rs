use serde::{Deserialize, Serialize};

// 引入相关模块和结构体。
use crate::common_infrastructure::{ instrument::Instrument, Side};
use crate::common_infrastructure::order::OrderId;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct ClientTrade
{
    pub trade_id: ClientTradeId,
    pub client_order_id: OrderId,
    pub instrument: Instrument,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub fees: f64,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientTradeId(pub i64);

impl<S> From<S> for ClientTradeId where S: Into<i64>
{
    fn from(id: S) -> Self
    {
        Self(id.into())
    }
}
