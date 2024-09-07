use crate::common::{trade::ClientTrade, Side};
use serde::{Deserialize, Serialize};

/// [`Position`] 更新事件。该事件发生在接收到新的 [`MarketEvent`] 数据时。
///
/// # 结构体概述
/// `PositionDelta` 结构体用于描述仓位在接收到市场数据更新时发生的变化。它包含与仓位相关的关键信息，如当前市场价格、当前仓位的总价值以及未实现盈亏等。
///
/// 该结构体在量化交易系统中至关重要，因为它能够实时跟踪仓位的变化，特别是在价格波动的情况下，帮助交易者和策略及时调整仓位。
///
/// # 字段说明
/// - `side`: 仓位的方向，例如买入`Side::Buy`或卖出`Side::Sell` // NOTE 不确定需不需要。作为验证字段而存在。
/// - `update_time`: 仓位更新的时间戳，通常是一个 UNIX 时间戳，表示最后一次更新的时间
/// - `size`: 待处理的仓位变化量的大小，通常以资产的数量表示
/// - `current_symbol_price`: 当前交易标的的市场价格，可能是买入价或卖出价
///
/// # 用途
/// `PositionDelta` 主要用于实时更新仓位的状态，使得策略或交易者能够在市场变化时及时调整仓位或进行风控。
///
/// 在接收到市场事件（如价格变化、订单执行等）后，系统会生成对应的 `PositionDelta` 实例并传递给相关组件或服务，以更新仓位的状态。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionDelta
{
    // 仓位的方向，例如买入（Side::Buy）或卖出（Side::Sell）
    // pub side: Side,
    // 交易标的的详细信息，包括基础资产和报价资产
    // pub instrument: Instrument,
    pub side: Side,                // 静态数据
    pub update_time: i64,          // 实时数据
    pub size: f64,                 // 实时数据
    pub current_symbol_price: f64, // 实时数据
}

impl From<&ClientTrade> for PositionDelta
{
    /// 从 `ClientTrade` 转换为 `PositionDelta`
    ///
    /// # 参数
    /// - `client_trade`: 包含交易的详细信息。
    ///
    /// # 返回值
    /// 返回一个新的 `PositionDelta`，其中包含交易时的相关数据，如更新的时间、交易数量、当前市场价格等。
    fn from(client_trade: &ClientTrade) -> Self
    {
        PositionDelta { side: client_trade.side,
                        update_time: client_trade.timestamp,      // 使用交易的时间戳作为更新时间
                        size: client_trade.size,                  // 使用交易的数量作为仓位的变化量
                        current_symbol_price: client_trade.price  /* 使用交易的价格作为当前市场价格 */ }
    }
}
