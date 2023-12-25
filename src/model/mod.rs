use self::{
    balance::SymbolBalance,
    order::{Cancelled, Open, Order},
    trade::Trade,
};
use cerebro_integration::model::Exchange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use uuid::Uuid;


pub mod balance;      // 定义balance模块
pub mod order;        // 定义订单模块
pub mod trade;        // 定义交易模块

/// Normalised Barter [`AccountEvent`] containing metadata about the included
/// [`AccountEventKind`] variant. Produced by [`ExecutionClients`](crate::ExecutionClient).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountEvent {
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub kind: AccountEventKind,
}

/// Defines the type of Barter [`AccountEvent`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AccountEventKind {
    // HTTP Only
    OrdersOpen(Vec<Order<Open>>),
    OrdersNew(Vec<Order<Open>>),
    OrdersCancelled(Vec<Order<Cancelled>>),

    // WebSocket Only
    Balance(SymbolBalance),
    Trade(Trade),

    // HTTP & WebSocket
    Balances(Vec<SymbolBalance>),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Uuid);

impl std::fmt::Display for ClientOrderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ClientStatus {
    Connected,
    CancelOnly,
    Disconnected,
}
