use crate::Deserialize;
use chrono::{DateTime, Utc};
use serde::Serialize;
// use serde::{Deserialize, Serialize};
// use crate::common::balance::Balance;
// use crate::common::position::Position;
// use crate::dashboard::summary::PositionSummariser;

/// `EquitySnapshot` 结构体表示在某个时间点上的总权益值，与 [`Balance.total`](Balance) 对应。
///
/// # 结构体概述
/// `EquitySnapshot` 用于记录在某个特定时间点上，投资组合或交易账户的总权益（即所有资产的总价值）。它包括两个字段：
/// - `time`：记录时间戳，表示该总权益值是在什么时候记录的。
/// - `total`：总权益值，以 `f64` 类型表示。
///
/// 这个结构体在量化交易和投资组合管理中非常重要，因为它提供了一个准确的历史记录，帮助分析和跟踪账户的资金波动情况。
///
/// # 主要用途
/// - **回撤计算**：在回撤分析中，`EquitySnapshot` 可以用来识别账户价值从一个峰值到谷底的下降幅度，从而计算最大回撤（Max Drawdown）等重要风险指标。
/// - **绩效评估**：通过记录多个时间点的 `EquitySnapshot`，可以分析投资组合在不同时间段的表现，包括计算收益率、波动性等指标。
/// - **时间序列分析**：`EquitySnapshot` 可以作为时间序列数据的一部分，用于分析权益值随时间变化的趋势，从而帮助做出更好的投资决策。

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct EquitySnapshot
{
    pub time: DateTime<Utc>,
    pub total: f64,
}

impl Default for EquitySnapshot
{
    /// 默认构造函数，初始化 `EquitySnapshot`，将 `time` 设置为当前时间，`total` 初始化为 `0.0`。
    fn default() -> Self
    {
        Self { time: Utc::now(), total: 0.0 }
    }
}

// impl From<Balance> for EquitySnapshot {
//     fn from(balance: Balance) -> Self {
//         Self {
//             time: balance.time,
//             total: balance.total,
//         }
//     }
// }
//
// impl PositionSummariser for EquitySnapshot {
//     /// Updates using the input [`Position`]'s PnL & associated timestamp.
//     fn update(&mut self, position: &Position) {
//         match position.meta.exit_balance {
//             None => {
//                 // Position is not exited, so simulate
//                 self.time = position.meta.update_time;
//                 self.total += position.unrealised_profit_loss;
//             }
//             Some(exit_balance) => {
//                 self.time = exit_balance.time;
//                 self.total += position.realised_profit_loss;
//             }
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::test_util::position;
//     use chrono::Duration;
//     use std::ops::Add;
//
//     #[test]
//     fn equity_point_update() {
//         fn equity_update_position_closed(exit_time: DateTime<Utc>, result_pnl: f64) -> Position {
//             let mut position = position();
//             position.meta.exit_balance = Some(Balance {
//                 time: exit_time,
//                 total: 100.0,
//                 available: 100.0,
//             });
//             position.realised_profit_loss = result_pnl;
//             position
//         }
//
//         fn equity_update_position_open(
//             last_update_time: DateTime<Utc>,
//             unreal_pnl: f64,
//         ) -> Position {
//             let mut position = position();
//             position.meta.exit_balance = None;
//             position.meta.update_time = last_update_time;
//             position.unrealised_profit_loss = unreal_pnl;
//             position
//         }
//
//         struct TestCase {
//             position: Position,
//             expected_equity: f64,
//             expected_time: DateTime<Utc>,
//         }
//
//         let base_time = Utc::now();
//
//         let mut equity_point = EquitySnapshot {
//             time: base_time,
//             total: 100.0,
//         };
//
//         let test_cases = vec![
//             TestCase {
//                 position: equity_update_position_closed(base_time.add(Duration::days(1)), 10.0),
//                 expected_equity: 110.0,
//                 expected_time: base_time.add(Duration::days(1)),
//             },
//             TestCase {
//                 position: equity_update_position_open(base_time.add(Duration::days(2)), -10.0),
//                 expected_equity: 100.0,
//                 expected_time: base_time.add(Duration::days(2)),
//             },
//             TestCase {
//                 position: equity_update_position_closed(base_time.add(Duration::days(3)), -55.9),
//                 expected_equity: 44.1,
//                 expected_time: base_time.add(Duration::days(3)),
//             },
//             TestCase {
//                 position: equity_update_position_open(base_time.add(Duration::days(4)), 68.7),
//                 expected_equity: 112.8,
//                 expected_time: base_time.add(Duration::days(4)),
//             },
//             TestCase {
//                 position: equity_update_position_closed(base_time.add(Duration::days(5)), 99999.0),
//                 expected_equity: 100111.8,
//                 expected_time: base_time.add(Duration::days(5)),
//             },
//             TestCase {
//                 position: equity_update_position_open(base_time.add(Duration::days(5)), 0.2),
//                 expected_equity: 100112.0,
//                 expected_time: base_time.add(Duration::days(5)),
//             },
//         ];
//
//         for (index, test) in test_cases.into_iter().enumerate() {
//             equity_point.update(&test.position);
//             let equity_diff = equity_point.total - test.expected_equity;
//             assert!(equity_diff < 1e-10, "Test case {} failed at assert", index);
//             assert_eq!(
//                 equity_point.time, test.expected_time,
//                 "Test case {} failed to assert_eq",
//                 index
//             );
//         }
//     }
// }

pub mod drawdown;
pub mod ratio;
