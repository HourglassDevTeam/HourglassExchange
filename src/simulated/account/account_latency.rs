
use rand::Rng;
use rand_distr::{Distribution, Normal};

#[derive(Clone, Debug)]
pub struct AccountLatency {
    pub fluctuation_mode: FluctuationMode,
    pub maximum: i64,
    pub minimum: i64,
    pub current_value: i64,
}

pub fn fluctuate_latency(latency: &mut AccountLatency, current_time: i64) {
    match latency.fluctuation_mode {
        FluctuationMode::Sine => {
            latency.current_value = ((latency.maximum - latency.minimum) as f64 * ((current_time as f64).sin() + 1.0) / 2.0) as i64 + latency.minimum;
        }
        FluctuationMode::Cosine => {
            latency.current_value = ((latency.maximum - latency.minimum) as f64 * ((current_time as f64).cos() + 1.0) / 2.0) as i64 + latency.minimum;
        }
        FluctuationMode::NormalDistribution => {
            let normal = Normal::new((latency.maximum + latency.minimum) as f64 / 2.0, (latency.maximum - latency.minimum) as f64 / 6.0).unwrap();
            let value = normal.sample(&mut rand::thread_rng()) as i64;
            latency.current_value = value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::Uniform => {
            latency.current_value = rand::thread_rng().gen_range(latency.minimum..=latency.maximum);
        }
        FluctuationMode::Exponential => {
            let exp_value = (((current_time as f64).exp() % (latency.maximum - latency.minimum) as f64) + latency.minimum as f64) as i64;
            latency.current_value = exp_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::Logarithmic => {
            let log_value = (((current_time as f64).ln().abs() % (latency.maximum - latency.minimum) as f64) + latency.minimum as f64) as i64;
            latency.current_value = log_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::LinearIncrease => {
            let linear_value = latency.minimum + (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::LinearDecrease => {
            let linear_value = latency.maximum - (current_time % (latency.maximum - latency.minimum));
            latency.current_value = linear_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::StepFunction => {
            let step_size = (latency.maximum - latency.minimum) / 10;
            let step_value = latency.minimum + ((current_time / step_size) % 10) * step_size;
            latency.current_value = step_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::RandomWalk => {
            let step_size = ((latency.maximum - latency.minimum) / 20).max(1);
            let direction: i64 = if rand::random() { 1 } else { -1 };
            let new_value = latency.current_value + direction * step_size;
            latency.current_value = new_value.clamp(latency.minimum, latency.maximum);
        }
        FluctuationMode::None => {
            latency.current_value = latency.minimum;
        }
    }
}