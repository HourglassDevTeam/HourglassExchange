// FIXME : code below needs to be restructured and fitted to the framework. need to provide enums?
// CONSIDER: can these positions coexist, if so enums might not be ideal.

use crate::common_skeleton::balance::{TokenBalance};
use cerebro_integration::model::Exchange;
use serde::{Deserialize, Serialize};

use crate::common_skeleton::{friction::Fees, instrument::Instrument, Side};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Option<Vec<MarginPosition>>, // NOTE useless in backtest
    perpetual_pos: Option<Vec<PerpetualPosition>>,
    futures_pos: Option<Vec<FuturesPosition>>,
    option_pos: Option<Vec<OptionPosition>>,
}

#[derive(Clone, Debug)]
pub struct MarginPosition {}
#[derive(Clone, Debug)]
pub struct FuturesPosition {}
#[derive(Clone, Debug)]
pub struct OptionPosition {}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerpetualPositionConfig
{
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
    direction_mode: DirectionMode,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PositionMarginMode
{
    Cross,
    Isolated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DirectionMode
{
    Oneway,
    Double,
}

// NOTE 可能需要多种头寸类型共存
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PositionKind
{
    Perpetual(PerpetualPosition),
    // Margin(MarginPosition),
    // Futures(FuturesPosition),
    // Option(OptionPosition),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerpetualPosition
{
    pub base: PositionMeta,
    pub pos_config: PerpetualPositionConfig,
    pub liquidation_price: f64,
    pub margin: f64,
    pub funding_fee: f64,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta
{
    pub position_id: String,
    pub enter_ts: i64,
    pub update_ts: i64,
    pub exit_balance: TokenBalance, // NOTE necessary but unsure currently
    pub account_exchange_ts: i64,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub side: Side, // NOTE 注意和DirectionMode之间的兼容性
    pub current_size: f64,
    pub current_fees_total: Fees,
    pub current_avg_price_gross: f64,
    pub current_symbol_price: f64,
    pub current_avg_price: f64,
    pub unrealised_pnl: f64,
    pub realised_pnl: f64,
}
