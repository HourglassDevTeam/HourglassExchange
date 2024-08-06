use serde::{Deserialize, Serialize};
use crate::common_skeleton::position::PositionMeta;
use crate::common_skeleton::position::positon_meta::{PositionDirectionMode, PositionMarginMode};

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PerpetualPosition {
    pub meta: PositionMeta,
    pub pos_config: PerpetualPositionConfig,
    pub liquidation_price: f64,
    pub margin: f64,
    pub funding_fee: f64,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PerpetualPositionConfig {
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
    position_mode: PositionDirectionMode,
}
