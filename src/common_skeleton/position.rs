// FIXME : code below needs to be restructured and fitted to the framework. need to provide enums?
// CONSIDER: can these positions coexist, if so enums might not be ideal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::common_skeleton::balance::Balance;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Vec<MarginPosition>, // useless in backtest NOTE what exactly is this
    swap_pos: Vec<PerpetualPosition>,
    // futures_pos: Vec<FuturesPosition>,
    // option_pos: Vec<OptionPosition>,
}

#[derive(Clone, Debug)]
pub struct MarginPosition {}
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
pub enum Position
{
    Perpetual(PerpetualPosition),
    // Margin(MarginPosition),
    // Futures(FuturesPosition),
    // Option(OptionPosition),
}


/// 包含与进入、更新和退出 [`Position`] 相关的跟踪UUID和时间戳的元数据。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta
{
    pub enter_time: i64,
    pub update_time: i64,
    pub exit_balance: Option<Balance>,
}


// NOTE 此处为一个尽量详细的Position案例
// #[derive(Debug, Default, Clone)]
// pub struct MetaPosition{
//     // 交易所时间，使用DateTime<Utc>类型，表示与交易所同步的UTC时间
//     pub exchange_time: DateTime<Utc>,
//     // 头寸ID，Option类型，表示可能不存在头寸ID的情况
//     pub position_id: Option<PositionID>,
//     pub exchange: Option<Exchange>,
//     pub instrument: Option<Instrument>,
//     pub meta: Option<PositionMeta>, NOTE Cross comparison is due here.
//     pub side: Option<Side>,
//     pub quantity: Option<f64>,
//     pub enter_fees: Option<CryptoFriction>,
//     pub enter_fees_total: Option<Fees>,
//     pub enter_avg_price_gross: Option<f64>,
//     pub enter_value_gross: Option<f64>,
//     pub exit_fees: Option<CryptoFriction>,
//     pub exit_fees_total: Option<Fees>,
//     pub exit_avg_price_gross: Option<f64>,
//     pub exit_value_gross: Option<f64>,
//     pub current_symbol_price: Option<f64>,
//     pub current_value_gross: Option<f64>,
//     pub unrealised_profit_loss: Option<f64>,
//     pub realised_profit_loss: Option<f64>,
// }
