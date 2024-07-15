#[derive(Clone, Debug)]
pub struct AccountConfig
{
    pub margin_mode: MarginMode,
    pub position_mode: PositionMode,
    pub commission_level: CommissionLevel,
    pub current_commission_rate: CommissionRates,
}

// NOTE 增加假设的佣金费率结构
#[derive(Clone, Debug)]
pub struct CommissionRates
{
    pub spot_maker: f64,
    pub spot_taker: f64,
    pub perpetual_open: f64,
    pub perpetual_close: f64,
    pub perpetual_funding: f64,
}

// NOTE 更新费率函数的样本：为 AccountConfig 添加一个方法来更新佣金费率
impl AccountConfig {
    // 更新当前佣金费率
    pub fn update_commission_rate(mut self, commission_rates: &CommissionRates) -> Self {
        self.current_commission_rate = match self.commission_level {
            CommissionLevel::Lv1 => CommissionRates {
                spot_maker: commission_rates.spot_maker * 0.9,
                spot_taker: commission_rates.spot_taker * 0.9,
                perpetual_open: commission_rates.perpetual_open * 0.9,
                perpetual_close: commission_rates.perpetual_close * 0.9,
                perpetual_funding: commission_rates.perpetual_funding * 0.9,
            },
            CommissionLevel::Lv2 => CommissionRates {
                spot_maker: commission_rates.spot_maker * 0.8,
                spot_taker: commission_rates.spot_taker * 0.8,
                perpetual_open: commission_rates.perpetual_open * 0.8,
                perpetual_close: commission_rates.perpetual_close * 0.8,
                perpetual_funding: commission_rates.perpetual_funding * 0.8,
            },
            CommissionLevel::Lv3 => CommissionRates {
                spot_maker: commission_rates.spot_maker * 0.7,
                spot_taker: commission_rates.spot_taker * 0.7,
                perpetual_open: commission_rates.perpetual_open * 0.7,
                perpetual_close: commission_rates.perpetual_close * 0.7,
                perpetual_funding: commission_rates.perpetual_funding * 0.7,
            },
            CommissionLevel::Lv4 => CommissionRates {
                spot_maker: commission_rates.spot_maker * 0.6,
                spot_taker: commission_rates.spot_taker * 0.6,
                perpetual_open: commission_rates.perpetual_open * 0.6,
                perpetual_close: commission_rates.perpetual_close * 0.6,
                perpetual_funding: commission_rates.perpetual_funding * 0.6,
            },
            CommissionLevel::Lv5 => CommissionRates {
                spot_maker: commission_rates.spot_maker * 0.5,
                spot_taker: commission_rates.spot_taker * 0.5,
                perpetual_open: commission_rates.perpetual_open * 0.5,
                perpetual_close: commission_rates.perpetual_close * 0.5,
                perpetual_funding: commission_rates.perpetual_funding * 0.5,
            },
        };
        self
    }
}

#[derive(Clone, Debug)]
pub enum MarginMode
{
    SimpleMode,
    SingleCurrencyMargin,
    MultiCurrencyMargin,
    PortfolioMargin,
}

#[derive(Clone, Debug)]
pub enum PositionMode
{
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

#[derive(Clone, Debug)]
pub enum CommissionLevel
{
    Lv1,
    Lv2,
    Lv3,
    Lv4,
    Lv5,
    // ..........
}

pub struct AccountConfigInitiator
{
    margin_mode: Option<MarginMode>,
    position_mode: Option<PositionMode>,
    commission_level: Option<CommissionLevel>,
}

impl AccountConfigInitiator
{
    pub fn new() -> Self
    {
        Self { margin_mode: None,
               position_mode: None,
               commission_level: None }
    }

    pub fn margin_mode(mut self, margin_mode: MarginMode) -> Self
    {
        self.margin_mode = Some(margin_mode);
        self
    }

    pub fn position_mode(mut self, position_mode: PositionMode) -> Self
    {
        self.position_mode = Some(position_mode);
        self
    }

    pub fn commission_level(mut self, commission_level: CommissionLevel) -> Self
    {
        self.commission_level = Some(commission_level);
        self
    }

    // NOTE initiated as all zeros. this could be replaced with actual values fetched from the exchange,
    //      be it simulated or not.
    pub fn initiate(self) -> Result<AccountConfig, &'static str>
    {
        Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
                           position_mode: self.position_mode.ok_or("position_mode is required")?,
                           commission_level: self.commission_level.ok_or("commission_level is required")?,
            current_commission_rate: CommissionRates {
                spot_maker: 0.0,
                spot_taker: 0.0,
                perpetual_open: 0.0,
                perpetual_close: 0.0,
                perpetual_funding: 0.0,
            },
        })
    }
}
