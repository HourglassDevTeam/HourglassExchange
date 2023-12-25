use crate::model::{order::OrderKind, ClientOrderId};
use cerebro_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, PartialEq, Eq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExecutionError {
    #[error("[CerebroBroker] : 【cerebro_brokerfailed to build component due to missing attributes: {0}")]
    BuilderIncomplete(String),

    #[error("[CerebroBroker] : SimulatedExchange error: {0}")]
    Simulated(String),

    #[error("[CerebroBroker] : Balance for symbol {0} insufficient to open order")]
    InsufficientBalance(Symbol),

    #[error("[CerebroBroker] : failed to find Order with ClientOrderId: {0}")]
    OrderNotFound(ClientOrderId),

    #[error("[CerebroBroker] : failed to open Order due to unsupported OrderKind: {0}")]
    UnsupportedOrderKind(OrderKind),
}
