use serde::{Deserialize, Serialize};

use crate::common_infrastructure::position::position_meta::PositionMeta;
use crate::common_infrastructure::position::{PositionDirectionMode, PositionMarginMode};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FuturePosition
{
    pub meta: PositionMeta,
    pub pos_config: FuturePositionConfig, // 静态数据
    pub liquidation_price: f64,              // 实时更新 NOTE : To Be Checked
    pub margin: f64,                         // 实时更新 NOTE : To Be Checked
    pub funding_fee: f64,                    // 实时更新 NOTE : To Be Checked
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FuturePositionConfig{
    pub(crate) pos_margin_mode: PositionMarginMode,
    pub(crate) leverage: f64,
    pub(crate) position_mode: PositionDirectionMode,
}