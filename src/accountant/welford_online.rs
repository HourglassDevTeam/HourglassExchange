/// Welford's 在线算法是一种用于逐次计算均值和方差的数值稳定方法，尤其适合于处理大数据集或流数据。
/// 该算法的主要优点在于它能够在单次遍历数据的过程中，以在线方式高效地计算均值和方差，且避免了在处理较大数据集时可能出现的数值不稳定问题。
///
/// ### 算法简介
///
/// Welford's 在线算法的基本思想是通过迭代更新均值和方差，避免存储整个数据集并进行多次遍历。
/// 对于每个新数据点，算法通过更新当前均值和方差的方式，实现了单次遍历即可获取准确结果。
///
/// 具体而言，算法维护以下三个变量：
///
/// - `mean`：当前数据的均值。
/// - `M2`：与方差相关的累积量，用于计算样本方差或总体方差。
/// - `n`：数据点的数量。
///
/// ### 计算步骤
///
/// 1. 对于每个新数据点 `x`，首先更新数据点的数量 `n`。
/// 2. 计算新的均值 `mean`：
///
///    ```text
///    mean_n = mean_(n-1) + (x - mean_(n-1)) / n
///    ```
///
/// 3. 计算新的累积量 `M2`：
///
///    ```text
///    M2_n = M2_(n-1) + (x - mean_(n-1)) * (x - mean_n)
///    ```
///
/// 4. 最后，可以通过 `M2` 计算样本方差或总体方差：
///
///    - 样本方差 (使用贝塞尔校正)：`variance = M2 / (n - 1)`
///    - 总体方差：`variance = M2 / n`
///
/// ### 数值稳定性
///
/// Welford's 在线算法的一个关键特性是其数值稳定性。传统的方差计算方法可能会因大数和小数之差的累积导致精度损失，
/// 而 Welford 的方法通过增量更新，避免了这种精度损失问题，特别适用于需要处理大量数据或精度要求较高的场景。
///
/// ### 适用场景
///
/// 该算法广泛应用于数据流处理、实时数据分析、大数据处理等领域，适用于不可能将所有数据保存在内存中的情况。
///
/// 更多详细信息请参考 [Welford's Online Algorithm](https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm)。

/// 计算下一个均值。
pub fn calculate_mean<T>(mut prev_mean: T, next_value: T, count: T) -> T
where
    T: Copy + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + std::ops::AddAssign,
{
    prev_mean += (next_value - prev_mean) / count;
    prev_mean
}

/// 计算下一个 Welford 在线递推关系 M。
pub fn calculate_recurrence_relation_m(
    prev_m: f64,
    prev_mean: f64,
    new_value: f64,
    new_mean: f64,
) -> f64 {
    prev_m + ((new_value - prev_mean) * (new_value - new_mean))
}

/// 使用贝塞尔校正（count - 1）和 Welford 在线递推关系 M 计算下一个无偏的“样本”方差。
pub fn calculate_sample_variance(recurrence_relation_m: f64, count: u64) -> f64 {
    match count < 2 {
        true => 0.0,
        false => recurrence_relation_m / (count as f64 - 1.0),
    }
}

/// 使用 Welford 在线递推关系 M 计算下一个有偏的“总体”方差。
pub fn calculate_population_variance(recurrence_relation_m: f64, count: u64) -> f64 {
    match count < 1 {
        true => 0.0,
        false => recurrence_relation_m / count as f64,
    }
}

