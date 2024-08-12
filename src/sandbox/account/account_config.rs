use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{
        instrument::{kind::InstrumentKind, Instrument},
        position::{PositionDirectionMode, PositionMarginMode},
    },
    error::ExecutionError,
    sandbox::utils::config_parser::read_config_file,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountConfig
{
    pub margin_mode: MarginMode,
    pub position_mode: PositionDirectionMode,
    pub position_margin_mode: PositionMarginMode,
    pub commission_level: CommissionLevel,
    pub current_commission_rate: CommissionRates,
    pub leverage_book: HashMap<Instrument, f64>, // 每种金融工具的杠杆比例Registry
    pub fees_book: HashMap<InstrumentKind, f64>, // 每种金融工具的手续费Registry NOTE 某种些交易所的设置颗粒会精确到Instrument.
}

// NOTE 增加假设的佣金费率结构, 用于模拟交易所账户上。每个账户都有自己的佣金费率。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CommissionRates
{
    pub spot_maker: f64,
    pub spot_taker: f64,
    pub perpetual_open: f64,
    pub perpetual_close: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommissionRatesInitiator
{
    pub spot_maker: Option<f64>,
    pub spot_taker: Option<f64>,
    pub perpetual_open: Option<f64>,
    pub perpetual_close: Option<f64>,
}

// 为了确保 CommissionRatesBuilder 被强制实现，可以将 CommissionRates 结构体的初始化方法封装在 builder 方法中。
// 这样，用户只能通过 builder 方法来创建 CommissionRates 实例。以下是具体实现：
impl CommissionRates
{
    pub fn builder() -> CommissionRatesInitiator
    {
        CommissionRatesInitiator::new()
    }
}

impl CommissionRatesInitiator
{
    pub fn new() -> Self
    {
        CommissionRatesInitiator { spot_maker: None,
                                   spot_taker: None,
                                   perpetual_open: None,
                                   perpetual_close: None }
    }

    pub fn spot_maker(mut self, rate: f64) -> Self
    {
        self.spot_maker = Some(rate);
        self
    }

    pub fn spot_taker(mut self, rate: f64) -> Self
    {
        self.spot_taker = Some(rate);
        self
    }

    pub fn perpetual_open(mut self, rate: f64) -> Self
    {
        self.perpetual_open = Some(rate);
        self
    }

    pub fn perpetual_close(mut self, rate: f64) -> Self
    {
        self.perpetual_close = Some(rate);
        self
    }

    pub fn build(self) -> Result<CommissionRates, &'static str>
    {
        Ok(CommissionRates { spot_maker: self.spot_maker.ok_or("Spot maker rate is missing")?,
                             spot_taker: self.spot_taker.ok_or("Spot taker rate is missing")?,
                             perpetual_open: self.perpetual_open.ok_or("Perpetual open rate is missing")?,
                             perpetual_close: self.perpetual_close.ok_or("Perpetual close rate is missing")? })
    }
}

// NOTE 更新费率函数的样本：为 AccountConfig 添加一个方法来更新佣金费率
impl AccountConfig
{
    pub fn new() -> Result<AccountConfig, ExecutionError>
    {
        read_config_file()
    }

    // 更新当前佣金费率
    pub fn update_commission_rate(mut self, commission_rates: &CommissionRates) -> Self
    {
        self.current_commission_rate = match self.commission_level {
            | CommissionLevel::Lv1 => CommissionRates { spot_maker: commission_rates.spot_maker * 0.9,
                                                        spot_taker: commission_rates.spot_taker * 0.9,
                                                        perpetual_open: commission_rates.perpetual_open * 0.9,
                                                        perpetual_close: commission_rates.perpetual_close * 0.9 },
            | CommissionLevel::Lv2 => CommissionRates { spot_maker: commission_rates.spot_maker * 0.8,
                                                        spot_taker: commission_rates.spot_taker * 0.8,
                                                        perpetual_open: commission_rates.perpetual_open * 0.8,
                                                        perpetual_close: commission_rates.perpetual_close * 0.8 },
            | CommissionLevel::Lv3 => CommissionRates { spot_maker: commission_rates.spot_maker * 0.7,
                                                        spot_taker: commission_rates.spot_taker * 0.7,
                                                        perpetual_open: commission_rates.perpetual_open * 0.7,
                                                        perpetual_close: commission_rates.perpetual_close * 0.7 },
            | CommissionLevel::Lv4 => CommissionRates { spot_maker: commission_rates.spot_maker * 0.6,
                                                        spot_taker: commission_rates.spot_taker * 0.6,
                                                        perpetual_open: commission_rates.perpetual_open * 0.6,
                                                        perpetual_close: commission_rates.perpetual_close * 0.6 },
            | CommissionLevel::Lv5 => CommissionRates { spot_maker: commission_rates.spot_maker * 0.5,
                                                        spot_taker: commission_rates.spot_taker * 0.5,
                                                        perpetual_open: commission_rates.perpetual_open * 0.5,
                                                        perpetual_close: commission_rates.perpetual_close * 0.5 },
        };
        self
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MarginMode
{
    SimpleMode,
    SingleCurrencyMargin,
    MultiCurrencyMargin,
    PortfolioMargin,
}

// NOTE 本模拟交易所特有，实际情况可能不同
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum CommissionLevel
{
    Lv1,
    Lv2,
    Lv3,
    Lv4,
    Lv5,
}

// NOTE : this initiator is currently not useful as we now import config from config.
// pub struct AccountConfigInitiator
// {
//     margin_mode: Option<MarginMode>,
//     position_mode: Option<PositionDirectionMode>,
//     position_margin_mode: Option<PositionMarginMode>,
//     commission_level: Option<CommissionLevel>,
// }
//
// impl AccountConfigInitiator
// {
//     pub fn new() -> Self
//     {
//         Self { margin_mode: None,
//                position_mode: None,
//                position_margin_mode: None,
//                commission_level: None }
//     }
//
//     pub fn margin_mode(mut self, margin_mode: MarginMode) -> Self
//     {
//         self.margin_mode = Some(margin_mode);
//         self
//     }
//
//     pub fn position_mode(mut self, position_mode: PositionDirectionMode) -> Self
//     {
//         self.position_mode = Some(position_mode);
//         self
//     }
//
//     pub fn commission_level(mut self, commission_level: CommissionLevel) -> Self
//     {
//         self.commission_level = Some(commission_level);
//         self
//     }
//
//     // NOTE CommissionRates here are initiated as all zeros. this could be replaced with actual values fetched from the exchange,
//     //      be it sandbox or not.
//     pub fn initiate(self) -> Result<AccountConfig, &'static str>
//     {
//         Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
//                            position_mode: self.position_mode.ok_or("position_mode is required")?,
//                            position_margin_mode: self.position_margin_mode.ok_or("position_mode is required")?,
//                            commission_level: self.commission_level.ok_or("commission_level is required")?,
//                            current_commission_rate: CommissionRates { spot_maker: 0.0,
//                                                                       spot_taker: 0.0,
//                                                                       perpetual_open: 0.0,
//                                                                       perpetual_close: 0.0 },
//                            leverage_book: Default::default(),
//                            fees_book: Default::default() })
//     }
// }
