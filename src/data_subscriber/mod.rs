use crate::common_skeleton::instrument::Instrument;
use std::collections::HashMap;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;
pub mod connector;
pub mod socket_error;
pub mod subscriber;
mod validator;

#[derive(Debug)]
pub struct SubscriptionId(pub String);

pub struct SubscriptionMeta
{
    /// `HashMap` containing the mapping between a [`SubscriptionId`] and
    /// it's associated Cerebro [`Instrument`].
    pub instrument_map: Map<Instrument>,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub subscriptions: Vec<WsMessage>,
}

pub struct Map<T>(pub HashMap<SubscriptionId, T>);
