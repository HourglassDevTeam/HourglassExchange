use serde::{Deserialize, Serialize};
use crate::common::account_positions::position_id::PositionId;
use crate::common::balance::Balance;
use crate::common::instrument::Instrument;
use crate::common::Side;
use crate::Exchange;

/// NOTE 这是初步的平仓仓位数据结构设计，可能需要更改。
///
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionExit {
    pub exchange: Exchange,           // 静态数据
    pub instrument: Instrument,       // 静态数据
    pub side: Side,                   // 静态数据
    pub position_id: PositionId,    // [`Position`]的唯一标识符，由交易工具和进入时间戳生成。
    pub exit_ts: i64,    // 触发 [`Position`] 平仓的 [`Order`] 的时间戳。
    pub exit_balance: Balance,    // 在退出 [`Position`] 时计算的投资组合 [`Balance`]。
    pub exit_fees: f64,    // 退出 [`Position`] 时产生的所有费用类型及其关联的 [`fee`]。
    pub exit_fees_total: f64,  // 退出时产生的总费用。进入 [`Position`] 时 [`Fees`] 中每个 [`FeeAmount`] 的总和。
    pub exit_avg_price_gross: f64,     // 不包含 exit_fees_total 的退出平均价格。
    pub exit_value_gross: f64,     // abs(数量) * exit_avg_price_gross。
    pub realised_pnl: f64,     // [`Position`] 退出后实现的盈亏。
}