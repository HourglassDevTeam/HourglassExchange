pub mod identification;
pub mod order_instructions;
pub mod states;

use crate::{
    common::{
        instrument::Instrument,
        order::{identification::client_order_id::ClientOrderId, order_instructions::OrderInstruction},
        Side,
    },
    Exchange,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State>
{
    pub kind: OrderInstruction, // 订单指令
    pub exchange: Exchange,     // 交易所
    pub instrument: Instrument, // 交易工具
    pub timestamp: i64,         // 生成的时候填客户端下单时间,NOTE 回测场景中之后会被加上一个随机延迟时间。
    pub cid: ClientOrderId,     // 客户端订单ID
    pub side: Side,             // 买卖方向
    pub state: State,           // 订单状态
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
    use super::*;
    use crate::common::order::states::{request_cancel::RequestCancel, request_open::RequestOpen};

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
        let req1 = RequestOpen {
            reduce_only: true,
            price: 50.0,
            size: 1.0,
        };
        let req2 = RequestOpen {
            reduce_only: false,
            price: 60.0,
            size: 2.0,
        };
        assert!(req1 < req2);
    }


    #[test]
    fn request_cancel_should_create_from_order_id()
    {
        let order_id = crate::common::order::identification::OrderId(123);
        let cancel_request: RequestCancel = order_id.clone().into();
        assert_eq!(cancel_request.id, order_id);
    }
}
