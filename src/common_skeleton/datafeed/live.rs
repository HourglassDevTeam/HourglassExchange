use std::pin::Pin;

use futures::Stream;

use crate::error::ExecutionError;

pub struct LiveFeed<Event>
{
    stream: Pin<Box<dyn Stream<Item=Result<Event, ExecutionError>> + Send>>,
}

impl<Event> LiveFeed<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item=Result<Event, ExecutionError>> + Send)>
    {
        self.stream.as_mut()
    }
}
