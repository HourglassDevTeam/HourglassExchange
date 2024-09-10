use serde::Serialize;
use crate::common::account_positions::{PositionDirectionMode, PositionMarginMode};
use crate::Deserialize;
use crate::common::instrument::Instrument;
use crate::common::order::identification::client_order_id::ClientOrderId;
use crate::Exchange;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct ConfigurationRequest
{
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub timestamp: i64,
    pub cid: Option<ClientOrderId>,
    pub leverage_rate : Option<f64>,
    pub position_margin_mode: Option<PositionMarginMode>,
    pub position_direction_mode: Option<PositionDirectionMode>,
}
