use crate::universal::{
    balance::TokenPosition,
    order::{Cancelled, Opened, Order},
    trade::Trade,
};
use cerebro_integration::model::Exchange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use uuid::Uuid;

/// NOTE: 如果需要记录交易所的时间戳，可以再添加一个专门的字段来表示交易所的时间，例如：    pub exchange_ts: DateTime<Utc> or i64
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientAccountEvent {
    // pub exchange_ts: DateTime<Utc>, // 交易所接收到事件的时间,
    pub client_ts: DateTime<Utc>, // 客户端接收到事件的时间, NOTE 类型待定 i64
    pub exchange: Exchange,       // 目标和源头交易所
    pub kind: AccountEventKind,   // 事件类型
}

/// 定义账户事件[`ClientAccountEvent`]的类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AccountEventKind {
    // HTTP Events
    OrdersOpen(Vec<Order<Opened>>),
    OrdersNew(Vec<Order<Opened>>),
    OrdersCancelled(Vec<Order<Cancelled>>),
    // OrdersFilled(Vec<Order<Filled>>),
    // OrdersPartiallyFilled(Vec<Order<PartiallyFilled>>),
    // ...
    // WebSocket Only - 仅限WebSocket
    Balance(TokenPosition),
    Trade(Trade),
    // PriceUpdate(PriceUpdate),
    // OrderBookUpdate(OrderBookUpdate),
    // MarketStatus(MarketStatus),
    // ...
    // HTTP & WebSocket - HTTP和WebSocket
    Balances(Vec<TokenPosition>),
    // Positions(Vec<Position>),
    // AccountInfo(AccountInfo),
    // MarginUpdate(MarginUpdate),
    // Transfer(Transfer),
    // Deposit(Deposit),
    // Withdrawal(Withdrawal),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Uuid); // 客户端订单ID结构

// 为ClientOrderId实现格式化显示
impl std::fmt::Display for ClientOrderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
