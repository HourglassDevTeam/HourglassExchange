use rand::Rng;
// 引入随机数生成器
use rand_distr::{Distribution, Normal};

// 引入随机分布库，包括常态分布

#[derive(Clone, Debug)] // 派生Clone和Debug特性
pub struct AccountLatency
{
    pub fluctuation_mode: FluctuationMode,
    pub maximum: i64,
    pub minimum: i64,
    pub current_value: i64,
}

#[derive(Clone, Debug)]
pub enum FluctuationMode
{
    Sine,
    Cosine,
    NormalDistribution,
    Uniform,
    Exponential,
    Logarithmic,
    LinearIncrease,
    LinearDecrease,
    StepFunction,
    RandomWalk,
    None,
}
impl AccountLatency
{
    /// 创建一个新的 `AccountLatency` 实例。
    pub fn new(fluctuation_mode: FluctuationMode, maximum: i64, minimum: i64) -> Self
    {
        Self {
            fluctuation_mode,
            maximum,
            minimum,
            current_value: minimum, /* 初始值设为最小值 */
        }
    }
}

pub fn fluctuate_latency(latency: &mut AccountLatency, current_time: i64)
{
    let range = (latency.maximum - latency.minimum) as f64;
    let half_range = range / 2.0;
    match latency.fluctuation_mode {
        | FluctuationMode::Sine => {
            latency.current_value = (half_range * ((current_time as f64).sin() + 1.0)) as i64 + latency.minimum;
        }
        | FluctuationMode::Cosine => {
            latency.current_value = (half_range * ((current_time as f64).cos() + 1.0)) as i64 + latency.minimum;
        }
        | FluctuationMode::NormalDistribution => {
            // 使用正态分布波动
            let normal = Normal::new((latency.maximum + latency.minimum) as f64 / 2.0, (latency.maximum - latency.minimum) as f64 / 6.0).unwrap();
            let value = normal.sample(&mut rand::thread_rng()) as i64;
            latency.current_value = value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::Uniform => {
            // 使用均匀分布波动
            latency.current_value = rand::thread_rng().gen_range(latency.minimum..=latency.maximum);
        }
        | FluctuationMode::Exponential => {
            // 使用指数函数波动
            let exp_value = (((current_time as f64).exp() % range) + latency.minimum as f64) as i64;
            latency.current_value = exp_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::Logarithmic => {
            // 使用对数函数波动
            let log_value = (((current_time as f64).ln().abs() % (latency.maximum - latency.minimum) as f64) + latency.minimum as f64) as i64;
            latency.current_value = log_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::LinearIncrease => {
            // 线性增加
            let linear_value = latency.minimum + (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::LinearDecrease => {
            // 线性减少
            let linear_value = latency.maximum - (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::StepFunction => {
            // 使用阶跃函数波动
            let step_size = (latency.maximum - latency.minimum) / 10;
            let step_value = latency.minimum + ((current_time / step_size) % 10) * step_size;
            latency.current_value = step_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::RandomWalk => {
            // 使用随机游走波动
            let step_size = ((latency.maximum - latency.minimum) / 20).max(1);
            let direction: i64 = if rand::random() { 1 } else { -1 };
            let new_value = latency.current_value + direction * step_size;
            latency.current_value = new_value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::None => {
            // 无波动
            latency.current_value = latency.minimum;
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_fluctuate_latency_sine()
    {
        let mut latency = AccountLatency::new(FluctuationMode::Sine, 100, 0);
        fluctuate_latency(&mut latency, 0);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_cosine()
    {
        let mut latency = AccountLatency::new(FluctuationMode::Cosine, 100, 0);
        fluctuate_latency(&mut latency, 0);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_normal_distribution()
    {
        let mut latency = AccountLatency::new(FluctuationMode::NormalDistribution, 100, 0);
        fluctuate_latency(&mut latency, 0);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_uniform()
    {
        let mut latency = AccountLatency::new(FluctuationMode::Uniform, 100, 0);
        fluctuate_latency(&mut latency, 0);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_exponential()
    {
        let mut latency = AccountLatency::new(FluctuationMode::Exponential, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_logarithmic()
    {
        let mut latency = AccountLatency::new(FluctuationMode::Logarithmic, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_linear_increase()
    {
        let mut latency = AccountLatency::new(FluctuationMode::LinearIncrease, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_linear_decrease()
    {
        let mut latency = AccountLatency::new(FluctuationMode::LinearDecrease, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_step_function()
    {
        let mut latency = AccountLatency::new(FluctuationMode::StepFunction, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_random_walk()
    {
        let mut latency = AccountLatency::new(FluctuationMode::RandomWalk, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_none()
    {
        let mut latency = AccountLatency::new(FluctuationMode::None, 100, 0);
        fluctuate_latency(&mut latency, 1);
        assert_eq!(latency.current_value, latency.minimum);
    }
}
