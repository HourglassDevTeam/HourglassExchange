// FIXME : code below needs to be restructured and fitted to the framework. need to provide enums?
// CONSIDER: can these positions coexist, if so enums might not be ideal.

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Vec<MarginPosition>, // useless in backtest
    swap_pos: Vec<SwapPosition>,
    // Note useful, and we're gonna build it
    // futures_pos: Vec<FuturesPosition>,
    // option_pos: Vec<OptionPosition>,
}

#[derive(Clone, Debug)]
pub struct MarginPosition {}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SwapPosition
{
    token: String,                  // token字段，用于存储某种代币的名称，类型为String
    pos_config: SwapPositionConfig, // pos_config字段，用于存储与SwapPosition相关的配置，SwapPositionConfig类型
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

// NOTE 如果确实需要多种头寸类型共存，可以考虑如下设计：
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Position
{
    Swap(SwapPosition),
    Margin(MarginPosition),
    // Futures(FuturesPosition),
    // Option(OptionPosition),
}

// NOTE 此处为一个尽量详细的元Position案例
// #[derive(Debug, Default, Clone)]
// pub struct MetaPosition{
//     // 交易所时间，使用DateTime<Utc>类型，表示与交易所同步的UTC时间
//     pub exchange_time: DateTime<Utc>,
//     // 头寸ID，Option类型，表示可能不存在头寸ID的情况
//     pub position_id: Option<PositionID>,
//     // 交易所信息，Option类型，表示头寸可能没有具体交易所的标记
//     pub exchange: Option<Exchange>,
//     // 交易工具，Option类型，表示头寸相关的交易工具或资产
//     pub instrument: Option<Instrument>,
//     // 头寸元数据，Option类型，包含头寸的额外信息
//     pub meta: Option<PositionMeta>,
//     // 交易方向，Option类型，可以是多头(buy)或空头(sell)
//     pub side: Option<Side>,
//     // 头寸数量，Option类型，表示持有的资产数量
//     pub quantity: Option<f64>,
//     // 入场交易费用，Option类型，表示交易时产生的费用
//     pub enter_fees: Option<CryptoFriction>,
//     // 入场总费用，Option类型，表示入场交易的总费用
//     pub enter_fees_total: Option<Fees>,
//     // 入场平均价格（毛），Option类型，表示考虑交易费用后的平均入场价格
//     pub enter_avg_price_gross: Option<f64>,
//     // 入场价值（毛），Option类型，表示头寸入场时的总价值
//     pub enter_value_gross: Option<f64>,
//     // 退出交易费用，Option类型，表示退出头寸时产生的费用
//     pub exit_fees: Option<CryptoFriction>,
//     // 退出总费用，Option类型，表示退出交易的总费用
//     pub exit_fees_total: Option<Fees>,
//     // 退出平均价格（毛），Option类型，表示考虑交易费用后的退出平均价格
//     pub exit_avg_price_gross: Option<f64>,
//     // 退出价值（毛），Option类型，表示退出头寸时的总价值
//     pub exit_value_gross: Option<f64>,
//     // 当前市场价格，Option类型，表示当前头寸资产的市场价格
//     pub current_symbol_price: Option<f64>,
//     // 当前价值（毛），Option类型，表示头寸当前的总价值
//     pub current_value_gross: Option<f64>,
//     // 未实现盈亏，Option类型，表示头寸当前的未实现利润或亏损
//     pub unrealised_profit_loss: Option<f64>,
//     // 已实现盈亏，Option类型，表示头寸已退出部分的利润或亏损
//     pub realised_profit_loss: Option<f64>,
// }
