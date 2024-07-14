use crate::common_skeleton::data::event::MarketEvent;
use serde::Serialize;
use uuid::Uuid;

// 鉴于Data的种类可能会很多，规避避开enum的开销和维护成本，使用泛型来定义AccountFeedData类型。
#[derive(Clone, Debug, Serialize)]
pub struct AccountDataFeed<Data>
{
    #[serde(alias = "counter", alias = "batch_counter")]
    pub batch_id: Uuid,
    pub data: Vec<MarketEvent<Data>>,
}
