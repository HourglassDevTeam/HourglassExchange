use serde::{Deserialize, Serialize};
/// FIXME : THIS GENERATION LOGIC IS IN NEED OF RECONFIRMATION.
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize, PartialOrd)]
pub struct PositionId(pub u64);

impl Display for PositionId
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

impl PositionId
{
    /// Generates a new `PositionId` using a variant of the snowflake algorithm.
    ///
    /// # Parameters
    ///
    /// - `timestamp`: The timestamp used for ID generation.
    /// - `machine_id`: The ID of the machine generating the ID, max value is 1023.
    /// - `counter`: The current counter value.
    ///
    /// # Returns
    ///
    /// Returns a unique `PositionId`.
    pub fn new(timestamp: u64, machine_id: u64, counter: u64) -> Self
    {
        let id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);
        PositionId(id)
    }

    /// Returns the internal `u64` value of the `PositionId`.
    pub fn value(&self) -> u64
    {
        self.0
    }
}
