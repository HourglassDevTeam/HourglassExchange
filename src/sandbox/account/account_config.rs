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
    pub margin_mode: MarginMode,                               // 保证金模式，用于定义账户的保证金类型（例如逐仓、全仓）
    pub global_position_direction_mode: PositionDirectionMode, // 持仓方向模式，定义账户是否允许同时持有多头和空头（双向持仓）或仅允许单一方向的仓位
    pub global_position_margin_mode: PositionMarginMode,       // 持仓保证金模式，定义每个仓位的保证金计算方式
    pub commission_level: CommissionLevel,                     // 手续费等级，决定账户的交易手续费率
    pub funding_rate: f64,                                     // 资金费率，用于合约交易中计算资金费用
    pub global_leverage_rate: f64,                             // 账户杠杆率，决定账户在杠杆交易中的放大倍数
    pub fees_book: HashMap<InstrumentKind, CommissionRates>,   // 手续费表，存储每种合约类型的手续费率
    pub execution_mode: SandboxMode,                           // 执行模式，定义账户是在沙盒模式（模拟交易）还是在真实环境中运行
    pub max_price_deviation: f64,                              // 最大价格偏差，用于限制订单价格与市场价格的偏离范围
    pub lazy_account_positions: bool,                          // 是否惰性更新以节约性能
    pub liquidation_threshold: f64,                            // 平仓的门槛，通常为一个0.9~1的系数
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
        Self { maker_fees: None, taker_fees: None }
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

pub trait FeesQuerier
{
    fn get_maker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>;

    fn get_taker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>;
}

pub trait ConfigLoader
{
    fn load_config() -> Result<AccountConfig, ExchangeError>;
}

impl ConfigLoader for AccountConfig
{
    fn load_config() -> Result<AccountConfig, ExchangeError>
    {
        read_config_file()
    }
}

impl FeesQuerier for AccountConfig
{
    fn get_maker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>
    {
        self.fees_book
            .get(instrument_kind)
            .map(|rates| rates.maker_fees)
            .ok_or_else(|| ExchangeError::SandBox(format!("Open fee rate for {:?} not found", instrument_kind)))
    }

    fn get_taker_fee_rate(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExchangeError>
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

pub struct AccountConfigBuilder
{
    margin_mode: Option<MarginMode>,
    position_mode: Option<PositionDirectionMode>,
    commission_level: Option<CommissionLevel>,
    fund_fee_rate: Option<f64>,
    global_leverage_rate: Option<f64>,
    fees_book: Option<HashMap<InstrumentKind, CommissionRates>>,
    execution_mode: Option<SandboxMode>,
    max_price_deviation: Option<f64>,
    lazy_account_positions: Option<bool>,
    liquidation_threshold: Option<f64>,
}

impl Default for AccountConfigBuilder
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl AccountConfigBuilder
{
    pub fn new() -> Self
    {
        Self { margin_mode: None,
               position_mode: None,
               commission_level: None,
               fund_fee_rate: None,
               global_leverage_rate: None,
               fees_book: None,
               execution_mode: None,
               max_price_deviation: None,
               lazy_account_positions: None,
               liquidation_threshold: None }
    }

    pub fn margin_mode(mut self, margin_mode: MarginMode) -> Self
    {
        self.margin_mode = Some(margin_mode);
        self
    }

    pub fn max_price_deviation(mut self, max_price_deviation: f64) -> Self
    {
        self.max_price_deviation = Some(max_price_deviation);
        self
    }

    pub fn position_direction_mode(mut self, position_direction_mode: PositionDirectionMode) -> Self
    {
        self.position_mode = Some(position_direction_mode);
        self
    }

    pub fn commission_level(mut self, commission_level: CommissionLevel) -> Self
    {
        self.commission_level = Some(commission_level);
        self
    }

    /// NOTE 这里可以设置合法的`funding_rate`限制
    pub fn funding_rate(mut self, funding_rate: f64) -> Result<Self, ExchangeError>
    {
        // 假设资金费率的合理范围是 -0.003 到 0.003
        if funding_rate >= -0.003 && funding_rate <= 0.003 {
            self.fund_fee_rate = Some(funding_rate);
            Ok(self)
        }
        else {
            Err(ExchangeError::SandBox("Invalid funding rate".into()))
        }
    }

    /// NOTE 这里可以设置合法的`global_leverage_rate`限制
    pub fn global_leverage_rate(mut self, global_leverage_rate: f64) -> Result<Self, ExchangeError>
    {
        // 假设杠杆率的合理范围是 1 到 100
        if global_leverage_rate >= 1.0 && global_leverage_rate <= 100.0 {
            self.global_leverage_rate = Some(global_leverage_rate);
            Ok(self)
        }
        else {
            Err(ExchangeError::SandBox("Invalid global leverage rate".into()))
        }
    }

    pub fn fees_book(mut self, fees_book: HashMap<InstrumentKind, CommissionRates>) -> Self
    {
        self.fees_book = Some(fees_book);
        self
    }

    pub fn execution_mode(mut self, execution_mode: SandboxMode) -> Self
    {
        self.execution_mode = Some(execution_mode);
        self
    }

    pub fn lazy_account_positions(mut self, lazy_account_positions: bool) -> Self
    {
        self.lazy_account_positions = Some(lazy_account_positions);
        self
    }

    /// NOTE 这里可以设置合法的`liquidation_threshold`限制
    pub fn liquidation_threshold(mut self, liquidation_threshold: f64) -> Result<Self, ExchangeError>
    {
        if liquidation_threshold >= 0.8 && liquidation_threshold <= 0.999 {
            self.liquidation_threshold = Some(liquidation_threshold);
            Ok(self)
        }
        else {
            Err(ExchangeError::SandBox("input liquidation threshold invalid.".into()))
        }
    }

    pub fn initiate(self) -> Result<AccountConfig, &'static str>
    {
        Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
                           global_position_direction_mode: self.position_mode.ok_or("position_mode is required")?,
                           global_position_margin_mode: PositionMarginMode::Cross,
                           commission_level: self.commission_level.ok_or("commission level is required")?,
                           funding_rate: self.fund_fee_rate.ok_or("fund_fee_rate is required")?,
                           global_leverage_rate: Default::default(),
                           fees_book: Default::default(),
                           execution_mode: SandboxMode::Backtest,
                           max_price_deviation: self.max_price_deviation.ok_or("max price deviation is required")?,
                           lazy_account_positions: self.lazy_account_positions.ok_or("lazy_account_positions switch is required")?,
                           liquidation_threshold: self.liquidation_threshold.ok_or("liquidation threshold is required")? })
    }
}
