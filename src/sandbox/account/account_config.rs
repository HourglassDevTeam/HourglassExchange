use crate::{
    common::{
        account_positions::{PositionDirectionMode, PositionMarginMode},
        instrument::kind::InstrumentKind,
    },
    error::ExchangeError,
    sandbox::utils::config_parser::read_config_file,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountConfig
{
    pub margin_mode: MarginMode,
    pub position_direction_mode: PositionDirectionMode,
    pub position_margin_mode: PositionMarginMode,
    pub commission_level: CommissionLevel,
    pub funding_rate: f64,
    pub account_leverage_rate: f64,
    pub fees_book: HashMap<InstrumentKind, CommissionRates>,
    pub execution_mode: SandboxMode,
    pub max_price_deviation:f64,
    // pub stop_loss_threshold: Option<f64>,    // 止损阈值，用于设置当资产价格向不利方向移动并达到该阈值时，自动平仓以避免进一步损失。
    // pub take_profit_threshold: Option<f64>,  // 止盈阈值，用于在市场价格达到一定盈利目标时自动平仓以锁定利润。
    // pub trailing_stop_loss: Option<f64>,   // 跟踪止损，用于动态调整止损价格，跟随市场价格的波动来保护盈利。当价格向有利方向移动时，止损价格也相应调整；当价格逆向移动时，止损触发。
    // pub trailing_take_profit: Option<f64>,  // 跟踪止盈，与 trailing_stop_loss 类似，但它是在价格达到某个盈利水平后，动态设置止盈点以保证在价格下跌前锁定更多利润。
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum SandboxMode
{
    Backtest,
    Online,
}
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
        Self { maker_fees: None,
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

impl AccountConfig
{
    pub fn new() -> Result<AccountConfig, ExchangeError>
    {
        read_config_file()
    }

    pub fn get_maker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>
    {
        self.fees_book
            .get(instrument_kind)
            .map(|rates| rates.maker_fees)
            .ok_or_else(|| ExchangeError::SandBox(format!("Open fee rate for {:?} not found", instrument_kind)))
    }

    pub fn get_taker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>
    {
        self.fees_book
            .get(instrument_kind)
            .map(|rates| rates.taker_fees)
            .ok_or_else(|| ExchangeError::SandBox(format!("Close fee rate for {:?} not found", instrument_kind)))
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

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum CommissionLevel
{
    Lv1,
    Lv2,
    Lv3,
    Lv4,
    Lv5,
}

pub struct AccountConfigInitiator
{
    margin_mode: Option<MarginMode>,
    position_mode: Option<PositionDirectionMode>,
    position_margin_mode: Option<PositionMarginMode>,
    commission_level: Option<CommissionLevel>,
    fund_fee_rate: Option<f64>,
    max_price_deviation:  Option<f64>,
}
impl Default for AccountConfigInitiator
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl AccountConfigInitiator
{
    pub fn new() -> Self
    {
        Self { margin_mode: None,
               position_mode: None,
               position_margin_mode: None,
               commission_level: None,
               fund_fee_rate: None,
            max_price_deviation: None,
        }
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

    pub fn initiate(self) -> Result<AccountConfig, &'static str>
    {
        Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
                           position_direction_mode: self.position_mode.ok_or("position_mode is required")?,
                           position_margin_mode: self.position_margin_mode.ok_or("position margin mode is required")?,
                           commission_level: self.commission_level.ok_or("commission level is required")?,
                           funding_rate: self.fund_fee_rate.ok_or("fund_fee_rate is required")?,
                           account_leverage_rate: Default::default(),
                           fees_book: Default::default(),
                           execution_mode: SandboxMode::Backtest,
                           max_price_deviation: self.max_price_deviation.ok_or("max price deviation is required")?,
        })
    }
}
