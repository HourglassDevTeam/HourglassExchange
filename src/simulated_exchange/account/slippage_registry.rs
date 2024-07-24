use crate::common_skeleton::{datafeed::event::MarketEvent, order::OrderId};
use crate::common_skeleton::order::{Cancelled, Open, Order, RequestOpen};


/// NOTE 在需要模拟延迟的回测场景下仅使用这种Pending状态。
#[allow(dead_code)]
pub struct SimulatedOpen
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
    pub filled_quantity: f64,
}

impl From<(OrderId, Order<RequestOpen>)> for Order<SimulatedOpen>
{
    fn from((id, request): (OrderId, Order<RequestOpen>)) -> Self
    {
        Self { kind: request.kind,
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            client_ts: request.client_ts,
            side: request.side,
            state: SimulatedOpen { id,
                price: request.state.price,
                size: request.state.size,
                filled_quantity: 0.0 } }
    }
}
