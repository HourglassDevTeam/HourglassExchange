use crate::{
    common::{
        account_positions::{PositionDirectionMode},
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
    pub margin_mode: MarginMode,                             // 保证金模式，用于定义账户的保证金类型（例如逐仓、全仓）
    pub position_direction_mode: PositionDirectionMode,      // 持仓方向模式，定义账户是否允许同时持有多头和空头（双向持仓）或仅允许单一方向的仓位
    // pub position_margin_mode: PositionMarginMode,            // 持仓保证金模式，定义每个仓位的保证金计算方式
    pub commission_level: CommissionLevel,                   // 手续费等级，决定账户的交易手续费率
    pub funding_rate: f64,                                   // 资金费率，用于合约交易中计算资金费用
    pub account_leverage_rate: f64,                          // 账户杠杆率，决定账户在杠杆交易中的放大倍数
    pub fees_book: HashMap<InstrumentKind, CommissionRates>, // 手续费表，存储每种合约类型的手续费率
    pub execution_mode: SandboxMode,                         // 执行模式，定义账户是在沙盒模式（模拟交易）还是在真实环境中运行
    pub max_price_deviation: f64,                            /* 最大价格偏差，用于限制订单价格与市场价格的偏离范围
                                                              * pub stop_loss_threshold: Option<f64>,    // 止损阈值，用于设置当资产价格向不利方向移动并达到该阈值时，自动平仓以避免进一步损失。
                                                              * pub take_profit_threshold: Option<f64>,  // 止盈阈值，用于在市场价格达到一定盈利目标时自动平仓以锁定利润。
                                                              * pub trailing_stop_loss: Option<f64>,   // 跟踪止损，用于动态调整止损价格，跟随市场价格的波动来保护盈利。当价格向有利方向移动时，止损价格也相应调整；当价格逆向移动时，止损触发。
                                                              * pub trailing_take_profit: Option<f64>,  // 跟踪止盈，与 trailing_stop_loss 类似，但它是在价格达到某个盈利水平后，动态设置止盈点以保证在价格下跌前锁定更多利润。 */
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
    commission_level: Option<CommissionLevel>,
    fund_fee_rate: Option<f64>,
    max_price_deviation: Option<f64>,
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
               commission_level: None,
               fund_fee_rate: None,
               max_price_deviation: None }
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
                           commission_level: self.commission_level.ok_or("commission level is required")?,
                           funding_rate: self.fund_fee_rate.ok_or("fund_fee_rate is required")?,
                           account_leverage_rate: Default::default(),
                           fees_book: Default::default(),
                           execution_mode: SandboxMode::Backtest,
                           max_price_deviation: self.max_price_deviation.ok_or("max price deviation is required")? })
    }
}
