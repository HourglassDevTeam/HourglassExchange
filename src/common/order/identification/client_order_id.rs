use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display, sync::LazyLock};

// **ClientOrderId**
// - **定义和作用**：`ClientOrderId` 是由客户端生成的，主要用于客户端内部的订单管理和跟踪。它在客户端内唯一，可以帮助用户追踪订单状态，而不需要等待交易所生成的 `OrderID`。
// - **设计合理性**：`ClientOrderId` 的设计对于提高用户体验非常有用，特别是在订单提交后用户可以立即获取订单状态信息。对于未来扩展成的Web或手机App，这种设计能够提供更好的响应速度和用户交互体验。然而，需要注意的是，`ClientOrderId` 在系统中应该保持唯一性，并与 `OrderID` 关联，以防止冲突。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientOrderId(pub Option<String>); // 可选的字符串类型辅助标识符

// 为 ClientOrderId 实现格式化显示
impl Display for ClientOrderId
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
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
/// static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]{6,20}$").unwrap());
///
/// assert!(ID_REGEX.is_match("abc123")); // 有效的 ID
/// assert!(ID_REGEX.is_match("A1_B2-C3")); // 包含下划线和连字符的有效 ID
/// assert!(!ID_REGEX.is_match("ab")); // 太短
/// assert!(!ID_REGEX.is_match("abc!@#")); // 包含无效字符
/// assert!(!ID_REGEX.is_match("a".repeat(21).as_str())); // 太长
/// ```
///
/// 此正则表达式特别适用于确保用户生成的 `ClientOrderId` 值符合预期格式，
/// 从而减少因格式错误的 ID 导致的错误概率。
static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]{6,20}$").unwrap());

impl ClientOrderId
{
    // 用户自定义或生成唯一的字符串ID
    pub fn new(custom_id: Option<String>) -> Result<Self, String>
    {
        if let Some(ref id) = custom_id {
            if Self::validate_id_format(id) {
                Ok(ClientOrderId(Some(id.clone())))
            }
            else {
                Err("Invalid ClientOrderId format".into())
            }
        }
        else {
            // If no custom ID is provided, return `None`.
            Ok(ClientOrderId(None))
        }
    }

    // 验证 ID 格式
    pub(crate) fn validate_id_format(id: &str) -> bool
    {
        ID_REGEX.is_match(id)
    }
}
