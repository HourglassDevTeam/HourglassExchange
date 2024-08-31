use crate::common::position::Position;
use crate::dashboard::metrics::EquitySnapshot;
use crate::dashboard::summary::PositionSummariser;
use crate::dashboard::{
    metrics::drawdown::{AvgDrawdown, Drawdown, MaxDrawdown},
    summary::TableBuilder,
};
/// `DrawdownSummary` 模块用于计算和跟踪投资组合或交易策略中的最大回撤、平均回撤以及它们的持续时间。
///
/// # 模块概述
/// `DrawdownSummary` 是一个综合工具，用于跟踪交易头寸的表现，特别关注下行风险。它主要计算三个关键指标：
/// - **当前回撤 (`current_drawdown`)**：正在发生的从峰值到当前价值的下降幅度。
/// - **最大回撤 (`max_drawdown`)**：历史上最大的一次从峰值到谷底的下降幅度。
/// - **平均回撤 (`avg_drawdown`)**：在给定时间段内，所有回撤的平均值和持续时间。
///
/// 这些指标对于评估投资策略的风险非常重要，尤其是在评估下行波动性时。较大的回撤意味着策略可能会经历剧烈的价值波动，而较长的回撤持续时间则意味着投资者可能需要更长时间才能恢复到峰值。
///
/// # 计算原理
/// 1. **EquitySnapshot 计算**：每当一个交易头寸结束时（即 `exit_balance` 已经确定），我们就会计算一个 `EquitySnapshot`，它表示在该时间点的总权益。这包括头寸的结束时间（这里暂时使用当前时间 `Utc::now()`）和总金额。
///
/// 2. **更新回撤 (`update`)**：当一个新的 `EquitySnapshot` 被生成时，我们使用它来更新当前回撤。如果当前回撤结束（即策略从谷底恢复到新的峰值），我们会更新最大回撤和平均回撤。
///    - **当前回撤**：跟踪正在进行中的回撤，并在它结束时记录其最终数值。
///    - **最大回撤**：记录历史上最深的回撤，这对于评估策略的最坏情况非常重要。
///    - **平均回撤**：计算一段时间内所有回撤的平均值和持续时间，这有助于理解策略的典型下行表现。
///
/// 3. **TableBuilder 实现**：为 `DrawdownSummary` 提供了一个表格生成功能，使这些关键指标可以清晰地展示出来。表格标题包含最大回撤、最大回撤天数、平均回撤和平均回撤天数。这些信息对于投资者或策略开发者来说非常重要。
///
/// # 时间戳的选择
/// `EquitySnapshot` 的时间戳字段 `time` 目前设置为 `Utc::now()`，即当前时间。这是为了简单起见，如果 `PositionMeta` 中包含了更合适的历史时间戳（如头寸关闭的时间），则应使用该时间戳，以更准确地反映回撤的发生时间。这种方法允许更精确的回撤分析，使得这些数据能够更好地反映策略的历史表现。
///
/// # 用途
/// 该模块特别适用于量化交易和投资组合管理中的风险评估。通过跟踪并分析回撤，投资者可以更好地理解他们的策略在不同市场条件下的表现，并做出相应的调整，以优化回报与风险的平衡。

use chrono::Utc;
use prettytable::{row, Row};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct DrawdownSummary
{
    pub current_drawdown: Drawdown,
    pub avg_drawdown: AvgDrawdown,
    pub max_drawdown: MaxDrawdown,
}


/// FIXME 这里的time到底输入历史时间戳还是实时时间戳？？？
impl PositionSummariser for DrawdownSummary {
    fn update(&mut self, position: &Position) {
        // 通过模式匹配获取不同头寸类型的 `exit_balance` 和时间戳，并构造 `EquitySnapshot`
        let equity_point = match position {
            // 如果是 Perpetual 类型头寸
            Position::Perpetual(pos) => EquitySnapshot {
                time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 LeveragedToken 类型头寸
            Position::LeveragedToken(pos) => EquitySnapshot {
                time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 Future 类型头寸
            Position::Future(pos) => EquitySnapshot {
                time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 Option 类型头寸
            Position::Option(pos) => EquitySnapshot {
                time: Utc::now(), // NOTE 暂时使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
        };

        // 更新 DrawdownSummary
        if let Some(ended_drawdown) = self.current_drawdown.update(equity_point) {
            self.avg_drawdown.update(&ended_drawdown);
            self.max_drawdown.update(&ended_drawdown);
        }
    }
}
impl TableBuilder for DrawdownSummary
{
    fn titles(&self) -> Row
    {
        row!["Max Drawdown", "Max Drawdown Days", "Avg. Drawdown", "Avg. Drawdown Days",]
    }

    fn row(&self) -> Row
    {
        row![format!("{:.3}", self.max_drawdown.drawdown.drawdown),
             self.max_drawdown.drawdown.duration.num_days().to_string(),
             format!("{:.3}", self.avg_drawdown.mean_drawdown),
             self.avg_drawdown.mean_duration.num_days().to_string(),]
    }
}

impl DrawdownSummary
{
    pub fn new(starting_equity: f64) -> Self
    {
        Self {
            current_drawdown: Drawdown::init(starting_equity),
            avg_drawdown: AvgDrawdown::init(),
            max_drawdown: MaxDrawdown::init(),
        }
    }
}
