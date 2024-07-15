// FIXME : code below needs to be restructured and fitted to the framework. need to provide enums?
// CONSIDER: can these positions coexist, if so enums might not be ideal.

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AccountPositions
{
    margin_pos: Vec<MarginPosition>,  // useless in backtest
    swap_pos: Vec<SwapPosition>,      // Note useful, and we're gonna build it
    // futures_pos: Vec<MarginPosition>,
    // option_pos: Vec<OptionPosition>,
}



// NOTE 如果确实需要多种头寸类型共存，可以考虑如下设计：
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Position {
    Swap(SwapPosition),
    // 如果未来需要添加其他类型，可以取消注释以下行：
    // Margin(MarginPosition),
    // Futures(FuturesPosition),
    // Option(OptionPosition),
}


#[derive(Clone, Debug)]
pub struct MarginPosition {}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SwapPosition
{
    token: String, // token字段，用于存储某种代币的名称，类型为String
    pos_config: SwapPositionConfig, // pos_config字段，用于存储与SwapPosition相关的配置，SwapPositionConfig类型
    pos_size: f64, // pos_size字段，表示头寸大小，类型为f64（64位浮点数）
    average_price: f64, // average_price字段，表示平均成交价格，类型为f64
    liquidation_price: f64, // liquidation_price字段，表示清算价格，类型为f64
    margin: f64, // margin字段，表示保证金比例，类型为f64
    pnl: f64, // pnl字段，表示未实现盈亏（Profit and Loss），类型为f64
    fee: f64, // fee字段，表示交易费用，类型为f64
    funding_fee: f64, // funding_fee字段，表示资金费用，类型为f64
    update_time_stamp: i64, // update_time_stamp字段，表示最后更新时间的时间戳，类型为i64（64位整数）
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
