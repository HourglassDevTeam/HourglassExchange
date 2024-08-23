use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{event::ClientOrderId, instrument::Instrument, Side},
    ExchangeVariant,
};

/// 订单类型枚举
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum OrderKind
{
    Market,
    Limit,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
    GoodTilCancelled,
}

impl Display for OrderKind
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | OrderKind::Market => "market",
            | OrderKind::Limit => "limit",
            | OrderKind::ImmediateOrCancel => "immediate_or_cancel",
            | OrderKind::FillOrKill => "fill_or_kill",
            | OrderKind::GoodTilCancelled => "good_til_cancelled",
            | OrderKind::PostOnly => "post_only",
            // | OrderKind::Stop => "stop",
            // | OrderKind::StopLimit => "stop_limit",
            // | OrderKind::TrailingStop => "trailing_stop",
            // | OrderKind::Iceberg => "iceberg",
        })
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State>
{
    pub kind: OrderKind,           // 订单种类
    pub exchange: ExchangeVariant, // 交易所
    pub instrument: Instrument,    // 交易工具
    pub client_ts: i64,            // 客户端下单时间
    pub cid: ClientOrderId,        // 客户端订单ID
    pub side: Side,                // 买卖方向
    pub state: State,              // 订单状态
}

/// 订单初始状态。发送到client进行操作
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct RequestOpen
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
}

/// 发送RequestOpen到client后尚未收到确认响应时的状态
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Pending
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
    pub(crate) predicted_ts: i64,
}

/// 在RequestCancel结构体中只记录OrderId的原因主要是因为取消订单操作通常只需要知道哪个订单需要被取消。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel
{
    pub id: OrderId, // Consider : 需要记录 CID 吗 ????
}

// 从Id直接生成RequestCancel
impl<Id> From<Id> for RequestCancel where Id: Into<OrderId>
{
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}

/// 开放状态的订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Open
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
    pub filled_quantity: f64,
    pub order_role: OrderRole,
    pub received_ts: i64,
    // 交易所下单时间 NOTE this might be only applicable in a sandbox exchange. 流动性充足的情况下received到trade状态的时间差不超过2ms，并且是交易所端不可避免的。‘ */
    // pub expired_ts:i64, /* 交易所订单过期时间 NOTE this might be only applicable in a sandbox exchange.*/
}

impl Open
{
    pub fn remaining_quantity(&self) -> f64
    {
        self.size - self.filled_quantity
    }
}

/// 完全成交状态的订单, FullFill 以后在account层面 [AccountOrders] 删除对应open订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FullyFill
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
}

/// 完全成交状态的订单,FullFill 以后在account层面 [AccountOrders] 仍然保留open订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PartialFill
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Copy, Clone, PartialOrd, Serialize, Deserialize, PartialEq)]
pub enum OrderRole
{
    Maker,
    Taker,
}
impl Ord for Order<Open>
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        match self.state.price.partial_cmp(&other.state.price) {
            | Some(ordering) => ordering,
            | None => panic!("[UniLinkExecution] : Failed to compare prices. One of the prices is NaN: self = {:?}, other = {:?}",
                             self.state.price, other.state.price),
        }
    }
}

impl PartialOrd for Order<Open>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        // 使用 Ord 实现的 cmp 方法，这样 partial_cmp 可以继承 Ord 的错误处理逻辑
        Some(self.cmp(other))
    }
}

// 为Order<Open>实现Eq
impl Eq for Order<Open> {}

/// 构建订单在被取消后的状态
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Cancelled
{
    pub id: OrderId,
}

impl<Id> From<Id> for Cancelled where Id: Into<OrderId>
{
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}

/// 订单ID / OrderId，应当由交易所生成。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub String);

impl<Id> From<Id> for OrderId where Id: Display
{
    fn from(id: Id) -> Self
    {
        Self(id.to_string())
    }
}

impl From<Order<Open>> for Order<Cancelled>
{
    fn from(order: Order<Open>) -> Self
    {
        Self { kind: order.kind,
               exchange: order.exchange,
               instrument: order.instrument.clone(),
               cid: order.cid,
               client_ts: order.client_ts,
               side: order.side,
               state: Cancelled { id: order.state.id } }
    }
}
