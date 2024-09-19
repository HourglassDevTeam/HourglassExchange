use serde::{Deserialize, Serialize};

use crate::{
    common::account_positions::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode},
    hourglass::config_request::ConfigurationRequest,
};

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

impl From<ConfigurationRequest> for LeveragedTokenPositionConfig
{
    fn from(config_request: ConfigurationRequest) -> Self
    {
        LeveragedTokenPositionConfig { pos_margin_mode: config_request.position_margin_mode,  // 提供默认值或根据需求处理 None
                                       leverage: config_request.leverage_rate,                // 提供默认杠杆值，或根据需求处理 None
                                       position_mode: config_request.position_direction_mode  /* 提供默认值或根据需求处理 None */ }
    }
}
