use rand::Rng; // 引入随机数生成器
use rand_distr::{Distribution, Normal}; // 引入随机分布库，包括常态分布

#[derive(Clone, Debug)] // 派生Clone和Debug特性
pub struct AccountLatency {
    pub fluctuation_mode: FluctuationMode, // 波动模式
    pub maximum: i64, // 最大延迟值
    pub minimum: i64, // 最小延迟值
    pub current_value: i64, // 当前延迟值
}

pub fn fluctuate_latency(latency: &mut AccountLatency, current_time: i64) {
    match latency.fluctuation_mode {
        FluctuationMode::Sine => {
            // 使用正弦函数波动
            latency.current_value = ((latency.maximum - latency.minimum) as f64 * ((current_time as f64).sin() + 1.0) / 2.0) as i64 + latency.minimum;
        }
        FluctuationMode::Cosine => {
            // 使用余弦函数波动
            latency.current_value = ((latency.maximum - latency.minimum) as f64 * ((current_time as f64).cos() + 1.0) / 2.0) as i64 + latency.minimum;
        }
        FluctuationMode::NormalDistribution => {
            // 使用正态分布波动
            let normal = Normal::new((latency.maximum + latency.minimum) as f64 / 2.0, (latency.maximum - latency.minimum) as f64 / 6.0).unwrap();
            let value = normal.sample(&mut rand::thread_rng()) as i64;
            latency.current_value = value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::Uniform => {
            // 使用均匀分布波动
            latency.current_value = rand::thread_rng().gen_range(latency.minimum..=latency.maximum);
        }
        FluctuationMode::Exponential => {
            // 使用指数函数波动
            let exp_value = (((current_time as f64).exp() % (latency.maximum - latency.minimum) as f64) + latency.minimum as f64) as i64;
            latency.current_value = exp_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::Logarithmic => {
            // 使用对数函数波动
            let log_value = (((current_time as f64).ln().abs() % (latency.maximum - latency.minimum) as f64) + latency.minimum as f64) as i64;
            latency.current_value = log_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::LinearIncrease => {
            // 线性增加
            let linear_value = latency.minimum + (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::LinearDecrease => {
            // 线性减少
            let linear_value = latency.maximum - (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::StepFunction => {
            // 使用阶跃函数波动
            let step_size = (latency.maximum - latency.minimum) / 10;
            let step_value = latency.minimum + ((current_time / step_size) % 10) * step_size;
            latency.current_value = step_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::RandomWalk => {
            // 使用随机游走波动
            let step_size = ((latency.maximum - latency.minimum) / 20).max(1);
            let direction: i64 = if rand::random() { 1 } else { -1 };
            let new_value = latency.current_value + direction * step_size;
            latency.current_value = new_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::None => {
            // 无波动
            latency.current_value = latency.minimum;
        }
    }
}
