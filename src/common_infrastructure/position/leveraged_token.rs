use serde::{Deserialize, Serialize};

use crate::common_infrastructure::position::position_meta::PositionMeta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LeveragedTokenPosition
{
    pub meta: PositionMeta,
}
