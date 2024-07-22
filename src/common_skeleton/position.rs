// FIXME : code below needs to be restructured and fitted to the framework. need to provide enums?
// CONSIDER: can these positions coexist, if so enums might not be ideal.

use cerebro_integration::model::Exchange;
use serde::{Deserialize, Serialize};

use crate::common_skeleton::balance::Balance;
use crate::common_skeleton::friction::Fees;
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::Side;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Option<Vec<MarginPosition>>, // useless in backtest NOTE what exactly is this
    swap_pos: Option<Vec<PerpetualPosition>>,
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
#[derive(Clone, Debug)]
pub struct PerpetualPositionConfig
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

// NOTE 如果确实需要多种头寸类型共存，可以考虑如下设计：
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum PositionKind
{
    Perpetual(PerpetualPosition),
    // Margin(MarginPosition),
    // Futures(FuturesPosition),
    // Option(OptionPosition),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PerpetualPosition
{
    token: String,                  // token字段，用于存储某种代币的名称，类型为String
    pos_config: PerpetualPositionConfig, // pos_config字段，用于存储与SwapPosition相关的配置，SwapPositionConfig类型
    pos_size: f64,                  // pos_size字段，表示头寸大小，类型为f64（64位浮点数）
    average_price: f64,             // average_price字段，表示平均成交价格，类型为f64
    liquidation_price: f64,         // liquidation_price字段，表示清算价格，类型为f64
    margin: f64,                    // margin字段，表示保证金比例，类型为f64
    pnl: f64,                       // pnl字段，表示未实现盈亏（Profit and Loss），类型为f64
    fee: f64,                       // fee字段，表示交易费用，类型为f64
    funding_fee: f64,               // funding_fee字段，表示资金费用，类型为f64
    update_time_stamp: i64,         // update_time_stamp字段，表示最后更新时间的时间戳，类型为i64（64位整数）
}
// NOTE 此处为一个尽量详细的Position案例
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Position {
    pub position_id: String,
    pub enter_ts: i64,
    pub update_ts: i64,
    pub exit_balance: Balance,
    pub current_exchange_ts: i64,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub side: Side,
    pub quantity: f64,
    pub enter_fees: Fees,
    pub enter_fees_total: Fees,
    pub enter_avg_price_gross: f64,
    pub enter_value_gross: f64,
    pub exit_fees: Fees,
    pub exit_fees_total: Fees,
    pub exit_avg_price_gross: f64,
    pub exit_value_gross: f64,
    pub current_symbol_price: f64,
    pub current_value_gross: f64,
    pub unrealised_pnl: f64,
    pub realised_pnl: f64,
}
