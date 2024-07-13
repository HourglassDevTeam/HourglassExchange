use crate::universal::event::ClientOrderId;
use crate::universal::token::Token;
use crate::universal::instrument::Instrument;
use crate::universal::Side;
use cerebro_integration::model::{
    Exchange,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

/// 订单类型枚举
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum OrderKind {
    /// 市价单
    /// 行为：以当前市场价格立即执行订单。
    Market,
    /// 限价单
    /// 行为：订单只有在达到指定价格或更好的价格时才会执行。
    Limit,
    /// 仅挂单
    /// 行为：订单只会添加到订单簿，不会立即与现有订单匹配。如果订单会立即执行，则会被取消。
    /// 优点：确保订单会作为挂单存在，通常用于提供流动性。
    /// 缺点：无法立即执行订单。
    /// 应用场景：当希望成为市场的流动性提供者，并且不希望订单立即执行时使用。
    ImmediateOrCancel,
    /// 全成或全撤单
    /// 行为：订单必须立即完全执行，否则将被取消。
    /// 优点：确保订单要么全部执行，要么完全取消。
    /// 缺点：如果无法完全执行，订单会被取消。
    /// 应用场景：当希望订单立即完全执行，而不是部分执行时使用。
    FillOrKill,
    /// 有效直到取消
    /// 行为：订单将一直保持有效，直到被执行或手动取消。
    /// 优点：订单不会因时间限制而过期。
    /// 缺点：订单可能会长期挂在订单簿上。
    /// 应用场景：当希望订单在达到指定价格时被执行，而不考虑时间限制时使用。
    GoodTilCancelled,
}

impl Display for OrderKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            | OrderKind::Market => "market",
            | OrderKind::Limit => "limit",
            | OrderKind::ImmediateOrCancel => "immediate_or_cancel (IOC)",
            | OrderKind::FillOrKill => "fill_or_kill (FOK)",
            | OrderKind::GoodTilCancelled => "good_til_cancelled (GTC)",
            // | OrderKind::Stop => "stop",
            // | OrderKind::StopLimit => "stop_limit",
            // | OrderKind::TrailingStop => "trailing_stop",
            // | OrderKind::Iceberg => "iceberg",
        })
    }
}

/// 订单结构体，注意State在这里是泛型
#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State> {
    pub exchange: Exchange,     // 交易所
    pub instrument: Instrument, // 交易工具
    // Consider : 需要记录 OrderId 吗 ????
    pub cid: ClientOrderId, // 客户端订单ID
    pub side: Side,         // 买卖方向
    pub state: State,       // 订单状态
}

/// 订单初始状态。发送到client进行操作
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct RequestOpen {
    pub kind: OrderKind,
    pub price: f64,
    pub quantity: f64,
}

// NOTE that this needs to be adjusted according to the specifics of our trading instruments.
impl Order<RequestOpen> {
    pub fn calculate_required_available_balance(&self) -> (&Token, f64) {
        match self.side {
            | Side::Buy => (&self.instrument.quote, self.state.price * self.state.quantity),
            | Side::Sell => (&self.instrument.base, self.state.quantity),
        }
    }
}

/// 发送RequestOpen到client后尚未收到确认响应时的状态
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Pending;

/// 在RequestCancel结构体中只记录OrderId的原因主要是因为取消订单操作通常只需要知道哪个订单需要被取消。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel {
    pub id: OrderId, // Consider : 需要记录 CID 吗 ????
}

// 从Id直接生成RequestCancel
impl<Id> From<Id> for RequestCancel
where
    Id: Into<OrderId>,
{
    fn from(id: Id) -> Self {
        Self { id: id.into() }
    }
}

/// 打开状态的订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Opened {
    pub id: OrderId,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64, /* or remain
                               * or remaining quantity  , essentially the same. */
}

impl Opened {
    pub fn remaining_quantity(&self) -> f64 {
        self.quantity - self.filled_quantity
    }
}

/// 完全成交状态的订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FullyFilled {
    pub id: OrderId,
    pub price: f64,
    pub quantity: f64,
}

/// 使得Order<Open> 之间可以比较大小
/// NOTE: 此处Self 等同于 Order<Open>，表示 other 参数也是一个 Order<Open> 类型的引用。
impl Ord for Order<Opened> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .unwrap_or_else(|| panic!("[UniLinkExecution] : {:?}.partial_cmp({:?}) impossible", self, other))
    }
}

impl PartialOrd for Order<Opened> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.side, other.side) {
            | (Side::Buy, Side::Buy) => match self.state.price.partial_cmp(&other.state.price)? {
                | Ordering::Equal => self.state.remaining_quantity().partial_cmp(&other.state.remaining_quantity()),
                | non_equal => Some(non_equal),
            },
            | (Side::Sell, Side::Sell) => match other.state.price.partial_cmp(&self.state.price)? {
                | Ordering::Equal => other.state.remaining_quantity().partial_cmp(&self.state.remaining_quantity()),
                | non_equal => Some(non_equal),
            },
            | _ => None,
        }
    }
}

// 为Order<Open>实现Eq
impl Eq for Order<Opened> {}

/// 构建订单在被取消后的状态
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Cancelled {
    pub id: OrderId,
}

impl<Id> From<Id> for Cancelled
where
    Id: Into<OrderId>,
{
    fn from(id: Id) -> Self {
        Self { id: id.into() }
    }
}

/// 订单ID / OrderId，应当由交易所生成。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub String);

impl<Id> From<Id> for OrderId
where
    Id: Display,
{
    fn from(id: Id) -> Self {
        Self(id.to_string())
    }
}

impl From<&Order<RequestOpen>> for Order<Pending> {
    fn from(request: &Order<RequestOpen>) -> Self {
        Self {
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            side: request.side,
            state: Pending,
        }
    }
}

impl From<(OrderId, Order<RequestOpen>)> for Order<Opened> {
    fn from((id, request): (OrderId, Order<RequestOpen>)) -> Self {
        Self {
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            side: request.side,
            state: Opened {
                id,
                price: request.state.price,
                quantity: request.state.quantity,
                filled_quantity: 0.0,
            },
        }
    }
}

impl From<Order<Opened>> for Order<Cancelled> {
    fn from(order: Order<Opened>) -> Self {
        Self {
            exchange: order.exchange.clone(),
            instrument: order.instrument.clone(),
            cid: order.cid,
            side: order.side,
            state: Cancelled { id: order.state.id },
        }
    }
}
