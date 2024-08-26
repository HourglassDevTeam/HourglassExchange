use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind
{
    Spot,
    #[serde(alias = "Swap", alias = "SWAP", alias = "PERPETUAL")]
    Perpetual,
    Future,
    CryptoOption,
    CryptoLeveragedToken,
    CommodityOption,
    CommodityFuture,
}

impl Default for InstrumentKind
{
    fn default() -> Self
    {
        Self::Spot
    }
}

impl Display for InstrumentKind
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self {
            | InstrumentKind::Spot => write!(f, "spot"),
            | InstrumentKind::Future => {
                write!(f, "future")
            }
            | InstrumentKind::Perpetual => write!(f, "perpetual"),
            | InstrumentKind::CryptoOption => {
                write!(f, "option")
            }
            | InstrumentKind::CryptoLeveragedToken => {
                write!(f, "margin")
            }
            | InstrumentKind::CommodityFuture => {
                write!(f, "commodity_future")
            }
            | InstrumentKind::CommodityOption => {
                write!(f, "commodity_option")
            }
        }
    }
}

impl TryFrom<String> for InstrumentKind
{
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error>
    {
        match s.as_str() {
            | "Spot" => Ok(InstrumentKind::Spot),
            | "Perpetual" => Ok(InstrumentKind::Perpetual),
            | "Future" => Ok(InstrumentKind::Future),
            | "Option" => Ok(InstrumentKind::CryptoOption),
            | "Margin" => Ok(InstrumentKind::CryptoLeveragedToken),
            _ => {
                Err(format!("Unknown instrument kind: {}", s))
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrument_kind_display_should_format_correctly() {
        assert_eq!(format!("{}", InstrumentKind::Spot), "spot");
        assert_eq!(format!("{}", InstrumentKind::Perpetual), "perpetual");
        assert_eq!(format!("{}", InstrumentKind::Future), "future");
        assert_eq!(format!("{}", InstrumentKind::CryptoOption), "option");
        assert_eq!(format!("{}", InstrumentKind::CryptoLeveragedToken), "margin");
        assert_eq!(format!("{}", InstrumentKind::CommodityFuture), "commodity_future");
        assert_eq!(format!("{}", InstrumentKind::CommodityOption), "commodity_option");
    }

    #[test]
    fn instrument_kind_default_should_return_spot() {
        assert_eq!(InstrumentKind::default(), InstrumentKind::Spot);
    }

    #[test]
    fn instrument_kind_from_string_should_convert_correctly() {
        assert_eq!(InstrumentKind::try_from("Spot".to_string()), Ok(InstrumentKind::Spot));
        assert_eq!(InstrumentKind::try_from("Perpetual".to_string()), Ok(InstrumentKind::Perpetual));
        assert_eq!(InstrumentKind::try_from("Future".to_string()), Ok(InstrumentKind::Future));
        assert_eq!(InstrumentKind::try_from("Option".to_string()), Ok(InstrumentKind::CryptoOption));
        assert_eq!(InstrumentKind::try_from("Margin".to_string()), Ok(InstrumentKind::CryptoLeveragedToken));
    }

    #[test]
    fn instrument_kind_from_string_should_return_err_on_unknown_kind() {
        let result = InstrumentKind::try_from("Unknown".to_string());
        assert!(result.is_err(), "Expected an error for unknown instrument kind");
    }
}