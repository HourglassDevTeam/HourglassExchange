pub mod order_instructions;
pub mod states;

use crate::{
    common::{instrument::Instrument, order::order_instructions::OrderInstruction, Side},
    Exchange,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::sync::LazyLock;

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

/// 用于验证 `ClientOrderId` 格式的静态正则表达式。
///
/// 此 `LazyLock` 变量初始化了一个 `Regex` 模式，用于强制执行以下规则:
///
/// - **允许的字符:** `ClientOrderId` 只能包含字母（A-Z, a-z）、数字（0-9）、
///   下划线 (`_`) 和连字符 (`-`)。
///
/// - **长度:** `ClientOrderId` 的长度必须在 6 到 20 个字符之间。这确保了 ID 既不会太短而无意义，
///   也不会太长而繁琐。
///
/// ### 示例
///
/// ```rust
/// use regex::Regex;
/// use std::sync::LazyLock;
///
/// static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
///     Regex::new(r"^[a-zA-Z0-9_-]{6,20}$").unwrap()
/// });
///
/// assert!(ID_REGEX.is_match("abc123"));      // 有效的 ID
/// assert!(ID_REGEX.is_match("A1_B2-C3"));    // 包含下划线和连字符的有效 ID
/// assert!(!ID_REGEX.is_match("ab"));         // 太短
/// assert!(!ID_REGEX.is_match("abc!@#"));     // 包含无效字符
/// assert!(!ID_REGEX.is_match("a".repeat(21).as_str())); // 太长
/// ```
///
/// 此正则表达式特别适用于确保用户生成的 `ClientOrderId` 值符合预期格式，
/// 从而减少因格式错误的 ID 导致的错误概率。
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
            // If no custom ID is provided, return `None`.
            Ok(ClientOrderId(None))
        }
    }


    // 验证 ID 格式
    fn validate_id_format(id: &str) -> bool {
        ID_REGEX.is_match(id)
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
