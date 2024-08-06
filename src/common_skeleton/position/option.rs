use serde::{Deserialize, Serialize};

use crate::common_skeleton::position::position_meta::PositionMeta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OptionPosition
{
    pub meta: PositionMeta,
}
