use crate::common::order::identification::OrderId;
/// `OrderFills` 的作用在系统中通常是用于跟踪订单的执行状态，尤其是当订单部分或完全被成交时，`OrderFills` 可以记录相关信息，如成交的数量、价格等。
///  这对于分析订单执行的详细情况、生成报告或调试系统中的问题非常有用。
///
/// 在系统中， `OrderFills` 结构目前没有被使用，可能存在以下使用场景：
///
/// ### **可能的使用场景**：
/// - **订单历史跟踪**：`OrderFills` 可以被用来记录订单的执行历史，特别是在部分成交的情况下。你可以在订单部分或完全成交时生成 `OrderFills`，并将它们保存到订单历史记录中。
/// - **调试和分析**：在系统发生异常或需要详细分析订单执行时，可以利用 `OrderFills` 追踪每个订单的执行细节。
/// - **报表生成**：`OrderFills` 也可以用来生成关于订单执行情况的详细报表，帮助用户理解订单是如何被市场执行的。
///
/// ### **如何利用 `OrderFills`**：
/// - **修改 `generate_client_trade_event` 函数**：在生成 `ClientTrade` 事件之前，首先生成一个对应的 `OrderFill`（如 `PartialFill` 或 `FullyFill`），并将其存储在相应的数据结构中。
/// - **保存 `OrderFills`**：可以考虑将生成的 `OrderFills` 存储在订单的状态中，或者持久化到数据库中以便后续查询。
/// - **扩展系统功能**：如果未来需要更加详细的订单执行状态管理，`OrderFills` 可以作为基础结构被扩展和利用。
use serde::{Deserialize, Serialize};

/// `FullyFill` 结构体表示订单完全成交的状态。
/// 完全成交状态意味着订单的所有数量已经被匹配和执行。
/// 在订单完全成交后，订单通常会从 `AccountOrders` 中删除。
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FullyFill
{
    /// 完全成交的订单ID，唯一标识订单。
    pub id: OrderId,
    /// 完全成交时的价格。
    pub price: f64,
    /// 完全成交的订单数量。
    pub size: f64,
}

/// `PartialFill` 结构体表示订单部分成交的状态。
/// 部分成交状态意味着订单的部分数量已经被匹配和执行，
/// 但订单仍然在 `AccountOrders` 中保留，以待后续的成交。
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PartialFill
{
    /// 部分成交的订单ID，唯一标识订单。
    pub id: OrderId,
    /// 部分成交时的价格。
    pub price: f64,
    /// 部分成交的订单数量。
    pub size: f64,
}
