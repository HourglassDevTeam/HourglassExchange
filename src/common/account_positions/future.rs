use crate::{
    common::account_positions::{position_meta::PositionMeta, PositionDirectionMode, PositionMarginMode},
    hourglass::config_request::ConfigurationRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FuturePosition
{
    pub meta: PositionMeta,
    pub pos_config: FuturePositionConfig, // 静态数据
    pub liquidation_price: f64,           // 实时更新
    pub isolated_margin: Option<f64>,
    pub funding_fee: f64, // 实时更新
}

impl FuturePosition
{
    /// 更新平仓价格
    pub fn update_liquidation_price(&mut self, new_price: f64)
    {
        self.liquidation_price = new_price;
    }

    /// 更新资金费用
    pub fn update_funding_fee(&mut self, new_fee: f64)
    {
        self.funding_fee = new_fee;
    }

    /// 更新静态数据部分
    pub fn update_pos_config(&mut self, new_config: FuturePositionConfig)
    {
        self.pos_config = new_config;
    }

    /// 更新 PositionMeta
    pub fn update_meta(&mut self, new_meta: PositionMeta)
    {
        self.meta = new_meta;
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FuturePositionConfig
{
    pub(crate) pos_margin_mode: PositionMarginMode,
    pub(crate) leverage: f64,
    pub(crate) position_direction_mode: PositionDirectionMode,
}

impl From<ConfigurationRequest> for FuturePositionConfig
{
    fn from(config_request: ConfigurationRequest) -> Self
    {
        FuturePositionConfig { pos_margin_mode: config_request.position_margin_mode,
                               leverage: config_request.leverage_rate,
                               position_direction_mode: config_request.position_direction_mode }
    }
}

#[allow(dead_code)]
pub struct FuturePositionBuilder
{
    meta: Option<PositionMeta>,
    pos_config: Option<FuturePositionConfig>,
    liquidation_price: Option<f64>,
    funding_fee: Option<f64>,
}

impl Default for FuturePositionBuilder
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[allow(dead_code)]
impl FuturePositionBuilder
{
    pub fn new() -> Self
    {
        Self { meta: None,
               pos_config: None,
               liquidation_price: None,
               funding_fee: None }
    }

    pub fn meta(mut self, meta: PositionMeta) -> Self
    {
        self.meta = Some(meta);
        self
    }

    pub fn pos_config(mut self, pos_config: FuturePositionConfig) -> Self
    {
        self.pos_config = Some(pos_config);
        self
    }

    pub fn liquidation_price(mut self, liquidation_price: f64) -> Self
    {
        self.liquidation_price = Some(liquidation_price);
        self
    }

    pub fn funding_fee(mut self, funding_fee: f64) -> Self
    {
        self.funding_fee = Some(funding_fee);
        self
    }

    pub fn build(self) -> Option<FuturePosition>
    {
        Some(FuturePosition { meta: self.meta?,
                              pos_config: self.pos_config?,
                              liquidation_price: self.liquidation_price?,
                              isolated_margin: None,
                              funding_fee: self.funding_fee? })
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
    fn future_position_should_update_liquidation_price_and_funding_fee()
    {
        let mut position = FuturePosition { meta: PositionMeta { position_id: PositionId(123124124124124),
                                                                 enter_ts: 1625247600,
                                                                 update_ts: 1625247601,
                                                                 exit_balance: TokenBalance::new(Token::from("BTC"), Balance::new(0.0, 0.0, Some(0.0))),
                                                                 exchange: Exchange::Hourglass,
                                                                 instrument: Instrument::new("BTC", "USDT", InstrumentKind::Spot),
                                                                 side: Side::Buy,
                                                                 current_size: 1.0,
                                                                 current_fees_total: 66.0,
                                                                 current_avg_price_gross: 50_000.0,
                                                                 current_symbol_price: 61_000.0,
                                                                 current_avg_price: 50_000.0,
                                                                 unrealised_pnl: 11_000.0,
                                                                 realised_pnl: 0.0 },
                                            pos_config: FuturePositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                                               leverage: 1.0,
                                                                               position_direction_mode: PositionDirectionMode::LongShort },
                                            liquidation_price: 100.0,
                                            isolated_margin: None,
                                            funding_fee: 10.0 };

        position.update_liquidation_price(150.0);
        assert_eq!(position.liquidation_price, 150.0);

        position.update_funding_fee(15.0);
        assert_eq!(position.funding_fee, 15.0);
    }
}
