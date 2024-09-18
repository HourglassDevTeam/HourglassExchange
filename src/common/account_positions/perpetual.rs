use serde::{Deserialize, Serialize};

use crate::{
    common::account_positions::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode},
    sandbox::config_request::ConfigurationRequest,
};

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PerpetualPosition
{
    pub meta: PositionMeta,                  // 复合类型，包含静态数据、实时更新数据和静态更新数据
    pub pos_config: PerpetualPositionConfig, // 静态数据
    pub isolated_margin: Option<f64>,
    pub liquidation_price: Option<f64>, // 实时更新 NOTE : 需要持续更新吗
}

impl PerpetualPosition
{
    /// 更新平仓价格
    pub fn update_liquidation_price(&mut self, new_price: f64)
    {
        self.liquidation_price = Some(new_price);
    }

    /// 更新静态数据部分
    pub fn update_pos_config(&mut self, new_config: PerpetualPositionConfig)
    {
        self.pos_config = new_config;
    }

    /// 更新PositionMeta
    pub fn update_meta(&mut self, new_meta: PositionMeta)
    {
        self.meta = new_meta;
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PerpetualPositionConfig
{
    pub(crate) pos_margin_mode: PositionMarginMode,
    pub(crate) leverage: f64, /// 杠杆率, 大于1.0
    pub(crate) position_direction_mode: PositionDirectionMode,
}

impl From<ConfigurationRequest> for PerpetualPositionConfig
{
    fn from(config_request: ConfigurationRequest) -> Self
    {
        PerpetualPositionConfig { pos_margin_mode: config_request.position_margin_mode,            // 提供默认值或根据需求处理 None
                                  leverage: config_request.leverage_rate,                          // 提供默认杠杆值，或根据需求处理 None
                                  position_direction_mode: config_request.position_direction_mode  /* 提供默认值或根据需求处理 None */ }
    }
}

#[allow(dead_code)]
pub struct PerpetualPositionBuilder
{
    meta: Option<PositionMeta>,
    pos_config: Option<PerpetualPositionConfig>,
    isolated_margin: Option<f64>,
    liquidation_price: Option<f64>,
}

impl Default for PerpetualPositionBuilder
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[allow(dead_code)]
impl PerpetualPositionBuilder
{
    pub fn new() -> Self
    {
        Self { meta: None,
               pos_config: None,
               isolated_margin: None,
               liquidation_price: None }
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

    pub fn build(self) -> Option<PerpetualPosition>
    {
        Some(PerpetualPosition { meta: self.meta?,
                                 pos_config: self.pos_config?,
                                 isolated_margin: None,
                                 liquidation_price: Some(self.liquidation_price?) })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{
            account_positions::{position_id::PositionId, PositionDirectionMode, PositionMarginMode},
            balance::{Balance, TokenBalance},
            instrument::{kind::InstrumentKind, Instrument},
            token::Token,
            Side,
        },
        Exchange,
    };

    #[test]
    fn perpetual_position_should_update_liquidation_price()
    {
        let mut position = PerpetualPosition { meta: PositionMeta { position_id: PositionId(123124124124124),
                                                                    enter_ts: 1625247600,
                                                                    update_ts: 1625247601,
                                                                    exit_balance: TokenBalance::new(Token::from("BTC"), Balance::new(0.0, 0.0, Some(0.0))),
                                                                    exchange: Exchange::SandBox,
                                                                    instrument: Instrument::new("BTC", "USDT", InstrumentKind::Spot),
                                                                    side: Side::Buy,
                                                                    current_size: 1.0,
                                                                    current_fees_total: 66.0,
                                                                    current_avg_price_gross: 50_000.0,
                                                                    current_symbol_price: 61_000.0,
                                                                    current_avg_price: 50_000.0,
                                                                    unrealised_pnl: 11_000.0,
                                                                    realised_pnl: 0.0 },
                                               pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                                                     leverage: 1.0,
                                                                                     position_direction_mode: PositionDirectionMode::LongShort },
                                               isolated_margin: None,
                                               liquidation_price: Some(100.0) };
        position.update_liquidation_price(150.0);
        assert_eq!(position.liquidation_price, Some(150.0));
    }
}
