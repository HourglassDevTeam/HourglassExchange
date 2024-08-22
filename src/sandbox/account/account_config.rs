use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{
        instrument::{kind::InstrumentKind},
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
    pub account_leverage_rate: f64,    // NOTE 每种金融工具应该拥有杠杆比例Registry。这个写法是高度简化的。
    pub fees_book: HashMap<InstrumentKind, CommissionRates>, // 每种金融工具的手续费Registry NOTE 某种些交易所的设置颗粒会精确到Instrument.
}

// NOTE 增加假设的佣金费率结构, 用于模拟交易所账户上。每个账户都有自己的佣金费率。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CommissionRates
{
    pub maker_fees: f64,
    pub taker_fees: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommissionRatesInitiator
{
    pub maker_fees: Option<f64>,
    pub taker_fees: Option<f64>,
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

impl Default for CommissionRatesInitiator
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl CommissionRatesInitiator
{
    pub fn new() -> Self
    {
        CommissionRatesInitiator { maker_fees: None,
                                   taker_fees: None }
    }

    pub fn maker(mut self, rate: f64) -> Self
    {
        self.maker_fees = Some(rate);
        self
    }

    pub fn taker(mut self, rate: f64) -> Self
    {
        self.taker_fees = Some(rate);
        self
    }

    pub fn build(self) -> Result<CommissionRates, &'static str>
    {
        Ok(CommissionRates { maker_fees: self.maker_fees.ok_or("Spot maker rate is missing")?,
                             taker_fees: self.taker_fees.ok_or("Spot taker rate is missing")? })
    }
}
// NOTE 更新费率函数的样本：为 AccountConfig 添加一个方法来更新佣金费率
impl AccountConfig
{
    pub fn new() -> Result<AccountConfig, ExecutionError>
    {
        read_config_file()
    }

    pub fn get_maker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        self.fees_book
            .get(instrument_kind)
            .map(|rates| rates.maker_fees)
            .ok_or_else(|| ExecutionError::SandBox(format!("Open fee rate for {:?} not found", instrument_kind)))
    }

    // 获取指定InstrumentKind的平仓费率
    pub fn get_taker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        self.fees_book
            .get(instrument_kind)
            .map(|rates| rates.taker_fees)
            .ok_or_else(|| ExecutionError::SandBox(format!("Close fee rate for {:?} not found", instrument_kind)))
    }

    // 更新当前佣金费率 NOTE This function is currently only demonstrative.
    pub fn update_commission_rate_with_discount(mut self, commission_rates: &CommissionRates) -> Self
    {
        self.current_commission_rate = match self.commission_level {
            | CommissionLevel::Lv1 => CommissionRates { maker_fees: commission_rates.maker_fees * 0.9,
                                                        taker_fees: commission_rates.taker_fees * 0.9 },
            | CommissionLevel::Lv2 => CommissionRates { maker_fees: commission_rates.maker_fees * 0.8,
                                                        taker_fees: commission_rates.taker_fees * 0.8 },
            | CommissionLevel::Lv3 => CommissionRates { maker_fees: commission_rates.maker_fees * 0.7,
                                                        taker_fees: commission_rates.taker_fees * 0.7 },
            | CommissionLevel::Lv4 => CommissionRates { maker_fees: commission_rates.maker_fees * 0.6,
                                                        taker_fees: commission_rates.taker_fees * 0.6 },
            | CommissionLevel::Lv5 => CommissionRates { maker_fees: commission_rates.maker_fees * 0.5,
                                                        taker_fees: commission_rates.taker_fees * 0.5 },
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
pub struct AccountConfigInitiator
{
    margin_mode: Option<MarginMode>,
    position_mode: Option<PositionDirectionMode>,
    position_margin_mode: Option<PositionMarginMode>,
    commission_level: Option<CommissionLevel>,
}

impl AccountConfigInitiator
{
    pub fn new() -> Self
    {
        Self { margin_mode: None,
               position_mode: None,
               position_margin_mode: None,
               commission_level: None }
    }

    pub fn margin_mode(mut self, margin_mode: MarginMode) -> Self
    {
        self.margin_mode = Some(margin_mode);
        self
    }

    pub fn position_mode(mut self, position_mode: PositionDirectionMode) -> Self
    {
        self.position_mode = Some(position_mode);
        self
    }

    pub fn commission_level(mut self, commission_level: CommissionLevel) -> Self
    {
        self.commission_level = Some(commission_level);
        self
    }

    // NOTE CommissionRates should be imported from config file in the project root.
    pub fn initiate(self) -> Result<AccountConfig, &'static str>
    {
        Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
                           position_mode: self.position_mode.ok_or("position_mode is required")?,
                           position_margin_mode: self.position_margin_mode.ok_or("position_mode is required")?,
                           commission_level: self.commission_level.ok_or("commission_level is required")?,
                           current_commission_rate: CommissionRates { maker_fees: 0.0, taker_fees: 0.0 },
                           account_leverage_rate: Default::default(),
                           fees_book: Default::default() })
    }
}
