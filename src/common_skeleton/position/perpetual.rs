use serde::{Deserialize, Serialize};

use crate::common_skeleton::position::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode};

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PerpetualPosition
{
    pub meta: PositionMeta,
    pub pos_config: PerpetualPositionConfig,
    pub liquidation_price: f64,
    pub margin: f64,
    pub funding_fee: f64,
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
        Self { meta: None,
               pos_config: None,
               liquidation_price: None,
               margin: None,
               funding_fee: None }
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

    pub fn build(self) -> Result<PerpetualPosition, &'static str>
    {
        Ok(PerpetualPosition { meta: self.meta.ok_or("meta is required")?,
                               pos_config: self.pos_config.ok_or("pos_config is required")?,
                               liquidation_price: self.liquidation_price.ok_or("liquidation_price is required")?,
                               margin: self.margin.ok_or("margin is required")?,
                               funding_fee: self.funding_fee.ok_or("funding_fee is required")? })
    }
}
