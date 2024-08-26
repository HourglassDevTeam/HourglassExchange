use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{event::ClientOrderId, instrument::Instrument, Side},
    Exchange,
};

/// 订单类型枚举
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum OrderExecutionType
{
    Market,
    Limit,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
    GoodTilCancelled,
}

impl Display for OrderExecutionType
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | OrderExecutionType::Market => "market",
            | OrderExecutionType::Limit => "limit",
            | OrderExecutionType::ImmediateOrCancel => "immediate_or_cancel",
            | OrderExecutionType::FillOrKill => "fill_or_kill",
            | OrderExecutionType::GoodTilCancelled => "good_til_cancelled",
            | OrderExecutionType::PostOnly => "post_only",
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
    pub kind: OrderExecutionType,       // 订单种类
    pub exchange: Exchange,             // 交易所
    pub instrument: Instrument,         // 交易工具
    pub client_ts: i64,                 // 客户端下单时间
    pub client_order_id: ClientOrderId, // 客户端订单ID
    pub side: Side,                     // 买卖方向
    pub state: State,                   // 订单状态
}

/// 订单初始状态。发送到client进行操作
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RequestOpen
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
}

/// FIXME this comparison is now only for the tests below but could be ill-logic.
impl PartialOrd for RequestOpen {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // First compare by `price`
        match self.price.partial_cmp(&other.price) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // Then compare by `size`
        match self.size.partial_cmp(&other.size) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // Finally compare by `reduce_only`
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}

/// 发送RequestOpen到client后尚未收到确认响应时的状态
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Pending
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
    pub(crate) predicted_ts: i64,
}


/// FIXME this comparison is now only for the tests below but could be ill-logic.
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // First compare by `price`
        match self.price.partial_cmp(&other.price) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // Then compare by `size`
        match self.size.partial_cmp(&other.size) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // Then compare by `predicted_ts`
        match self.predicted_ts.partial_cmp(&other.predicted_ts) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // Finally compare by `reduce_only`
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}


/// 在RequestCancel结构体中只记录OrderId的原因主要是因为取消订单操作通常只需要知道哪个订单需要被取消。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel
{
    pub id: OrderId,
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
               client_order_id: order.client_order_id,
               client_ts: order.client_ts,
               side: order.side,
               state: Cancelled { id: order.state.id } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_execution_type_display_should_format_correctly() {
        assert_eq!(format!("{}", OrderExecutionType::Market), "market");
        assert_eq!(format!("{}", OrderExecutionType::Limit), "limit");
        assert_eq!(format!("{}", OrderExecutionType::PostOnly), "post_only");
        assert_eq!(format!("{}", OrderExecutionType::ImmediateOrCancel), "immediate_or_cancel");
        assert_eq!(format!("{}", OrderExecutionType::FillOrKill), "fill_or_kill");
        assert_eq!(format!("{}", OrderExecutionType::GoodTilCancelled), "good_til_cancelled");
    }

    #[test]
    fn request_open_should_be_comparable() {
        let req1 = RequestOpen { reduce_only: true, price: 50.0, size: 1.0 };
        let req2 = RequestOpen { reduce_only: false, price: 60.0, size: 2.0 };
        assert!(req1 < req2);
    }

    #[test]
    fn pending_should_be_comparable() {
        let pending1 = Pending { reduce_only: true, price: 50.0, size: 1.0, predicted_ts: 1000 };
        let pending2 = Pending { reduce_only: false, price: 60.0, size: 2.0, predicted_ts: 2000 };
        assert!(pending1 < pending2);
    }

    #[test]
    fn request_cancel_should_create_from_order_id() {
        let order_id = OrderId("123".to_string());
        let cancel_request: RequestCancel = order_id.clone().into();
        assert_eq!(cancel_request.id, order_id);
    }

    #[test]
    fn open_order_remaining_quantity_should_be_calculated_correctly() {
        let open_order = Open {
            id: OrderId("123".to_string()),
            price: 50.0,
            size: 10.0,
            filled_quantity: 3.0,
            order_role: OrderRole::Maker,
            received_ts: 1000,
        };
        assert_eq!(open_order.remaining_quantity(), 7.0);
    }


    #[test]
    fn order_id_should_convert_from_string() {
        let order_id: OrderId = "123".to_string().into();
        assert_eq!(order_id.0, "123");
    }

    #[test]
    fn order_id_should_convert_to_cancelled() {
        let order_id: OrderId = "123".to_string().into();
        let cancelled_order: Cancelled = order_id.into();
        assert_eq!(cancelled_order.id.0, "123");
    }

}
