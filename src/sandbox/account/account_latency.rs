use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
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
}
impl AccountLatency
{
    /// 创建一个新的 `AccountLatency` 实例。
    pub fn new(fluctuation_mode: FluctuationMode, maximum: i64, minimum: i64) -> Self
    {
        Self { fluctuation_mode,
               maximum,
               minimum,
               current_value: minimum }
    }
}

pub fn fluctuate_latency(latency: &mut AccountLatency, seed: i64)
{
    let range = (latency.maximum - latency.minimum) as f64;
    let dynamic_seed = seed + (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64 % 1000);

    match latency.fluctuation_mode {
        | FluctuationMode::Sine => {
            let adjusted_seed = ((dynamic_seed as f64 / 100.0).sin() + 1.0) / 2.0; // 0到1之间的值
            latency.current_value = (range * adjusted_seed) as i64 + latency.minimum;
        }
        | FluctuationMode::Cosine => {
            let adjusted_seed = ((dynamic_seed as f64 / 100.0).cos() + 1.0) / 2.0; // 0到1之间的值
            latency.current_value = (range * adjusted_seed) as i64 + latency.minimum;
        }
        | FluctuationMode::NormalDistribution => {
            let normal = Normal::new((latency.maximum + latency.minimum) as f64 / 2.0, range / 4.0).unwrap();
            let value = normal.sample(&mut rand::thread_rng()) as i64;
            latency.current_value = value.clamp(latency.minimum, latency.maximum);
        }
        | FluctuationMode::Uniform => {
            latency.current_value = rand::thread_rng().gen_range(latency.minimum..=latency.maximum);
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::order::identification::machine_id::generate_machine_id;

    #[test]
    fn test_fluctuate_latency_sine()
    {
        let machine_id = generate_machine_id().unwrap();
        let mut latency = AccountLatency::new(FluctuationMode::Sine, 100, 0);
        fluctuate_latency(&mut latency, machine_id as i64);
        println!("{:?}", latency);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_cosine()
    {
        let machine_id = generate_machine_id().unwrap();
        let mut latency = AccountLatency::new(FluctuationMode::Cosine, 100, 0);
        fluctuate_latency(&mut latency, machine_id as i64);
        println!("{:?}", latency);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_normal_distribution()
    {
        let machine_id = generate_machine_id().unwrap();
        let mut latency = AccountLatency::new(FluctuationMode::NormalDistribution, 100, 0);
        fluctuate_latency(&mut latency, machine_id as i64);
        println!("{:?}", latency);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }

    #[test]
    fn test_fluctuate_latency_uniform()
    {
        let machine_id = generate_machine_id().unwrap();
        let mut latency = AccountLatency::new(FluctuationMode::Uniform, 100, 0);
        fluctuate_latency(&mut latency, machine_id as i64);
        println!("{:?}", latency);
        assert!(latency.current_value >= latency.minimum && latency.current_value <= latency.maximum);
    }
}
