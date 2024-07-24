use crate::common_skeleton::order::{Order, RequestOpen};
use crate::common_skeleton::order::OrderId;

/// NOTE 在需要模拟延迟的回测场景下仅使用这种Open订单状态。
#[allow(dead_code)]
pub struct SimulatedOpen
{
    pub processed_ts: i64, // NOTE this timestamp is only applicable in the simulated environment.
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
    pub filled_quantity: f64,
}

impl From<(OrderId, i64, Order<RequestOpen>)> for Order<SimulatedOpen> {
    fn from((id, exchange_ts, request): (OrderId, i64, Order<RequestOpen>)) -> Self {
        Self {
            kind: request.kind,
            exchange: request.exchange.clone(),
            instrument: request.instrument.clone(),
            cid: request.cid,
            client_ts: request.client_ts,
            side: request.side,
            state: SimulatedOpen {
                processed_ts: exchange_ts,
                id,
                price: request.state.price,
                size: request.state.size,
                filled_quantity: 0.0,
            },
        }
    }
}
