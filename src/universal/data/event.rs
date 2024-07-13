use crate::universal::trade::Trade;
use crate::{universal::instrument::Instrument, Exchange};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]

pub struct MarketEvent<T> {
    pub exchange_time: DateTime<Utc>,
    pub received_time: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: T,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]

pub enum DataKind {
    Trade(Trade),
    // OrderBook25(OrderBook25),
    // Candle(Candle),
    // Liquidation(Liquidation),
}

impl From<MarketEvent<Trade>> for MarketEvent<DataKind> {
    fn from(event: MarketEvent<Trade>) -> Self {
        Self {
            exchange_time: event.exchange_time,
            received_time: event.received_time,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Trade(event.kind),
        }
    }
}
