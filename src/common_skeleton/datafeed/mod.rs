use serde::{Deserialize, Serialize};

pub mod event;
pub mod historical;
pub mod live;

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
