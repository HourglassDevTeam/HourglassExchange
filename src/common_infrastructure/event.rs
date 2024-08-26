use std::fmt::Formatter;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    common_infrastructure::{
        balance::TokenBalance,
        order::{Cancelled, FullyFill, Open, Order, PartialFill},
        position::AccountPositions,
        trade::ClientTrade,
    },
    sandbox::account::account_config::AccountConfig,
    Exchange,
};

/// NOTE: 如果需要记录交易所的时间戳，可以再添加一个专门的字段来表示交易所的时间，例如：    pub exchange_ts: DateTime<Utc> or i64
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountEvent
{
    pub exchange_timestamp: i64,   // 交易所发送事件的时间,
    pub exchange: Exchange, // 目标和源头交易所
    pub kind: AccountEventKind,    // 事件类型
}

/// 定义账户事件[`AccountEvent`]的类型。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AccountEventKind
{
    // Order Events
    OrdersOpen(Vec<Order<Open>>),
    OrdersNew(Vec<Order<Open>>),
    OrdersCancelled(Vec<Order<Cancelled>>),
    OrdersFilled(Vec<Order<FullyFill>>),
    OrdersPartiallyFilled(Vec<Order<PartialFill>>),
    Balance(TokenBalance),
    Trade(ClientTrade),
    Balances(Vec<TokenBalance>),
    Positions(AccountPositions),
    AccountConfig(AccountConfig),
    // OrderBookUpdate(OrderBookUpdate),
    // MarketStatus(MarketStatus),
    // MarginUpdate(MarginUpdate),
    // Transfer(Transfer),
    // Deposit(Deposit),
    // Withdrawal(Withdrawal),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Uuid); // 客户端订单ID结构

// 为ClientOrderId实现格式化显示
impl std::fmt::Display for ClientOrderId
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}
