use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// 订单类型枚举
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum OrderInstruction
{
    Market,
    Limit,
    PostOnlyLimit,
    ImmediateOrCancel,
    FillOrKill,
    GoodTilCancelled,
    Cancel,
}

impl Display for OrderInstruction
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | OrderInstruction::Market => "market",
            | OrderInstruction::Limit => "limit",
            | OrderInstruction::ImmediateOrCancel => "immediate_or_cancel",
            | OrderInstruction::FillOrKill => "fill_or_kill",
            | OrderInstruction::GoodTilCancelled => "good_til_cancelled",
            | OrderInstruction::PostOnlyLimit => "post_only",
            | OrderInstruction::Cancel => "cancel_request",
        })
    }
}
