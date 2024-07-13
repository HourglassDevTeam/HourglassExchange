use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub mod historical;
pub mod live;
pub mod event;


pub trait FeedGenerator<Event>
{
    fn next(&mut self) -> Feed<Event>;
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum Feed<Event>
{
    Next(Event),
    Unhealthy,
    Finished,
}

