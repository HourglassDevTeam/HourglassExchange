use crate::common::{
    account_positions::{
        future::FuturePosition,
        leveraged_token::LeveragedTokenPosition,
        option::OptionPosition,
        perpetual::PerpetualPosition

        ,
    }
    ,
};
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::common::account_positions::position_id::PositionId;


/// NOTE
///     鉴于平仓的仓位不再有唯一性，要使用PositionId作为Key
///
#[derive(Clone, Debug)]
pub struct AccountExitedPositions

{
    pub margin_pos_long: Arc<RwLock<HashMap<PositionId, LeveragedTokenPosition>>>,
    pub margin_pos_short: Arc<RwLock<HashMap<PositionId, LeveragedTokenPosition>>>,
    pub perpetual_pos_long: Arc<RwLock<HashMap<PositionId, PerpetualPosition>>>,
    pub perpetual_pos_short: Arc<RwLock<HashMap<PositionId, PerpetualPosition>>>,
    pub futures_pos_long: Arc<RwLock<HashMap<PositionId, FuturePosition>>>,
    pub futures_pos_short: Arc<RwLock<HashMap<PositionId, FuturePosition>>>,
    pub option_pos_long_call: Arc<RwLock<HashMap<PositionId, OptionPosition>>>,
    pub option_pos_long_put: Arc<RwLock<HashMap<PositionId, OptionPosition>>>,
    pub option_pos_short_call: Arc<RwLock<HashMap<PositionId, OptionPosition>>>,
    pub option_pos_short_put: Arc<RwLock<HashMap<PositionId, OptionPosition>>>,
}


#[allow(dead_code)]
impl AccountExitedPositions {
    pub fn init() -> Self {
        Self {
            margin_pos_long: Arc::new(RwLock::new(HashMap::new())),
            margin_pos_short: Arc::new(RwLock::new(HashMap::new())),
            perpetual_pos_long: Arc::new(RwLock::new(HashMap::new())),
            perpetual_pos_short: Arc::new(RwLock::new(HashMap::new())),
            futures_pos_long: Arc::new(RwLock::new(HashMap::new())),
            futures_pos_short: Arc::new(RwLock::new(HashMap::new())),
            option_pos_long_call: Arc::new(RwLock::new(HashMap::new())),
            option_pos_long_put: Arc::new(RwLock::new(HashMap::new())),
            option_pos_short_call: Arc::new(RwLock::new(HashMap::new())),
            option_pos_short_put: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    /// 插入方法，推断 `PositionId` 并插入 `LeveragedTokenPosition` 到 `margin_pos_long`
    pub async fn insert_margin_pos_long(&self, position: LeveragedTokenPosition) {
        let position_id = position.meta.position_id.clone(); // 从 position 中推断出 position_id
        let mut pos_long = self.margin_pos_long.write().await;
        pos_long.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `LeveragedTokenPosition` 到 `margin_pos_short`
    pub async fn insert_margin_pos_short(&self, position: LeveragedTokenPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_short = self.margin_pos_short.write().await;
        pos_short.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `PerpetualPosition` 到 `perpetual_pos_long`
    pub async fn insert_perpetual_pos_long(&self, position: PerpetualPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_long = self.perpetual_pos_long.write().await;
        pos_long.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `PerpetualPosition` 到 `perpetual_pos_short`
    pub async fn insert_perpetual_pos_short(&self, position: PerpetualPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_short = self.perpetual_pos_short.write().await;
        pos_short.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `FuturePosition` 到 `futures_pos_long`
    pub async fn insert_futures_pos_long(&self, position: FuturePosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_long = self.futures_pos_long.write().await;
        pos_long.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `FuturePosition` 到 `futures_pos_short`
    pub async fn insert_futures_pos_short(&self, position: FuturePosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_short = self.futures_pos_short.write().await;
        pos_short.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `OptionPosition` 到 `option_pos_long_call`
    pub async fn insert_option_pos_long_call(&self, position: OptionPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_call = self.option_pos_long_call.write().await;
        pos_call.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `OptionPosition` 到 `option_pos_long_put`
    pub async fn insert_option_pos_long_put(&self, position: OptionPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_put = self.option_pos_long_put.write().await;
        pos_put.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `OptionPosition` 到 `option_pos_short_call`
    pub async fn insert_option_pos_short_call(&self, position: OptionPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_call = self.option_pos_short_call.write().await;
        pos_call.insert(position_id, position);
    }

    /// 插入方法，推断 `PositionId` 并插入 `OptionPosition` 到 `option_pos_short_put`
    pub async fn insert_option_pos_short_put(&self, position: OptionPosition) {
        let position_id = position.meta.
            position_id.clone(); // 推断 position_id
        let mut pos_put = self.option_pos_short_put.write().await;
        pos_put.insert(position_id, position);
    }

    /// 重置方法：清空所有持仓数据
    pub async fn reset_positions(&self) {
        self.margin_pos_long.write().await.clear();
        self.margin_pos_short.write().await.clear();
        self.perpetual_pos_long.write().await.clear();
        self.perpetual_pos_short.write().await.clear();
        self.futures_pos_long.write().await.clear();
        self.futures_pos_short.write().await.clear();
        self.option_pos_long_call.write().await.clear();
        self.option_pos_long_put.write().await.clear();
        self.option_pos_short_call.write().await.clear();
        self.option_pos_short_put.write().await.clear();
    }
}

impl Serialize for AccountExitedPositions
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Helper function to convert Arc<RwLock<HashMap<K, V>>> to HashMap<K, V>
        fn to_map<K, V>(positions: &Arc<RwLock<HashMap<K, V>>>) -> HashMap<K, V>
        where
            K: Clone + Eq + Hash,
            V: Clone,
        {
            let positions_read = positions.blocking_read();
            positions_read.clone()
        }

        // Serialize all fields
        let mut state = serializer.serialize_struct("ClosedPositions
", 10)?;
        state.serialize_field("margin_pos_long", &to_map(&self.margin_pos_long))?;
        state.serialize_field("margin_pos_short", &to_map(&self.margin_pos_short))?;
        state.serialize_field("perpetual_pos_long", &to_map(&self.perpetual_pos_long))?;
        state.serialize_field("perpetual_pos_short", &to_map(&self.perpetual_pos_short))?;
        state.serialize_field("futures_pos_long", &to_map(&self.futures_pos_long))?;
        state.serialize_field("futures_pos_short", &to_map(&self.futures_pos_short))?;
        state.serialize_field("option_pos_long_call", &to_map(&self.option_pos_long_call))?;
        state.serialize_field("option_pos_long_put", &to_map(&self.option_pos_long_put))?;
        state.serialize_field("option_pos_short_call", &to_map(&self.option_pos_short_call))?;
        state.serialize_field("option_pos_short_put", &to_map(&self.option_pos_short_put))?;
        state.end()
    }
}

// Manually implement PartialEq for ClosedPositions

impl PartialEq for AccountExitedPositions
{
    fn eq(&self, other: &Self) -> bool {
        fn hashmap_eq<K, V>(a: &Arc<RwLock<HashMap<K, V>>>, b: &Arc<RwLock<HashMap<K, V>>>) -> bool
        where
            K: Eq + Hash + Clone,
            V: PartialEq + Clone,
        {
            let a_read = a.blocking_read();
            let b_read = b.blocking_read();

            let a_map: HashMap<K, V> = a_read.clone();
            let b_map: HashMap<K, V> = b_read.clone();

            a_map == b_map
        }

        hashmap_eq(&self.margin_pos_long, &other.margin_pos_long)
            && hashmap_eq(&self.margin_pos_short, &other.margin_pos_short)
            && hashmap_eq(&self.perpetual_pos_long, &other.perpetual_pos_long)
            && hashmap_eq(&self.perpetual_pos_short, &other.perpetual_pos_short)
            && hashmap_eq(&self.futures_pos_long, &other.futures_pos_long)
            && hashmap_eq(&self.futures_pos_short, &other.futures_pos_short)
            && hashmap_eq(&self.option_pos_long_call, &other.option_pos_long_call)
            && hashmap_eq(&self.option_pos_long_put, &other.option_pos_long_put)
            && hashmap_eq(&self.option_pos_short_call, &other.option_pos_short_call)
            && hashmap_eq(&self.option_pos_short_put, &other.option_pos_short_put)
    }
}

impl<'de> Deserialize<'de> for AccountExitedPositions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ClosedPositionsData {
            margin_pos_long: HashMap<PositionId, LeveragedTokenPosition>,
            margin_pos_short: HashMap<PositionId, LeveragedTokenPosition>,
            perpetual_pos_long: HashMap<PositionId, PerpetualPosition>,
            perpetual_pos_short: HashMap<PositionId, PerpetualPosition>,
            futures_pos_long: HashMap<PositionId, FuturePosition>,
            futures_pos_short: HashMap<PositionId, FuturePosition>,
            option_pos_long_call: HashMap<PositionId, OptionPosition>,
            option_pos_long_put: HashMap<PositionId, OptionPosition>,
            option_pos_short_call: HashMap<PositionId, OptionPosition>,
            option_pos_short_put: HashMap<PositionId, OptionPosition>,
        }

        let data = ClosedPositionsData::deserialize(deserializer)?;

        Ok(AccountExitedPositions {
            margin_pos_long: Arc::new(RwLock::new(data.margin_pos_long)),
            margin_pos_short: Arc::new(RwLock::new(data.margin_pos_short)),
            perpetual_pos_long: Arc::new(RwLock::new(data.perpetual_pos_long)),
            perpetual_pos_short: Arc::new(RwLock::new(data.perpetual_pos_short)),
            futures_pos_long: Arc::new(RwLock::new(data.futures_pos_long)),
            futures_pos_short: Arc::new(RwLock::new(data.futures_pos_short)),
            option_pos_long_call: Arc::new(RwLock::new(data.option_pos_long_call)),
            option_pos_long_put: Arc::new(RwLock::new(data.option_pos_long_put)),
            option_pos_short_call: Arc::new(RwLock::new(data.option_pos_short_call)),
            option_pos_short_put: Arc::new(RwLock::new(data.option_pos_short_put)),
        })
    }
}