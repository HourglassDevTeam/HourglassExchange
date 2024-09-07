use serde::{Deserialize, Serialize};
// 引入相关模块和结构体。
use crate::{
    common::{
        instrument::Instrument,
        order::identification::{client_order_id::ClientOrderId, OrderId},
        Side,
    },
    Exchange,
};

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct ClientTrade
{
    pub exchange: Exchange,
    pub timestamp: i64,
    pub trade_id: ClientTradeId,
    pub order_id: OrderId,
    pub cid: Option<ClientOrderId>,
    pub instrument: Instrument,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub fees: f64,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct ClientTradeId(pub i64);

impl<S> From<S> for ClientTradeId where S: Into<i64>
{
    fn from(id: S) -> Self
    {
        Self(id.into())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::{
        instrument::{kind::InstrumentKind, InstrumentInitiator},
        token::Token,
    };

    #[test]
    fn instrument_display_should_format_correctly()
    {
        let instrument = Instrument::new(Token::new("BTC"), Token::new("USDT"), InstrumentKind::Spot);
        assert_eq!(format!("{}", instrument), "(BTC_USDT, spot)");
    }

    #[test]
    fn instrument_should_be_comparable()
    {
        let instrument1 = Instrument::new(Token::new("BTC"), Token::new("USDT"), InstrumentKind::Spot);
        let instrument2 = Instrument::new(Token::new("ETH"), Token::new("USDT"), InstrumentKind::Spot);
        assert!(instrument1 < instrument2);
    }

    #[test]
    fn instrument_from_tuple_should_work()
    {
        let instrument = Instrument::from((Token::new("BTC"), Token::new("USDT"), InstrumentKind::Spot));
        assert_eq!(format!("{}", instrument), "(BTC_USDT, spot)");
    }

    #[test]
    fn instrument_initiator_should_create_instrument()
    {
        let initiator = InstrumentInitiator::new().base(Token::new("BTC")).quote(Token::new("USDT")).kind(InstrumentKind::Spot);
        let instrument = initiator.initiate().expect("Failed to create instrument");
        assert_eq!(format!("{}", instrument), "(BTC_USDT, spot)");
    }

    #[test]
    fn instrument_initiator_should_fail_if_missing_base()
    {
        let initiator = InstrumentInitiator::new().quote(Token::new("USDT")).kind(InstrumentKind::Spot);
        let result = initiator.initiate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Base is missing");
    }

    #[test]
    fn instrument_initiator_should_fail_if_missing_quote()
    {
        let initiator = InstrumentInitiator::new().base(Token::new("BTC")).kind(InstrumentKind::Spot);
        let result = initiator.initiate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Quote is missing");
    }

    #[test]
    fn instrument_initiator_should_fail_if_missing_kind()
    {
        let initiator = InstrumentInitiator::new().base(Token::new("BTC")).quote(Token::new("USDT"));
        let result = initiator.initiate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Instrument kind is missing");
    }
}
