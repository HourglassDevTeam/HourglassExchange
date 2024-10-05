use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

pub mod account_positions;
pub mod balance; // 通用balance模块
pub mod datafeed;
pub mod event; // 定义通用事件和状态
pub mod friction;
pub mod instrument;
pub mod order;
pub mod stable_token;
pub mod status;
pub mod token;
pub mod token_list;
pub mod trade;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Side
{
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}

impl Display for Side
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | Side::Buy => "buy",
            | Side::Sell => "sell",
        })
    }
}

impl FromStr for Side
{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        match s.to_lowercase().as_str() {
            | "buy" | "b" => Ok(Side::Buy),
            | "sell" | "s" => Ok(Side::Sell),
            | _ => Err(format!("'{}' is not a valid Side", s)),
        }
    }
}

impl Side
{
    /// 如果你在某个逻辑中需要反转订单的方向，比如在进行某种对冲交易时，需要根据当前订单方向生成相反方向的订单，那么可以使用 toggle 方法。
    pub fn toggle(&self) -> Self
    {
        match self {
            | Side::Buy => Side::Sell,
            | Side::Sell => Side::Buy,
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn side_display_should_format_correctly()
    {
        assert_eq!(format!("{}", Side::Buy), "buy");
        assert_eq!(format!("{}", Side::Sell), "sell");
    }

    #[test]
    fn side_from_str_should_convert_correctly()
    {
        assert_eq!(Side::from_str("buy").unwrap(), Side::Buy);
        assert_eq!(Side::from_str("BUY").unwrap(), Side::Buy);
        assert_eq!(Side::from_str("b").unwrap(), Side::Buy);
        assert_eq!(Side::from_str("sell").unwrap(), Side::Sell);
        assert_eq!(Side::from_str("SELL").unwrap(), Side::Sell);
        assert_eq!(Side::from_str("s").unwrap(), Side::Sell);
    }

    #[test]
    fn side_from_str_should_return_error_for_invalid_input()
    {
        assert!(Side::from_str("invalid").is_err());
    }

    #[test]
    fn side_toggle_should_switch_side()
    {
        assert_eq!(Side::Buy.toggle(), Side::Sell);
        assert_eq!(Side::Sell.toggle(), Side::Buy);
    }
}
