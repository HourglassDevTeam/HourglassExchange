#[derive(Clone, Debug)]
pub struct AccountConfig
{
    pub margin_mode: MarginMode,
    pub position_mode: PositionMode,
    pub commission_level: CommissionLevel,
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

    pub fn initiate(self) -> Result<AccountConfig, &'static str>
    {
        Ok(AccountConfig { margin_mode: self.margin_mode.ok_or("margin_mode is required")?,
                           position_mode: self.position_mode.ok_or("position_mode is required")?,
                           commission_level: self.commission_level.ok_or("commission_level is required")? })
    }
}
