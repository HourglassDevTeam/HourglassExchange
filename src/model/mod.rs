use std::fmt::Formatter;

use cerebro_integration::model::Exchange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use self::{
    balance::SymbolBalance,
    order::{Cancelled, Open, Order},
    trade::Trade,
};

pub mod balance; // 定义balance模块
pub mod order; // 定义订单模块
pub mod trade; // 定义交易模块


/// 标准化的[`AccountEvent`]，包含了相关[`AccountEventKind`]变体的元数据。由[`ExecutionClients`](crate::ExecutionClient)生成。
/// NOTE: 如果需要记录交易所的时间戳，可以在 AccountEvent 结构中添加一个专门的字段来表示交易所的时间，例如：    pub exchange: Exchange
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountEvent {
    pub received_time: DateTime<Utc>, // 客户端接收到事件的时间
    pub exchange: Exchange,           // 交易所
    pub kind: AccountEventKind,       // 事件类型
}

/// 定义账户事件[`AccountEvent`]的类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AccountEventKind {
    // HTTP Only - 仅限HTTP
    OrdersOpen(Vec<Order<Open>>),
    OrdersNew(Vec<Order<Open>>),
    OrdersCancelled(Vec<Order<Cancelled>>),

    // WebSocket Only - 仅限WebSocket
    Balance(SymbolBalance),
    Trade(Trade),

    // HTTP & WebSocket - HTTP和WebSocket
    Balances(Vec<SymbolBalance>),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Uuid); // 客户端订单ID结构

impl std::fmt::Display for ClientOrderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0) // 实现格式化显示
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ClientStatus {
    Connected,    // 已连接
    CancelOnly,   // 仅取消
    Disconnected, // 已断开
}
