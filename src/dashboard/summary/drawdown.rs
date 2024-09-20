// use crate::{
//     common::account_positions::Position,
//     dashboard::{
//         metrics::{
//             drawdown::{AvgDrawdown, Drawdown, MaxDrawdown},
//             EquitySnapshot,
//         },
//         summary::{PositionSummariser, TableBuilder},
//     },
// };
// /// `DrawdownSummary` 模块用于计算和跟踪投资组合或交易策略中的最大回撤、平均回撤以及它们的持续时间。
// ///
// /// # 模块概述
// /// `DrawdownSummary` 是一个综合工具，用于跟踪交易头寸的表现，特别关注下行风险。它主要计算三个关键指标：
// /// - **当前回撤 (`current_drawdown`)**：正在发生的从峰值到当前价值的下降幅度。
// /// - **最大回撤 (`max_drawdown`)**：历史上最大的一次从峰值到谷底的下降幅度。
// /// - **平均回撤 (`avg_drawdown`)**：在给定时间段内，所有回撤的平均值和持续时间。
// ///
// /// 这些指标对于评估投资策略的风险非常重要，尤其是在评估下行波动性时。较大的回撤意味着策略可能会经历剧烈的价值波动，而较长的回撤持续时间则意味着投资者可能需要更长时间才能恢复到峰值。
// ///
// /// # 计算原理
// /// 1. **EquitySnapshot 计算**：每当一个交易头寸结束时（即 `exit_balance` 已经确定），我们就会计算一个 `EquitySnapshot`，它表示在该时间点的总权益。这包括头寸的结束时间（这里暂时使用当前时间 `Utc::now()`）和总金额。
// ///
// /// 2. **更新回撤 (`update`)**：当一个新的 `EquitySnapshot` 被生成时，我们使用它来更新当前回撤。如果当前回撤结束（即策略从谷底恢复到新的峰值），我们会更新最大回撤和平均回撤。
// ///    - **当前回撤**：跟踪正在进行中的回撤，并在它结束时记录其最终数值。
// ///    - **最大回撤**：记录历史上最深的回撤，这对于评估策略的最坏情况非常重要。
// ///    - **平均回撤**：计算一段时间内所有回撤的平均值和持续时间，这有助于理解策略的典型下行表现。
// ///
// /// 3. **TableBuilder 实现**：为 `DrawdownSummary` 提供了一个表格生成功能，使这些关键指标可以清晰地展示出来。表格标题包含最大回撤、最大回撤天数、平均回撤和平均回撤天数。这些信息对于投资者或策略开发者来说非常重要。
// ///
// /// # 时间戳的选择
// /// `EquitySnapshot` 的时间戳字段 `time` 目前设置为 `Utc::now()`，即当前时间。这是为了简单起见，如果 `PositionMeta` 中包含了更合适的历史时间戳（如头寸关闭的时间），则应使用该时间戳，以更准确地反映回撤的发生时间。这种方法允许更精确的回撤分析，使得这些数据能够更好地反映策略的历史表现。
// ///
// /// # 用途
// /// 该模块特别适用于量化交易和投资组合管理中的风险评估。通过跟踪并分析回撤，投资者可以更好地理解他们的策略在不同市场条件下的表现，并做出相应的调整，以优化回报与风险的平衡。
// use chrono::Utc;
// use prettytable::{row, Row};
// use serde::{Deserialize, Serialize};
//
// #[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
// pub struct DrawdownSummary
// {
//     pub current_drawdown: Drawdown,
//     pub avg_drawdown: AvgDrawdown,
//     pub max_drawdown: MaxDrawdown,
// }
//
// /// FIXME 这里的time到底输入历史时间戳还是实时时间戳？？？
// impl PositionSummariser for DrawdownSummary
// {
//     fn update(&mut self, position: &Position)
//     {
//         //  Only update DrawdownSummary with closed Positions
//         // 通过模式匹配获取不同头寸类型的 `exit_balance` 和时间戳，并构造 `EquitySnapshot`
//         let equity_point = match position {
//             // 如果是 Perpetual 类型头寸
//             | Position::Perpetual(pos) => EquitySnapshot { time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
//                                                            total: pos.meta.exit_balance.balance.total },
//             // 如果是 LeveragedToken 类型头寸
//             | Position::LeveragedToken(pos) => EquitySnapshot { time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
//                                                                 total: pos.meta.exit_balance.balance.total },
//             // 如果是 Future 类型头寸
//             | Position::Future(pos) => EquitySnapshot { time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
//                                                         total: pos.meta.exit_balance.balance.total },
//             // 如果是 Option 类型头寸
//             | Position::Option(pos) => EquitySnapshot { time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
//                                                         total: pos.meta.exit_balance.balance.total },
//         };
//
//         // 更新 DrawdownSummary
//         if let Some(ended_drawdown) = self.current_drawdown.update(equity_point) {
//             self.avg_drawdown.update(&ended_drawdown);
//             self.max_drawdown.update(&ended_drawdown);
//         }
//     }
// }
//
// /// `DrawdownSummary` 模块用于跟踪和汇总投资组合或交易策略中的回撤情况，包括最大回撤、平均回撤及其持续时间。
// ///
// /// # 模块功能
// /// `DrawdownSummary` 提供了一套综合性指标来评估策略的下行风险，包括：
// /// - **最大回撤 (`max_drawdown`)**：历史上从峰值到谷底的最大跌幅，反映策略的最糟糕表现。
// /// - **平均回撤 (`avg_drawdown`)**：在多个回撤周期中的平均值，帮助了解策略的典型表现。
// /// - **回撤持续时间**：计算回撤从开始到恢复的持续时间，评估策略从损失中恢复所需的时间。
// ///
// /// 这些指标可以通过 `TableBuilder` 生成表格形式的结果，方便直观地展示策略的风险情况。
// ///
// /// # 计算原理
// /// - **实时更新**：每当交易头寸结束时，会生成一个 `EquitySnapshot`，表示当前总权益，并用于更新当前回撤的状态。如果发现策略从低谷恢复到新的峰值，当前回撤就结束，会记录并更新最大和平均回撤。
// ///
// /// # 实现细节
// /// 实现了 `PositionSummariser` 和 `TableBuilder` 接口，分别用于更新回撤信息和生成表格输出。
//
// impl TableBuilder for DrawdownSummary
// {
//     /// `titles` 方法用于定义表格的列标题
//     ///
//     /// # 返回值
//     /// 返回一个包含表格列标题的 `Row` 对象：
//     /// - "Max Drawdown"：最大回撤金额。
//     /// - "Max Drawdown Days"：最大回撤的持续天数。
//     /// - "Avg. Drawdown"：平均回撤金额。
//     /// - "Avg. Drawdown Days"：平均回撤的持续天数。
//     ///
//     /// 这些标题帮助清晰地展示每个回撤相关的关键指标。
//     fn titles(&self) -> Row
//     {
//         row!["Max Drawdown", "Max Drawdown Days", "Avg. Drawdown", "Avg. Drawdown Days",]
//     }
//
//     /// `row` 方法用于生成当前 `DrawdownSummary` 的表格数据行。
//     ///
//     /// # 返回值
//     /// 返回一个包含当前最大回撤、回撤天数、平均回撤和平均回撤天数的 `Row` 对象：
//     /// - 使用 `self.max_drawdown.drawdown.drawdown` 获取最大回撤金额，并格式化为小数点后三位。
//     /// - 使用 `self.max_drawdown.drawdown.duration.num_days()` 获取最大回撤持续的天数。
//     /// - 使用 `self.avg_drawdown.mean_drawdown` 获取平均回撤金额，并格式化为小数点后三位。
//     /// - 使用 `self.avg_drawdown.mean_duration.num_days()` 获取平均回撤的持续天数。
//     ///
//     /// 这些数据行使得策略的下行表现可以以表格形式直观展示。
//     fn row(&self) -> Row
//     {
//         row![format!("{:.3}", self.max_drawdown.drawdown.drawdown),
//              self.max_drawdown.drawdown.duration.num_days().to_string(),
//              format!("{:.3}", self.avg_drawdown.mean_drawdown),
//              self.avg_drawdown.mean_duration.num_days().to_string(),]
//     }
// }
//
// impl DrawdownSummary
// {
//     /// `new` 方法用于创建一个新的 `DrawdownSummary` 实例。
//     ///
//     /// # 参数
//     /// - `starting_equity`：初始权益金额，用于初始化当前回撤。
//     ///
//     /// # 返回值
//     /// 返回一个新的 `DrawdownSummary` 对象，包含初始的最大回撤、平均回撤和当前回撤值：
//     /// - `current_drawdown` 初始化为使用 `starting_equity` 的 `Drawdown` 对象，用于跟踪当前的回撤状态。
//     /// - `avg_drawdown` 初始化为空的 `AvgDrawdown` 对象，用于跟踪所有回撤的平均情况。
//     /// - `max_drawdown` 初始化为空的 `MaxDrawdown` 对象，用于记录历史上的最大回撤。
//     pub fn new(starting_equity: f64) -> Self
//     {
//         Self { current_drawdown: Drawdown::init(starting_equity),
//                avg_drawdown: AvgDrawdown::init(),
//                max_drawdown: MaxDrawdown::init() }
//     }
// }
