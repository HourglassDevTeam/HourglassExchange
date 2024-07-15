// FIXME code below needs to be restructured and fitted to the framework.

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Vec<MarginPosition>,  // useless in backtest
    swap_pos: Vec<SwapPosition>,      // Note useful, and we gonna build it
    futures_pos: Vec<MarginPosition>, // useless
    option_pos: Vec<OptionPosition>,  // useless
}

#[derive(Clone, Debug)]
pub struct MarginPosition {}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SwapPosition
{
    token: String,
    pos_config: SwapPositionConfig,
    pos_size: f64,
    average_price: f64,
    liquidation_price: f64,
    margin: f64,
    pnl: f64,
    fee: f64,
    funding_fee: f64,
    update_time_stamp: i64,
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SwapPositionConfig
{
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
}

#[derive(Clone, Debug)]
pub enum PositionMarginMode
{
    Cross,
    Isolated,
}

#[derive(Clone, Debug)]
pub struct FuturesPosition {}

#[derive(Clone, Debug)]
pub struct OptionPosition {}
