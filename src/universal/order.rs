use crate::universal::event::ClientOrderId;

use cerebro_integration::model::{
    instrument::{symbol::Symbol, Instrument},
    Exchange, Side,
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
    PostOnly,
    /// 立即或取消
    /// 行为：订单会尽快以指定价格或更好的价格部分或全部执行，未执行的部分会被取消。
    /// 优点：确保订单能尽快执行，不会成为挂单。
    /// 缺点：部分订单可能被取消，未能全部执行。
    /// 应用场景：当希望订单尽快执行，但不希望部分订单留在订单簿上时使用。
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
    /// 当日有效
    /// 行为：订单在当天有效，如果当天未执行，则会被取消。
    /// 优点：订单在当天有效，避免长期挂单。
    /// 缺点：如果当天未能执行，订单会被取消。
    /// 应用场景：当希望订单在当天被执行，如果未执行则自动取消时使用。
    GoodForDay,
    /// 止损单
    /// 行为：当市场价格达到预设的触发价格时，订单会变成市价单。
    /// 优点：帮助限制损失或保护利润。
    /// 缺点：在市场剧烈波动时，执行价格可能会有所不同。
    /// 应用场景：用于设置止损点，保护投资免受过大损失。
    Stop,
    /// 止损限价单
    /// 行为：当市场价格达到预设的触发价格时，订单会变成限价单。
    /// 优点：结合了止损单和限价单的优点，控制执行价格。
    /// 缺点：如果市场价格快速波动，可能无法执行。
    /// 应用场景：希望在达到止损点时以特定价格或更好的价格执行订单。
    StopLimit,
    /// 跟踪止损单
    /// 行为：止损价格会随着市场价格的变动而调整，保持一定的距离。
    /// 优点：在锁定利润的同时，随着市场价格的变动，调整止损价格。
    /// 缺点：可能在市场剧烈波动时触发。
    /// 应用场景：在市场趋势有利时保护利润。
    TrailingStop,
    /// 冰山订单
    /// 行为：大订单分成多个小订单逐步显示在市场上。
    /// 优点：隐藏大订单的实际规模，减少对市场价格的影响。
    /// 缺点：可能增加执行时间。
    /// 应用场景：希望大订单逐步执行，减少对市场影响时使用。
    Iceberg,
}

impl Display for OrderKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            | OrderKind::Market => "market",
            | OrderKind::Limit => "limit",
            | OrderKind::PostOnly => "post_only",
            | OrderKind::ImmediateOrCancel => "immediate_or_cancel (IOC)",
            | OrderKind::FillOrKill => "fill_or_kill (FOK)",
            | OrderKind::GoodTilCancelled => "good_til_cancelled (GTC)",
            | OrderKind::GoodForDay => "good_for_day (GFD)",
            | OrderKind::Stop => "stop",
            | OrderKind::StopLimit => "stop_limit",
            | OrderKind::TrailingStop => "trailing_stop",
            | OrderKind::Iceberg => "iceberg",
        })
    }
}

/// 订单结构体，注意State在这里是泛型
#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State> {
    pub exchange: Exchange,     // 交易所
    pub instrument: Instrument, // 交易工具
    // NOTE 需要记录 OrderId 吗
    pub cid: ClientOrderId, // 客户端订单ID
    pub side: Side,         // 买卖方向
    pub state: State,       // 订单状态
}

/// 订单初始状态。发送到ClientExecution进行操作
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct RequestOpen {
    pub kind: OrderKind,
    pub price: f64,
    pub quantity: f64,
}

impl Order<RequestOpen> {
    /// 计算订单所需的可用余额
    pub fn required_available_balance(&self) -> (&Symbol, f64) {
        match self.side {
            | Side::Buy => (&self.instrument.quote, self.state.price * self.state.quantity),
            | Side::Sell => (&self.instrument.base, self.state.quantity),
        }
    }
}

/// 订单在发送RequestOpen到ClientExecution后尚未收到确认响应时的状态
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Pending;

/// 在RequestCancel结构体中只记录OrderId的原因主要是因为取消订单操作通常只需要知道哪个订单需要被取消。
/// 在大多数交易系统中，取消订单操作的主要任务是识别和定位要取消的订单，然后将取消请求发送给交易所或执行系统。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel {
    pub id: OrderId,
}

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
pub struct Open {
    pub id: OrderId,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
}

impl Open {
    pub fn remaining_quantity(&self) -> f64 {
        self.quantity - self.filled_quantity
    }
}

/// 订单成交状态
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub enum OrderFill {
    Full,    // 完全成交
    Partial, // 部分成交
}

/// NOTE: 此处Self 等同于 Order<Open>，表示 other 参数也是一个 Order<Open> 类型的引用。
impl Ord for Order<Open> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .unwrap_or_else(|| panic!("[UniLinkExecution] : {:?}.partial_cmp({:?}) impossible", self, other))
    }
}

/// NOTE partial_cmp 方法返回一个 Option<Ordering>，它表示两个值的部分比较结果。Option<Ordering> 可以有以下几种可能的值：
///      Some(Ordering::Less) - 表示第一个值小于第二个值。
///      Some(Ordering::Equal) - 表示两个值相等。
///      Some(Ordering::Greater) - 表示第一个值大于第二个值。
///      None - 表示两个值不能比较（通常用于某些部分排序无法确定的情况）。
impl PartialOrd for Order<Open> {
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
impl Eq for Order<Open> {}

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

impl From<(OrderId, Order<RequestOpen>)> for Order<Open> {
    fn from((id, request): (OrderId, Order<RequestOpen>)) -> Self {
        Self {
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            side: request.side,
            state: Open {
                id,
                price: request.state.price,
                quantity: request.state.quantity,
                filled_quantity: 0.0,
            },
        }
    }
}

impl From<Order<Open>> for Order<Cancelled> {
    fn from(order: Order<Open>) -> Self {
        Self {
            exchange: order.exchange.clone(),
            instrument: order.instrument.clone(),
            cid: order.cid,
            side: order.side,
            state: Cancelled { id: order.state.id },
        }
    }
}
