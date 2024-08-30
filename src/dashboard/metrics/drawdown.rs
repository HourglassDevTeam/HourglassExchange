use crate::dashboard::{de_duration_from_secs, dispersion::Range, metrics::EquitySnapshot, se_duration_as_secs, welford_online};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// [`Drawdown`] 表示投资组合或投资在特定时期内从峰值到谷底的下降幅度。它是衡量下行波动性的一种方式。
///
/// # 背景
/// Drawdown 是投资中常用的概念，表示资产从峰值下降到谷底的幅度。较大的 Drawdown 表示投资有较大的下行风险。
///
/// 参考文档: <https://www.investopedia.com/terms/d/drawdown.asp>
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Drawdown
{
    /// 记录投资组合的最高和最低权益（equity）
    pub equity_range: Range,
    /// 当前的 Drawdown 数值，表示从峰值下降的百分比
    pub drawdown: f64,
    /// Drawdown 开始的时间点
    pub start_time: DateTime<Utc>,
    /// Drawdown 持续的时间，用 `Duration` 表示
    #[serde(deserialize_with = "de_duration_from_secs", serialize_with = "se_duration_as_secs")]
    pub duration: Duration,
}

impl Default for Drawdown
{
    /// 创建一个默认的 [`Drawdown`] 实例，所有字段初始化为默认值。
    fn default() -> Self
    {
        Self { equity_range: Default::default(),
               drawdown: 0.0,
               start_time: Utc::now(),
               duration: Duration::zero() }
    }
}

impl Drawdown
{
    /// 使用初始的权益值（equity）作为第一个峰值来初始化 [`Drawdown`]。
    ///
    /// # 参数
    /// - `starting_equity`: 初始的权益值
    ///
    /// # 返回
    /// 返回一个初始化的 `Drawdown` 实例。
    pub fn init(starting_equity: f64) -> Self
    {
        Self { equity_range: Range { activated: true,
                                     high: starting_equity,
                                     low: starting_equity },
               drawdown: 0.0,
               start_time: Utc::now(),
               duration: Duration::zero() }
    }

    /// 使用最新的权益点 [`EquitySnapshot`] 更新 [`Drawdown`]。如果 Drawdown 周期结束（投资从谷底恢复到高于之前的峰值），
    /// 则函数返回 `Some(Drawdown)`，否则返回 `None`。
    ///
    /// # 参数
    /// - `current`: 当前的权益点
    ///
    /// # 返回
    /// 如果 Drawdown 周期结束，返回 `Some(Drawdown)`；否则返回 `None`。
    pub fn update(&mut self, current: EquitySnapshot) -> Option<Drawdown>
    {
        match (self.is_waiting_for_peak(), current.total > self.equity_range.high) {
            // A) 当前没有 drawdown，等待下一个权益峰值（等待 B 的出现）
            | (true, true) => {
                self.equity_range.high = current.total;
                None
            }

            // B) 新的 drawdown 开始，上一个权益点设为峰值，当前权益更低
            | (true, false) => {
                self.start_time = current.time;
                self.equity_range.low = current.total;
                self.drawdown = self.calculate();
                None
            }

            // C) drawdown 持续进行中，权益低于最近的峰值
            | (false, false) => {
                self.duration = current.time.signed_duration_since(self.start_time);
                self.equity_range.update(current.total);
                self.drawdown = self.calculate(); // 如果不想，可以不立即计算
                None
            }

            // D) drawdown 结束，权益达到新峰值（进入 A）
            | (false, true) => {
                // 克隆上一个迭代的 Drawdown 以返回
                let finished_drawdown = Drawdown { equity_range: self.equity_range,
                                                   drawdown: self.drawdown,
                                                   start_time: self.start_time,
                                                   duration: self.duration };

                // 清理 - 在下一个 drawdown 开始时重写 start_time
                self.drawdown = 0.0; // 即等待新的峰值
                self.duration = Duration::zero();

                // 设置新的权益峰值，为下一次迭代做准备
                self.equity_range.high = current.total;

                Some(finished_drawdown)
            }
        }
    }

    /// 判断是否在等待下一个权益峰值。如果新的 [`EquitySnapshot`] 高于之前的峰值，则为 `true`。
    ///
    /// # 返回
    /// 返回 `true` 如果在等待峰值；否则返回 `false`。
    pub fn is_waiting_for_peak(&self) -> bool
    {
        self.drawdown == 0.0
    }

    /// 计算特定时期内的 [`Drawdown`] 值。公式为：[`Drawdown`] = (range_low - range_high) / range_high
    ///
    /// # 返回
    /// 返回计算得到的 drawdown 值。
    pub fn calculate(&self) -> f64
    {
        (-self.equity_range.calculate_range()) / self.equity_range.high
    }
}

/// [`MaxDrawdown`] 是投资组合或投资的最大峰值到谷底的下降幅度。它是衡量下行风险的一种方式，
/// 较大的 Max Drawdown 表示可能会有较大的波动。
///
/// 参考文档: <https://www.investopedia.com/terms/m/maximum-drawdown-mdd.asp>
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MaxDrawdown
{
    /// 当前的最大 Drawdown
    pub drawdown: Drawdown,
}

impl MaxDrawdown
{
    /// 使用 [`Drawdown`] 的默认值来初始化一个新的 [`MaxDrawdown`] 实例。
    ///
    /// # 返回
    /// 返回一个初始化的 `MaxDrawdown` 实例。
    pub fn init() -> Self
    {
        Self { drawdown: Drawdown::default() }
    }

    /// 使用最新的 [`Drawdown`] 更新 [`MaxDrawdown`]。如果输入的 drawdown 大于当前的 [`MaxDrawdown`]，
    /// 则使用新的 drawdown 替换它。
    ///
    /// # 参数
    /// - `next_drawdown`: 最新的 drawdown 值
    pub fn update(&mut self, next_drawdown: &Drawdown)
    {
        if next_drawdown.drawdown.abs() > self.drawdown.drawdown.abs() {
            self.drawdown = *next_drawdown;
        }
    }
}

/// [`AvgDrawdown`] 包含在特定时期内从一组 [`Drawdown`] 中计算的平均 drawdown 值和持续时间。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct AvgDrawdown
{
    /// 记录的 drawdown 数量
    pub count: u64,
    /// 平均 drawdown 值
    pub mean_drawdown: f64,
    /// 平均持续时间，用 `Duration` 表示
    #[serde(deserialize_with = "de_duration_from_secs", serialize_with = "se_duration_as_secs")]
    pub mean_duration: Duration,
    /// 平均持续时间，以毫秒为单位
    pub mean_duration_milliseconds: i64,
}

impl Default for AvgDrawdown
{
    /// 创建一个默认的 [`AvgDrawdown`] 实例，所有字段初始化为默认值。
    fn default() -> Self
    {
        Self { count: 0,
               mean_drawdown: 0.0,
               mean_duration_milliseconds: 0,
               mean_duration: Duration::zero() }
    }
}

impl AvgDrawdown
{
    /// 使用默认方法初始化一个新的 [`AvgDrawdown`]，为所有字段提供零值。
    ///
    /// # 返回
    /// 返回一个初始化的 `AvgDrawdown` 实例。
    pub fn init() -> Self
    {
        Self::default()
    }

    /// 使用最新的 [`Drawdown`] 更新 [`AvgDrawdown`]。
    ///
    /// # 参数
    /// - `drawdown`: 最新的 drawdown 值
    pub fn update(&mut self, drawdown: &Drawdown)
    {
        self.count += 1;

        self.mean_drawdown = welford_online::update_mean(self.mean_drawdown, drawdown.drawdown, self.count as f64);

        self.mean_duration_milliseconds = welford_online::update_mean(self.mean_duration_milliseconds, drawdown.duration.num_milliseconds(), self.count as i64);

        self.mean_duration = Duration::milliseconds(self.mean_duration_milliseconds);
    }
}
