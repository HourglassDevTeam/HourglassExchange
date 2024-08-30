use chrono::Utc;
use crate::dashboard::{
    metrics::drawdown::{AvgDrawdown, Drawdown, MaxDrawdown},
    summary::TableBuilder,
};
use prettytable::{row, Row};
use serde::{Deserialize, Serialize};
use crate::common::position::Position;
use crate::dashboard::metrics::EquityPoint;
use crate::dashboard::summary::PositionSummariser;

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
        // 通过模式匹配获取不同头寸类型的 `exit_balance` 和时间戳，并构造 `EquityPoint`
        let equity_point = match position {
            // 如果是 Perpetual 类型头寸
            Position::Perpetual(pos) => EquityPoint {
                time: Utc::now(), // 使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 LeveragedToken 类型头寸
            Position::LeveragedToken(pos) => EquityPoint {
                time: Utc::now(), // 使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 Future 类型头寸
            Position::Future(pos) => EquityPoint {
                time: Utc::now(), // 使用当前时间或从 `pos.meta` 获取时间戳
                total: pos.meta.exit_balance.balance.total,
            },
            // 如果是 Option 类型头寸
            Position::Option(pos) => EquityPoint {
                time: Utc::now(), // 使用当前时间或从 `pos.meta` 获取时间戳
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
        Self { current_drawdown: Drawdown::init(starting_equity),
               avg_drawdown: AvgDrawdown::init(),
               max_drawdown: MaxDrawdown::init() }
    }
}
