use thiserror::Error;

#[derive(Error, Copy, Clone, Debug)]
pub enum StatisticsError {
    #[error("Failed to build struct due to missing attributes: {0}")]
    BuilderIncomplete(&'static str),

    #[error("Failed to build struct due to insufficient metrics provided")]
    BuilderNoMetricsProvided,
}
