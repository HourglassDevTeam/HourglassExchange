use serde::{Deserialize, Serialize};
use crate::common::account_positions::position_id::PositionId;

/// [`Position`] 更新事件。该事件发生在接收到新的 [`MarketEvent`] 数据时。
///
/// # 结构体概述
/// `PositionDelta` 结构体用于描述仓位在接收到市场数据更新时发生的变化。它包含与仓位相关的关键信息，如当前市场价格、当前仓位的总价值以及未实现盈亏等。
///
/// 该结构体在量化交易系统中至关重要，因为它能够实时跟踪仓位的变化，特别是在价格波动的情况下，帮助交易者和策略及时调整仓位。
///
/// # 字段说明
/// - `position_id`: [`Position`] 的唯一标识符，由交易所、交易符号以及进入时间生成，确保每个仓位都有一个唯一的 ID。
/// - `update_time`: 更新事件的时间戳，记录该次更新发生的时间，通常表示市场事件发生时的时间。
/// - `current_symbol_price`: 当前交易标的（symbol）的收盘价格，即市场上最新的价格。
/// - `current_value_gross`: 当前仓位的总价值，计算方式为仓位的绝对数量乘以当前的市场价格（`abs(Quantity) * current_symbol_price`）。该字段不考虑费用或其他调整，仅反映仓位的粗略市值。
/// - `unrealised_profit_loss`: 仓位未实现的盈亏，表示在仓位未平仓的情况下，基于当前市场价格计算的盈亏。这是一个浮动值，随着市场价格的变化而变化。
///
/// # 用途
/// `PositionDelta` 主要用于实时更新仓位的状态，使得策略或交易者能够在市场变化时及时调整仓位或进行风控。
///
/// 在接收到市场事件（如价格变化、订单执行等）后，系统会生成对应的 `PositionDelta` 实例并传递给相关组件或服务，以更新仓位的状态。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionDelta {
    /// [`Position`] 的唯一标识符，由交易所、交易符号以及进入时间生成。
    pub position_id: PositionId,
    /// 更新事件的时间戳，记录该次更新发生的时间。
    pub update_time: i64,
    /// NOTE 数量的变化,这里暂定是有方向的。
    pub size_delta:f64,
    /// 当前交易标的（symbol）的收盘价格，即市场上最新的价格。
    pub current_symbol_price: f64,
    /// 当前仓位的总价值，计算方式为 abs(Quantity) * current_symbol_price。
    pub current_value_gross: f64,
    /// 未实现的盈亏，表示在仓位未平仓的情况下基于当前市场价格计算的盈亏。
    pub unrealised_profit_loss: f64,
}
