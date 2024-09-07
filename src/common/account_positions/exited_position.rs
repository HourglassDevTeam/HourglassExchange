use serde::{Deserialize, Serialize};
use crate::common::account_positions::position_id::PositionId;
use crate::common::account_positions::position_meta::PositionMeta;
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
    pub position_id: PositionId,    // 仓位的唯一标识符，由交易工具和进入时间戳生成。
    pub exit_ts: i64,    // 触发 仓位平仓的 [`Order`] 的时间戳。
    pub exit_balance: Balance,    // 在退出仓位时计算的投资组合 [`Balance`]。
    pub exit_fees: f64,    // 退出仓位时产生的所有费用类型及其关联的费用。
    pub exit_fees_total: f64,  /// 退出时产生的总费用。进入仓位时费用中每个费用的总和。
    pub exit_avg_price_gross: f64,     // 不包含 exit_fees_total 的退出平均价格。
    pub exit_value_gross: f64,     // abs(数量) * exit_avg_price_gross。
    pub realised_pnl: f64,     // 退出后实现的盈亏。
}


#[allow(dead_code)]
impl PositionExit {
    /// 从 `PositionMeta` 创建 `PositionExit`
    ///
    /// # 参数
    /// - `position_meta`: 包含仓位的所有元数据。
    /// - `exit_ts`: 平仓时间戳。
    /// - `exit_price`: 平仓时的价格。
    /// - `exit_quantity`: 平仓的数量。
    ///
    /// # 返回值
    /// 返回一个新的 `PositionExit`，其中包含从 `PositionMeta` 中提取的静态数据和退出时的相关信息。
    pub fn from_position_meta(
        position_meta: &PositionMeta,
        exit_ts: i64,
        exit_price: f64,
        exit_quantity: f64,
    ) -> Self {
        // 计算退出时的总价值（不考虑费用）
        let exit_value_gross = exit_quantity.abs() * exit_price; // NOTE 不前不确定是否带符号。

        // 计算实现盈亏 (realised_pnl)
        let realised_pnl = (exit_price - position_meta.current_avg_price) * exit_quantity; // NOTE 不前不确定。

        // 创建 `PositionExit`
        PositionExit {
            exchange: position_meta.exchange.clone(),               // 从 PositionMeta 获取静态数据
            instrument: position_meta.instrument.clone(),           // 从 PositionMeta 获取静态数据
            side: position_meta.side,                               // 从 PositionMeta 获取静态数据
            position_id: position_meta.position_id.clone(),         // 获取仓位的唯一标识符
            exit_ts,                                                // 应该使用推出时候的交易时间辍
            exit_balance: Balance::new(exit_quantity, exit_value_gross, realised_pnl),  // 计算平仓时的余额信息 NOTE 不前不确定。
            exit_fees: position_meta.current_fees_total,             // 使用 PositionMeta 中累计的费用
            exit_fees_total: position_meta.current_fees_total,       // 平仓时的总费用
            exit_avg_price_gross: exit_price,                       // 平仓时的价格
            exit_value_gross,                                       // 平仓时的总价值
            realised_pnl,                                           // 实现的盈亏
        }
    }
}