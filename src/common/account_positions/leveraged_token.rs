use serde::{Deserialize, Serialize};

use crate::common::account_positions::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LeveragedTokenPosition
{
    pub meta: PositionMeta,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LeveragedTokenPositionConfig
{
    pub(crate) pos_margin_mode: PositionMarginMode,
    pub(crate) leverage: f64,
    pub(crate) position_mode: PositionDirectionMode,
}
