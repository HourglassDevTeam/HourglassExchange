use serde::{Deserialize, Serialize};

pub mod event;
pub mod historical;
pub mod live;

pub trait MarketFeedDistributor<Event>
{
    fn fetch_next(&mut self) -> Feed<Event>;
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum Feed<Event>
{
    Next(Event),
    Unhealthy,
    Finished,
}
