pub mod order_instructions;
pub mod states;
pub mod identification;

use crate::{
    common::{instrument::Instrument, order::order_instructions::OrderInstruction, Side},
    Exchange,
};
use serde::{Deserialize, Serialize};
use crate::common::order::identification::client_order_id::ClientOrderId;

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State>
{
    pub kind: OrderInstruction,         // 订单指令
    pub exchange: Exchange,             // 交易所
    pub instrument: Instrument,         // 交易工具
    pub client_ts: i64,                 // 客户端下单时间
    pub cid: ClientOrderId, // 客户端订单ID
    pub side: Side,                     // 买卖方向
    pub state: State,                   // 订单状态
}

#[derive(Debug, Copy, Clone, PartialOrd, Serialize, Deserialize, PartialEq)]
pub enum OrderRole
{
    Maker,
    Taker,
}

#[cfg(test)]
mod tests
{
    use crate::common::order::states::{cancelled::Cancelled, open::Open, pending::Pending, request_cancel::RequestCancel, request_open::RequestOpen};
    use crate::common::order::identification::OrderId;
    use super::*;

    #[test]
    fn order_execution_type_display_should_format_correctly()
    {
        assert_eq!(format!("{}", OrderInstruction::Market), "market");
        assert_eq!(format!("{}", OrderInstruction::Limit), "limit");
        assert_eq!(format!("{}", OrderInstruction::PostOnly), "post_only");
        assert_eq!(format!("{}", OrderInstruction::ImmediateOrCancel), "immediate_or_cancel");
        assert_eq!(format!("{}", OrderInstruction::FillOrKill), "fill_or_kill");
        assert_eq!(format!("{}", OrderInstruction::GoodTilCancelled), "good_til_cancelled");
    }

    #[test]
    fn request_open_should_be_comparable()
    {
        let req1 = RequestOpen { reduce_only: true,
                                 price: 50.0,
                                 size: 1.0 };
        let req2 = RequestOpen { reduce_only: false,
                                 price: 60.0,
                                 size: 2.0 };
        assert!(req1 < req2);
    }

    #[test]
    fn pending_should_be_comparable()
    {
        let pending1 = Pending { reduce_only: true,
                                 price: 50.0,
                                 size: 1.0,
                                 predicted_ts: 1000 };
        let pending2 = Pending { reduce_only: false,
                                 price: 60.0,
                                 size: 2.0,
                                 predicted_ts: 2000 };
        assert!(pending1 < pending2);
    }

    #[test]
    fn request_cancel_should_create_from_order_id()
    {
        let order_id = crate::common::order::identification::OrderId("123".to_string());
        let cancel_request: RequestCancel = order_id.clone().into();
        assert_eq!(cancel_request.id, order_id);
    }

    #[test]
    fn open_order_remaining_quantity_should_be_calculated_correctly()
    {
        let open_order = Open { id: crate::common::order::identification::OrderId("123".to_string()),
                                price: 50.0,
                                size: 10.0,
                                filled_quantity: 3.0,
                                order_role: OrderRole::Maker,
                                received_ts: 1000 };
        assert_eq!(open_order.remaining_quantity(), 7.0);
    }

    #[test]
    fn order_id_should_convert_from_string()
    {
        let order_id: OrderId = "123".to_string().into();
        assert_eq!(order_id.0, "123");
    }

    #[test]
    fn order_id_should_convert_to_cancelled()
    {
        let order_id: OrderId = "123".to_string().into();
        let cancelled_order: Cancelled = order_id.into();
        assert_eq!(cancelled_order.id.0, "123");
    }
}
