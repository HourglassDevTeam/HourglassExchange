// CerebroBroker
//
// 高性能和标准化的交易接口，能够在多个金融场所执行交易。同时提供一个功能丰富的模拟交易所，以协助进行回测和干式交易。通过初始化其关联的ExecutionClient实例与交易所进行通信。
// 特性
// 简单易用：ExecutionClient trait 提供了一个统一且简单的语言，用于与交易所进行交互。
// 标准化：允许您的策略使用相同的接口与每个真实或模拟的交易所进行通信。
// 可扩展：CerebroBroker具有高度可扩展性，使其易于通过添加新的交易所集成。

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]
#![allow(clippy::type_complexity)]



use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use cerebro_integration::model::Exchange;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    error::ExecutionError,
    model::{
        AccountEvent,
        balance::SymbolBalance,
        order::{Cancelled, Open, Order, OrderId, RequestCancel, RequestOpen},
    },
};

/// Errors generated during live, dry, or simulated execution.
pub mod error;

/// Core data structures to support executing on exchanges.
///
/// eg/ `Order`, `Balance`, `Trade` etc.
pub mod model;

/// [`ExecutionClient`] implementations for official exchanges.
pub mod execution;

/// Simulated Exchange and it's associated simulated [`ExecutionClient`].
pub mod simulated;

/// Defines the communication with the exchange. Each exchange integration requires it's own
/// implementation.
#[async_trait]
pub trait ExecutionClient {
    const CLIENT: ExecutionId;
    type Config;

    /// Initialise a new [`ExecutionClient`] with the provided [`Self::Config`] and
    /// [`AccountEvent`] transmitter.
    ///
    /// **Note:**
    /// Usually entails spawning an asynchronous WebSocket event loop to consume [`AccountEvent`]s
    /// from the exchange, as well as returning the HTTP client `Self`.
    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>) -> Self;

    /// Fetch account [`Order<Open>`]s.
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>;

    /// Fetch account [`SymbolBalance`]s.
    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError>;

    /// Open orders.
    async fn open_orders(
        &self,
        open_requests: Vec<Order<RequestOpen>>,
    ) -> Vec<Result<Order<Open>, ExecutionError>>;

    /// Cancel [`Order<Open>`]s.
    async fn cancel_orders(
        &self,
        cancel_requests: Vec<Order<RequestCancel>>,
    ) -> Vec<Result<Order<Cancelled>, ExecutionError>>;

    /// Cancel all account [`Order<Open>`]s.
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>;
}

/// Unique identifier for an [`ExecutionClient`] implementation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "execution", rename_all = "snake_case")]
pub enum ExecutionId {
    Simulated,
    Ftx,
}

impl From<ExecutionId> for Exchange {
    fn from(execution_id: ExecutionId) -> Self {
        Exchange::from(execution_id.as_str())
    }
}

impl Display for ExecutionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExecutionId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionId::Simulated => "simulated",
            ExecutionId::Ftx => "ftx",
        }
    }
}

/// Utilities for generating common data structures required for testing.
pub mod test_util {
    use cerebro_data::subscription::trade::PublicTrade;
    use cerebro_integration::model::{
        Exchange,
        instrument::{Instrument, kind::InstrumentKind}, Side,
    };

    use crate::{
        model::{
            ClientOrderId,
            trade::{SymbolFees, Trade, TradeId},
        },
        Open,
        Order, OrderId, simulated::exchange::account::order::Orders,
    };

    pub fn client_orders(
        trade_number: u64,
        bids: Vec<Order<Open>>,
        asks: Vec<Order<Open>>,
    ) -> Orders {
        Orders {
            trade_counter: trade_number,
            bids,
            asks,
        }
    }

    pub fn order_open(
        cid: ClientOrderId,
        side: Side,
        price: f64,
        quantity: f64,
        filled: f64,
    ) -> Order<Open> {
        Order {
            exchange: Exchange::from("exchange"),
            instrument: Instrument::from(("base", "quote", InstrumentKind::Perpetual)),
            cid,
            side,
            state: Open {
                id: OrderId::from("order_id"),
                price,
                quantity,
                filled_quantity: filled,
            },
        }
    }

    pub fn public_trade(side: Side, price: f64, amount: f64) -> PublicTrade {
        PublicTrade {
            id: "trade_id".to_string(),
            price,
            amount,
            side,
        }
    }

    pub fn trade(id: TradeId, side: Side, price: f64, quantity: f64, fees: SymbolFees) -> Trade {
        Trade {
            id,
            order_id: OrderId::from("order_id"),
            instrument: Instrument::from(("base", "quote", InstrumentKind::Perpetual)),
            side,
            price,
            quantity,
            fees,
        }
    }
}
