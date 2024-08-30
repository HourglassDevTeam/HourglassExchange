use serde::{Deserialize, Serialize};
use crate::statistics::welford_online;

/// 表示一组数据集的离散程度的度量 - 范围、方差和标准差。
///
/// # 背景知识
/// 离散程度是描述数据如何分散或集中的一种方式。我们使用范围（Range）、方差（Variance）和标准差（Standard Deviation）来衡量这一点。
///
/// - **范围（Range）**：是数据集中的最大值与最小值之间的差值，反映了数据的总体分布情况。
/// - **方差（Variance）**：是数据集中的数据点与其平均值之间的平方差的平均值，用于衡量数据的波动程度。
/// - **标准差（Standard Deviation）**：是方差的平方根，更直观地表示数据的波动性。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct Dispersion {
    /// 数据集的范围（即最高值和最低值之间的差距）
    pub range: Range,
    /// Welford 在线算法的递推关系 M，用于计算方差和标准差
    pub recurrence_relation_m: f64,
    /// 数据集的方差
    pub variance: f64,
    /// 数据集的标准差
    pub std_dev: f64,
}

impl Dispersion {
    /// 迭代更新 Dispersion 的度量，包括范围、方差和标准差。
    ///
    /// # 原理解释
    /// 当你向数据集中添加新数据时，我们需要动态更新这些统计量。
    /// 这个方法通过提供前一个均值、新的均值、新的值和数据集中的值的数量来实现。
    ///
    /// 具体步骤如下：
    /// 1. **更新范围（Range）**：检查新值是否比当前的最大值更大，或者比最小值更小，如果是的话，更新相应的最大值或最小值。
    /// 2. **更新递推关系 M**：使用 Welford 在线算法的递推公式计算新的 M 值，这个 M 是计算方差的中间值。
    /// 3. **更新方差（Variance）**：利用 M 值和数据的总数量，重新计算方差。
    /// 4. **更新标准差（Standard Deviation）**：方差的平方根即为标准差，表示数据的波动程度。
    ///
    /// # 应用
    /// 在现实世界中，比如说你想要分析某个班级的考试成绩分布情况，你可以用这个方法来动态计算这些统计量，随时了解成绩的变化情况。
    ///
    /// # 参数
    /// - `prev_mean`: 前一个均值
    /// - `new_mean`: 新的均值
    /// - `new_value`: 新的值
    /// - `value_count`: 数据集中的值的数量
    pub fn update(&mut self, prev_mean: f64, new_mean: f64, new_value: f64, value_count: u64) {
        // 更新范围
        self.range.update(new_value);

        // 更新 Welford 在线算法的递推关系 M
        self.recurrence_relation_m = welford_online::update_variance_accumulator(
            self.recurrence_relation_m,
            prev_mean,
            new_value,
            new_mean,
        );

        // 更新总体方差
        self.variance =
            welford_online::compute_population_variance(self.recurrence_relation_m, value_count);

        // 更新标准差
        self.std_dev = self.variance.sqrt();
    }
}

/// 表示数据集的离散程度的度量，提供数据集中的最高值和最低值。使用惰性计算来计算它们之间的范围。
///
/// # 原理解释
/// 范围（Range）是数据集中最大值和最小值之间的差距，它告诉我们数据的分布情况有多广。
///
/// 例如：如果你在记录一个月的每日最高温度，你可能想知道这个月的温度变化范围有多大。这时候，范围就派上了用场。
///
/// # 应用
/// 在统计分析中，范围可以帮助你快速了解数据的总体分布情况，例如判断一个班级的考试成绩差距有多大。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct Range {
    /// 指示范围是否已初始化
    pub activated: bool,
    /// 数据集中的最高值
    pub high: f64,
    /// 数据集中的最低值
    pub low: f64,
}

impl Range {
    /// 使用提供的数据集的第一个值初始化范围。
    ///
    /// # 原理解释
    /// 当我们开始分析一个新的数据集时，通常会从第一个数据点开始初始化范围。这个初始值既是当前的最大值，也是当前的最小值。
    ///
    /// # 参数
    /// - `first_value`: 数据集中的第一个值
    ///
    /// # 返回
    /// 返回初始化后的 `Range` 结构体。
    pub fn init(first_value: f64) -> Self {
        Self {
            activated: false,
            high: first_value,
            low: first_value,
        }
    }

    /// 给定数据集中的下一个值，迭代更新范围。
    ///
    /// # 原理解释
    /// 当你加入新的数据点时，你需要检查这个新数据是否比当前的最高值更高，或者比当前的最低值更低。如果是，那么更新相应的最高值或最低值。
    ///
    /// # 参数
    /// - `new_value`: 数据集中的新值
    pub fn update(&mut self, new_value: f64) {
        match self.activated {
            true => {
                if new_value > self.high {
                    self.high = new_value;
                }

                if new_value < self.low {
                    self.low = new_value;
                }
            }
            false => {
                self.activated = true;
                self.high = new_value;
                self.low = new_value;
            }
        }
    }

    /// 计算数据集的最高值和最低值之间的范围。提供惰性计算功能。
    ///
    /// # 原理解释
    /// 有时候你可能不会立即需要范围的值，因此可以在需要时调用此方法来计算范围。
    ///
    /// # 返回
    /// 返回范围值，即 `high - low` 的结果。
    pub fn calculate_hl_diff(&self) -> f64 {
        self.high - self.low
    }
}
