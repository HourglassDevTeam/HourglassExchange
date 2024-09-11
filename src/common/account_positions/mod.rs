use crate::{
    common::{
        account_positions::{
            future::{FuturePosition, FuturePositionConfig},
            leveraged_token::{LeveragedTokenPosition, LeveragedTokenPositionConfig},
            option::{OptionPosition, OptionPositionConfig},
            perpetual::{PerpetualPosition, PerpetualPositionBuilder, PerpetualPositionConfig},
            position_id::PositionId,
            position_meta::PositionMetaBuilder,
        },
        balance::{Balance, TokenBalance},
        instrument::{kind::InstrumentKind, Instrument},
        trade::ClientTrade,
        Side,
    },
    error::ExchangeError,
    sandbox::{account::account_config::AccountConfig, config_request::ConfigurationRequest},
    Exchange,
};
use chrono::Utc;
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, hash::Hash, sync::Arc};
use tokio::sync::RwLock;

mod exited_position;
pub mod exited_positions;
pub mod future;
pub(crate) mod leveraged_token;
pub(crate) mod option;
pub(crate) mod perpetual;
mod position_delta;
pub(crate) mod position_id;
pub mod position_meta;

#[derive(Clone, Debug)]
pub struct AccountPositions
{
    pub margin_pos_long: Arc<RwLock<HashMap<Instrument, LeveragedTokenPosition>>>,
    pub margin_pos_short: Arc<RwLock<HashMap<Instrument, LeveragedTokenPosition>>>,
    pub perpetual_pos_long: Arc<RwLock<HashMap<Instrument, PerpetualPosition>>>,
    pub perpetual_pos_short: Arc<RwLock<HashMap<Instrument, PerpetualPosition>>>,
    pub futures_pos_long: Arc<RwLock<HashMap<Instrument, FuturePosition>>>,
    pub futures_pos_short: Arc<RwLock<HashMap<Instrument, FuturePosition>>>,
    pub option_pos_long_call: Arc<RwLock<HashMap<Instrument, OptionPosition>>>,
    pub option_pos_long_put: Arc<RwLock<HashMap<Instrument, OptionPosition>>>,
    pub option_pos_short_call: Arc<RwLock<HashMap<Instrument, OptionPosition>>>,
    pub option_pos_short_put: Arc<RwLock<HashMap<Instrument, OptionPosition>>>,
    pub margin_pos_long_config: Arc<RwLock<HashMap<Instrument, LeveragedTokenPositionConfig>>>,
    pub margin_pos_short_config: Arc<RwLock<HashMap<Instrument, LeveragedTokenPositionConfig>>>,
    pub perpetual_pos_long_config: Arc<RwLock<HashMap<Instrument, PerpetualPositionConfig>>>,
    pub perpetual_pos_short_config: Arc<RwLock<HashMap<Instrument, PerpetualPositionConfig>>>,
    pub futures_pos_long_config: Arc<RwLock<HashMap<Instrument, FuturePositionConfig>>>,
    pub futures_pos_short_config: Arc<RwLock<HashMap<Instrument, FuturePositionConfig>>>,
    pub option_pos_long_call_config: Arc<RwLock<HashMap<Instrument, OptionPositionConfig>>>,
    pub option_pos_long_put_config: Arc<RwLock<HashMap<Instrument, OptionPositionConfig>>>,
    pub option_pos_short_call_config: Arc<RwLock<HashMap<Instrument, OptionPositionConfig>>>,
    pub option_pos_short_put_config: Arc<RwLock<HashMap<Instrument, OptionPositionConfig>>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum PositionConfig
{
    Perpetual(PerpetualPositionConfig),
    Future(FuturePositionConfig),
    LeveragedToken(LeveragedTokenPositionConfig),
    Option(OptionPositionConfig),
}

impl From<ConfigurationRequest> for PositionConfig
{
    fn from(config_request: ConfigurationRequest) -> Self
    {
        match config_request.instrument.kind {
            | InstrumentKind::Perpetual => PositionConfig::Perpetual(PerpetualPositionConfig::from(config_request)),
            | InstrumentKind::Future => PositionConfig::Future(FuturePositionConfig::from(config_request)),
            | InstrumentKind::CryptoLeveragedToken => PositionConfig::LeveragedToken(LeveragedTokenPositionConfig::from(config_request)),
            | InstrumentKind::CryptoOption => PositionConfig::Option(OptionPositionConfig::from(config_request)),
            | _ => panic!("Unsupported instrument kind"), // 根据需求处理其他类型
        }
    }
}

impl Serialize for AccountPositions
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        // Helper function to convert Arc<RwLock<HashMap<K, V>>> to HashMap<K, V>
        fn to_map<K, V>(positions: &Arc<RwLock<HashMap<K, V>>>) -> HashMap<K, V>
            where K: Clone + Eq + Hash,
                  V: Clone
        {
            let positions_read = positions.blocking_read();
            positions_read.clone()
        }

        // Serialize all fields
        let mut state = serializer.serialize_struct("AccountPositions", 10)?;
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

// Manually implement PartialEq for AccountPositions
impl PartialEq for AccountPositions
{
    fn eq(&self, other: &Self) -> bool
    {
        fn hashmap_eq<K, V>(a: &Arc<RwLock<HashMap<K, V>>>, b: &Arc<RwLock<HashMap<K, V>>>) -> bool
            where K: Eq + Hash + Clone,
                  V: PartialEq + Clone
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

impl<'de> Deserialize<'de> for AccountPositions
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        struct AccountPositionsData
        {
            margin_pos_long: HashMap<Instrument, LeveragedTokenPosition>,
            margin_pos_short: HashMap<Instrument, LeveragedTokenPosition>,
            perpetual_pos_long: HashMap<Instrument, PerpetualPosition>,
            perpetual_pos_short: HashMap<Instrument, PerpetualPosition>,
            futures_pos_long: HashMap<Instrument, FuturePosition>,
            futures_pos_short: HashMap<Instrument, FuturePosition>,
            option_pos_long_call: HashMap<Instrument, OptionPosition>,
            option_pos_long_put: HashMap<Instrument, OptionPosition>,
            option_pos_short_call: HashMap<Instrument, OptionPosition>,
            option_pos_short_put: HashMap<Instrument, OptionPosition>,
            margin_pos_long_config: HashMap<Instrument, LeveragedTokenPositionConfig>,
            margin_pos_short_config: HashMap<Instrument, LeveragedTokenPositionConfig>,
            perpetual_pos_long_config: HashMap<Instrument, PerpetualPositionConfig>,
            perpetual_pos_short_config: HashMap<Instrument, PerpetualPositionConfig>,
            futures_pos_long_config: HashMap<Instrument, FuturePositionConfig>,
            futures_pos_short_config: HashMap<Instrument, FuturePositionConfig>,
            option_pos_long_call_config: HashMap<Instrument, OptionPositionConfig>,
            option_pos_long_put_config: HashMap<Instrument, OptionPositionConfig>,
            option_pos_short_call_config: HashMap<Instrument, OptionPositionConfig>,
            option_pos_short_put_config: HashMap<Instrument, OptionPositionConfig>,
        }

        let data = AccountPositionsData::deserialize(deserializer)?;

        Ok(AccountPositions { margin_pos_long: Arc::new(RwLock::new(data.margin_pos_long)),
                              margin_pos_short: Arc::new(RwLock::new(data.margin_pos_short)),
                              perpetual_pos_long: Arc::new(RwLock::new(data.perpetual_pos_long)),
                              perpetual_pos_short: Arc::new(RwLock::new(data.perpetual_pos_short)),
                              futures_pos_long: Arc::new(RwLock::new(data.futures_pos_long)),
                              futures_pos_short: Arc::new(RwLock::new(data.futures_pos_short)),
                              option_pos_long_call: Arc::new(RwLock::new(data.option_pos_long_call)),
                              option_pos_long_put: Arc::new(RwLock::new(data.option_pos_long_put)),
                              option_pos_short_call: Arc::new(RwLock::new(data.option_pos_short_call)),
                              option_pos_short_put: Arc::new(RwLock::new(data.option_pos_short_put)),
                              margin_pos_long_config: Arc::new(RwLock::new(data.margin_pos_long_config)),
                              margin_pos_short_config: Arc::new(RwLock::new(data.margin_pos_short_config)),
                              perpetual_pos_long_config: Arc::new(RwLock::new(data.perpetual_pos_long_config)),
                              perpetual_pos_short_config: Arc::new(RwLock::new(data.perpetual_pos_short_config)),
                              futures_pos_long_config: Arc::new(RwLock::new(data.futures_pos_long_config)),
                              futures_pos_short_config: Arc::new(RwLock::new(data.futures_pos_short_config)),
                              option_pos_long_call_config: Arc::new(RwLock::new(data.option_pos_long_call_config)),
                              option_pos_long_put_config: Arc::new(RwLock::new(data.option_pos_long_put_config)),
                              option_pos_short_put_config: Arc::new(RwLock::new(data.option_pos_short_put_config)),
                              option_pos_short_call_config: Arc::new(RwLock::new(data.option_pos_short_call_config)) })
    }
}

impl AccountPositions
{
    /// 创建一个新的 `AccountPositions` 实例
    pub fn init() -> Self
    {
        Self { margin_pos_long: Arc::new(RwLock::new(HashMap::new())),
               margin_pos_short: Arc::new(RwLock::new(HashMap::new())),
               perpetual_pos_long: Arc::new(RwLock::new(HashMap::new())),
               perpetual_pos_short: Arc::new(RwLock::new(HashMap::new())),
               futures_pos_long: Arc::new(RwLock::new(HashMap::new())),
               futures_pos_short: Arc::new(RwLock::new(HashMap::new())),
               option_pos_long_call: Arc::new(RwLock::new(HashMap::new())),
               option_pos_long_put: Arc::new(RwLock::new(HashMap::new())),
               option_pos_short_call: Arc::new(RwLock::new(HashMap::new())),
               option_pos_short_put: Arc::new(RwLock::new(HashMap::new())),
               margin_pos_long_config: Arc::new(RwLock::new(HashMap::new())),
               margin_pos_short_config: Arc::new(RwLock::new(HashMap::new())),
               perpetual_pos_long_config: Arc::new(RwLock::new(HashMap::new())),
               perpetual_pos_short_config: Arc::new(RwLock::new(HashMap::new())),
               futures_pos_long_config: Arc::new(RwLock::new(HashMap::new())),
               futures_pos_short_config: Arc::new(RwLock::new(HashMap::new())),
               option_pos_long_call_config: Arc::new(RwLock::new(HashMap::new())),
               option_pos_long_put_config: Arc::new(RwLock::new(HashMap::new())),
               option_pos_short_call_config: Arc::new(RwLock::new(HashMap::new())),
               option_pos_short_put_config: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// TODO check init logic
    pub async fn build_new_perpetual_position(&self,
                                              config: &AccountConfig,
                                              trade: &ClientTrade,
                                              exchange_ts: i64,
                                              mode: PositionMarginMode)
                                              -> Result<PerpetualPosition, ExchangeError>
    {
        let position_mode = config.global_position_direction_mode.clone();
        // 计算初始保证金
        let initial_margin = trade.price * trade.size / config.global_leverage_rate;

        // 根据 Instrument 和 Side 动态生成 position_id
        let position_meta = PositionMetaBuilder::new().position_id(PositionId::new(&trade.instrument.clone(), trade.timestamp))
                                                      .enter_ts(exchange_ts)
                                                      .update_ts(exchange_ts)
                                                      .exit_balance(TokenBalance { // 初始化为 exit_balance
                                                                                   token: trade.instrument.base.clone(),
                                                                                   balance: Balance { time: Utc::now(),
                                                                                                      current_price: None,
                                                                                                      total: trade.size,
                                                                                                      available: trade.size } })
                                                      .exchange(Exchange::SandBox)
                                                      .instrument(trade.instrument.clone())
                                                      .side(trade.side)
                                                      .current_size(trade.size)
                                                      .current_fees_total(trade.fees)
                                                      .current_avg_price_gross(trade.price)
                                                      .current_symbol_price(trade.price)
                                                      .current_avg_price(trade.price)
                                                      .unrealised_pnl(0.0) // 初始化为 0.0
                                                      .realised_pnl(0.0) // 初始化为 0.0
                                                      .build()
                                                      .map_err(|err| ExchangeError::SandBox(format!("Failed to build account_positions meta: {}", err)))?;

        // 计算 liquidation_price
        let liquidation_price = if trade.side == Side::Buy {
            trade.price * (1.0 - initial_margin / (trade.size * trade.price))
        }
        else {
            trade.price * (1.0 + initial_margin / (trade.size * trade.price))
        };
        let pos_config = PerpetualPositionConfig { pos_margin_mode: mode,
                                                   leverage: config.global_leverage_rate,
                                                   position_mode };

        let new_position = PerpetualPositionBuilder::new().meta(position_meta)
                                                          .pos_config(pos_config)
                                                          .liquidation_price(liquidation_price)
                                                          .build()
                                                          .ok_or_else(|| ExchangeError::SandBox("Failed to build new account_positions".to_string()))?;

        Ok(new_position)
    }

    pub async fn update_position(&self, new_position: Position)
    {
        match new_position {
            | Position::Perpetual(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &self.perpetual_pos_long;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &self.perpetual_pos_short;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
            },
            | Position::LeveragedToken(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &self.margin_pos_long;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &self.margin_pos_short;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
            },
            | Position::Future(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &self.futures_pos_long;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &self.futures_pos_short;
                    let mut positions_lock = positions.write().await;
                    if let Some(existing_position) = positions_lock.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions_lock.insert(p.meta.instrument.clone(), p);
                    }
                }
            },
            | Position::Option(_p) => {
                todo!()
            }
        }
    }

    /// 检查账户中是否持有指定交易工具的多头仓位
    pub(crate) async fn has_long_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            | InstrumentKind::Spot => todo!("[UniLinkEx] : The system does not support creation or processing of positions of Spot as of yet."),
            | InstrumentKind::CommodityOption => todo!("[UniLinkEx] : The system does not support creation or processing of positions of CommodityOption as of yet."),
            | InstrumentKind::CommodityFuture => todo!("[UniLinkEx] : The system does not support creation or processing of positions of CommodityFuture as of yet."),
            | InstrumentKind::Perpetual => {
                let positions = self.perpetual_pos_long.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::Future => {
                let positions = self.futures_pos_long.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::CryptoOption => {
                let positions = self.option_pos_long_call.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::CryptoLeveragedToken => {
                let positions = self.margin_pos_long.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
        }
    }

    /// 检查账户中是否持有指定交易工具的空头仓位
    pub(crate) async fn has_short_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            | InstrumentKind::Spot => todo!("[UniLinkEx] : The system does not support creation or processing of positions of Spot as of yet."),
            | InstrumentKind::CommodityOption => todo!("[UniLinkEx] : The system does not support creation or processing of positions of CommodityOption as of yet."),
            | InstrumentKind::CommodityFuture => todo!("[UniLinkEx] : The system does not support creation or processing of positions of CommodityFuture as of yet."),
            | InstrumentKind::Perpetual => {
                let positions = self.perpetual_pos_short.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::Future => {
                let positions = self.futures_pos_short.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::CryptoOption => {
                let positions = self.option_pos_short_put.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
            | InstrumentKind::CryptoLeveragedToken => {
                let positions = self.margin_pos_short.read().await;
                positions.iter().any(|(key, _)| key == instrument)
            }
        }
    }
}

///  [NetMode] : 单向模式。在这种模式下，用户只能持有一个方向的仓位（多头或空头），而不能同时持有两个方向的仓位。
///  [LongShortMode] : 双向模式。在这种模式下，用户可以同时持有多头和空头仓位。这在一些复杂的交易策略中可能会有用，例如对冲策略。
#[derive(Clone, PartialOrd, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionDirectionMode
{
    LongShort,
    Net,
}

#[derive(Clone, PartialOrd, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionMarginMode
{
    Cross,
    Isolated,
}

/// NOTE: 可能需要多种头寸类型共存
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Position
{
    Perpetual(PerpetualPosition),
    LeveragedToken(LeveragedTokenPosition),
    Future(FuturePosition),
    Option(OptionPosition),
}
