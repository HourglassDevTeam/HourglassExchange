use serde::{Deserialize, Serialize};

use crate::common_infrastructure::position::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode};

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PerpetualPosition {
    pub meta: PositionMeta,                    // 复合类型，包含静态数据、实时更新数据和静态更新数据
    pub pos_config: PerpetualPositionConfig,   // 静态数据
    pub liquidation_price: f64,                // 实时更新 NOTE : To Be Checked
    pub margin: f64,                           // 实时更新 NOTE : To Be Checked
    pub funding_fee: f64,                      // 实时更新 NOTE : To Be Checked
}

impl PerpetualPosition {
    /// 更新平仓价格
    pub fn update_liquidation_price(&mut self, new_price: f64) {
        self.liquidation_price = new_price;
    }

    /// 更新保证金
    pub fn update_margin(&mut self, new_margin: f64) {
        self.margin = new_margin;
    }

    /// 更新资金费率
    pub fn update_funding_fee(&mut self, new_fee: f64) {
        self.funding_fee = new_fee;
    }

    /// 更新静态数据部分
    pub fn update_pos_config(&mut self, new_config: PerpetualPositionConfig) {
        self.pos_config = new_config;
    }

    /// 更新PositionMeta
    pub fn update_meta(&mut self, new_meta: PositionMeta) {
        self.meta = new_meta;
    }
}


#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PerpetualPositionConfig
{
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
    position_mode: PositionDirectionMode,
}
#[allow(dead_code)]
pub struct PerpetualPositionBuilder
{
    meta: Option<PositionMeta>,
    pos_config: Option<PerpetualPositionConfig>,
    liquidation_price: Option<f64>,
    margin: Option<f64>,
    funding_fee: Option<f64>,
}
#[allow(dead_code)]
impl PerpetualPositionBuilder
{
    pub fn new() -> Self
    {
        Self {
            meta: None,
            pos_config: None,
            liquidation_price: None,
            margin: None,
            funding_fee: None,
        }
    }

    pub fn meta(mut self, meta: PositionMeta) -> Self
    {
        self.meta = Some(meta);
        self
    }

    pub fn pos_config(mut self, pos_config: PerpetualPositionConfig) -> Self
    {
        self.pos_config = Some(pos_config);
        self
    }

    pub fn liquidation_price(mut self, liquidation_price: f64) -> Self
    {
        self.liquidation_price = Some(liquidation_price);
        self
    }

    pub fn margin(mut self, margin: f64) -> Self
    {
        self.margin = Some(margin);
        self
    }

    pub fn funding_fee(mut self, funding_fee: f64) -> Self
    {
        self.funding_fee = Some(funding_fee);
        self
    }

    pub fn build(self) -> Option<PerpetualPosition>
    {
        Some(PerpetualPosition {
            meta: self.meta?,
            pos_config: self.pos_config?,
            liquidation_price: self.liquidation_price?,
            margin: self.margin?,
            funding_fee: self.funding_fee?,
        })
    }
}
