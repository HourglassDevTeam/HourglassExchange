use crate::dashboard::{dispersion::Dispersion, summary::TableBuilder, welford_online};
use prettytable::{row, Row};
use serde::{Deserialize, Serialize};

/// `DataSummary` 结构体用于汇总数据集的统计信息，包括计数、总和、均值以及离散度（方差、标准差和范围）。
///
/// # 原理介绍
/// 数据汇总是一种常见的统计操作，它可以帮助我们从一组数据中提取出一些关键的统计信息，如数据的数量、总和、平均值，以及描述数据分散程度的方差、标准差和范围。
///
/// 该结构体结合了 Welford 在线算法和自定义的离散度（Dispersion）结构体，能够高效地在数据逐一输入的情况下动态更新这些统计量。
///
/// - **计数（Count）**：记录数据集中元素的数量。
/// - **总和（Sum）**：所有数据点的累加和。
/// - **均值（Mean）**：数据点的平均值。
/// - **离散度（Dispersion）**：描述数据分散程度的度量，包括方差、标准差和范围。
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Deserialize, Serialize)]
pub struct DataSummary
{
    /// 数据点的数量
    pub count: u64,
    /// 数据点的总和
    pub sum: f64,
    /// 数据点的均值
    pub mean: f64,
    /// 数据点的离散度（方差、标准差和范围）
    pub dispersion: Dispersion,
}

impl DataSummary
{
    /// 更新数据汇总结构体，给定一个新的数据点。
    ///
    /// # 原理解释
    /// 当一个新的数据点被添加时，我们需要更新结构体中的计数、总和、均值以及离散度。
    ///
    /// 具体步骤如下：
    /// 1. **计数增加**：每次添加新数据点，计数器都会增加。
    /// 2. **更新总和**：将新数据点的值加入总和。
    /// 3. **更新均值**：使用 Welford 在线算法高效地更新均值。
    /// 4. **更新离散度**：基于新的均值和数据点，动态更新离散度（方差、标准差和范围）。
    ///
    /// # 参数
    /// - `next_value`: 新的数据点的值
    pub fn update(&mut self, next_value: f64)
    {
        // 增加计数器
        self.count += 1;
        // 更新总和
        self.sum += next_value;
        // 更新均值
        let prev_mean = self.mean;
        self.mean = welford_online::update_mean(self.mean, next_value, self.count as f64);
        // 更新离散度
        self.dispersion.update(prev_mean, self.mean, next_value, self.count);
    }
}

/// 实现 `TableBuilder` 特性，用于生成表格表示形式的 `DataSummary`。
///
/// # 表格生成
/// 该实现允许将 `DataSummary` 对象的统计信息以表格的形式展示出来，
/// 包括计数、总和、均值、方差、标准差，以及范围的最高值和最低值。
impl TableBuilder for DataSummary
{
    /// 返回表格的标题行。
    ///
    /// # 返回
    /// 返回包含统计量名称的 `Row` 对象。
    fn titles(&self) -> Row
    {
        row!["Count", "Sum", "Mean", "Variance", "Std. Dev", "Range High", "Range Low",]
    }

    /// 返回表格的一行数据。
    ///
    /// # 返回
    /// 返回包含统计量值的 `Row` 对象。
    fn row(&self) -> Row
    {
        row![self.count,
             format!("{:.3}", self.sum),
             format!("{:.3}", self.mean),
             format!("{:.3}", self.dispersion.variance),
             format!("{:.3}", self.dispersion.std_dev),
             format!("{:.3}", self.dispersion.range.high),
             format!("{:.3}", self.dispersion.range.low),]
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::dashboard::dispersion::Range;

    #[test]
    fn update_data_summary_with_position()
    {
        struct TestCase
        {
            input_next_value: f64,
            expected_summary: DataSummary,
        }

        let mut data_summary = DataSummary::default();

        let test_cases = vec![TestCase { // Test case 0
            input_next_value: 1.1,
            expected_summary: DataSummary {
                count: 1,
                sum: 1.1,
                mean: 1.1,
                dispersion: Dispersion {
                    range: Range {
                        activated: true,
                        high: 1.1,
                        low: 1.1
                    },
                    recurrence_relation_m: 0.00,
                    variance: 0.0,
                    std_dev: 0.0
                }
            }
        },
                              TestCase { // Test case 1
                                  input_next_value: 1.2,
                                  expected_summary: DataSummary {
                                      count: 2,
                                      sum: 2.3,
                                      mean: (2.3 / 2.0),
                                      dispersion: Dispersion {
                                          range: Range {
                                              activated: true,
                                              high: 1.2,
                                              low: 1.1
                                          },
                                          recurrence_relation_m: 0.005,
                                          variance: 0.0025,
                                          std_dev: 0.05
                                      }
                                  }
                              },
                              TestCase { // Test case 2
                                  input_next_value: 1.3,
                                  expected_summary: DataSummary {
                                      count: 3,
                                      sum: (2.3 + 1.3),
                                      mean: (3.6 / 3.0),
                                      dispersion: Dispersion {
                                          range: Range {
                                              activated: true,
                                              high: 1.3,
                                              low: 1.1
                                          },
                                          recurrence_relation_m: 0.02,
                                          variance: 1.0 / 150.0,
                                          std_dev: (6.0_f64.sqrt() / 30.0)
                                      }
                                  }
                              }, ];

        for (index, test) in test_cases.into_iter().enumerate() {
            data_summary.update(test.input_next_value);
            assert_eq!(data_summary.count, test.expected_summary.count, "Count Input: {:?}", index);
            assert_eq!(data_summary.sum, test.expected_summary.sum, "Sum Input: {:?}", index);
            assert_eq!(data_summary.mean, test.expected_summary.mean, "Mean Input: {:?}", index);

            let recurrence_diff = data_summary.dispersion.recurrence_relation_m - test.expected_summary.dispersion.recurrence_relation_m;
            assert!(recurrence_diff < 1e-10, "Recurrence Input: {:?}", index);

            let variance_diff = data_summary.dispersion.variance - test.expected_summary.dispersion.variance;
            assert!(variance_diff < 1e-10, "Variance Input: {:?}", index);

            let std_dev_diff = data_summary.dispersion.std_dev - test.expected_summary.dispersion.std_dev;
            assert!(std_dev_diff < 1e-10, "Std. Dev. Input: {:?}", index);
        }
    }
}
