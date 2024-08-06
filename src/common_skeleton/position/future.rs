use serde::{Deserialize, Serialize};
use crate::common_skeleton::position::PositionMeta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FuturesPosition {
    pub meta: PositionMeta,
}
