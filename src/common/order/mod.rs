pub mod order_instructions;
pub mod states;

use std::fmt;
use std::fmt::{Display};
use std::sync::LazyLock;
use regex::Regex;
use crate::{
    common::{instrument::Instrument, order::order_instructions::OrderInstruction, Side},
    Exchange,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Order<State>
{
    pub kind: OrderInstruction,         // 订单指令
    pub exchange: Exchange,             // 交易所
    pub instrument: Instrument,         // 交易工具
    pub client_ts: i64,                 // 客户端下单时间
    pub client_order_id: ClientOrderId, // 客户端订单ID
    pub side: Side,                     // 买卖方向
    pub state: State,                   // 订单状态
}

#[derive(Debug, Copy, Clone, PartialOrd, Serialize, Deserialize, PartialEq)]
pub enum OrderRole
{
    Maker,
    Taker,
}


/// **OrderID**
///    - **定义和作用**：`OrderID` 通常由交易所生成，用于唯一标识某个订单。它是系统级的[标识符]，不同的[交易所]、不同的[订单类型]，都会生成不同的 `OrderID`。
///    - **设计合理性**：`OrderID` 的设计适合需要与交易所交互的场景，因为它通常是交易所内部或者[交易所]与[客户端]之间的标准标识符。
///  在扩展到To C端的Web应用和手机App时，`OrderID` 可以作为交易记录的[唯一标识符]，确保订单操作的[一致性]和[可追溯性]。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub String);

impl<Id> From<Id> for OrderId where Id: Display
{
    fn from(id: Id) -> Self
    {
        Self(id.to_string())
    }
}

//
// **ClientOrderId**
// - **定义和作用**：`ClientOrderId` 是由客户端生成的，主要用于客户端内部的订单管理和跟踪。它在客户端内唯一，可以帮助用户追踪订单状态，而不需要等待交易所生成的 `OrderID`。
// - **设计合理性**：`ClientOrderId` 的设计对于提高用户体验非常有用，特别是在订单提交后用户可以立即获取订单状态信息。对于未来扩展成的Web或手机App，这种设计能够提供更好的响应速度和用户交互体验。然而，需要注意的是，`ClientOrderId` 在系统中应该保持唯一性，并与 `OrderID` 关联，以防止冲突。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Option<String>); // 可选的字符串类型辅助标识符

// 为 ClientOrderId 实现格式化显示
impl Display for ClientOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_deref().unwrap_or("None"))
    }
}


// Initialize a static variable with LazyLock
static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9_-]{6,20}$").unwrap()
});


impl ClientOrderId {
    // 用户自定义或生成唯一的字符串ID
    pub fn new(custom_id: Option<String>) -> Result<Self, String> {
        if let Some(ref id) = custom_id {
            if Self::validate_id_format(id) {
                Ok(ClientOrderId(Some(id.clone())))
            } else {
                Err("Invalid ClientOrderId format".into())
            }
        } else {
            // 如果没有提供自定义ID，则生成一个唯一的字符串
            Ok(ClientOrderId(Some(Self::generate_unique_id())))
        }
    }

    // 验证 ID 格式
    fn validate_id_format(id: &str) -> bool {
        let re = Regex::new(r"^[a-zA-Z0-9_-]{6,20}$").unwrap(); // 允许的格式：6-20个字符，只允许字母、数字、下划线和短划线
        re.is_match(id)
    }

    // 自动生成唯一的 ID
    fn generate_unique_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let timestamp = chrono::Utc::now().timestamp_millis();
        let random_number: u64 = rng.gen_range(100000..999999);
        format!("{}-{}", timestamp, random_number)
    }
}
#[cfg(test)]
mod tests
{
    use crate::common::order::states::{cancelled::Cancelled, open::Open, pending::Pending, request_cancel::RequestCancel, request_open::RequestOpen};

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
        let order_id = OrderId("123".to_string());
        let cancel_request: RequestCancel = order_id.clone().into();
        assert_eq!(cancel_request.id, order_id);
    }

    #[test]
    fn open_order_remaining_quantity_should_be_calculated_correctly()
    {
        let open_order = Open { id: OrderId("123".to_string()),
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
