use crate::{
    common::{
        balance::{Balance, TokenBalance},
        friction::{Fees, PerpetualFees},
        instrument::{kind::InstrumentKind, Instrument},
        position::{
            future::FuturePosition,
            leveraged_token::LeveragedTokenPosition,
            option::OptionPosition,
            perpetual::{PerpetualPosition, PerpetualPositionBuilder, PerpetualPositionConfig},
            position_id::PositionId,
            position_meta::PositionMetaBuilder,
        },
        trade::ClientTrade,
        Side,
    },
    error::ExchangeError,
    sandbox::account::account_config::AccountConfig,
    Exchange,
};
use chrono::Utc;
use dashmap::DashMap;
use serde::ser::SerializeStruct;
/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

pub mod future;
pub(crate) mod leveraged_token;
pub(crate) mod option;
pub mod perpetual;
pub mod position_id;
pub mod position_meta;

#[derive(Clone, Debug)]
pub struct AccountPositions
{
    pub margin_pos_long: DashMap<Instrument, LeveragedTokenPosition>, // NOTE useless in backtest
    pub margin_pos_short: DashMap<Instrument, LeveragedTokenPosition>,
    pub perpetual_pos_long: DashMap<Instrument, PerpetualPosition>,
    pub perpetual_pos_short: DashMap<Instrument, PerpetualPosition>,
    pub futures_pos_long: DashMap<Instrument, FuturePosition>,
    pub futures_pos_short: DashMap<Instrument, FuturePosition>,
    pub option_pos_long_call: DashMap<Instrument, OptionPosition>,
    pub option_pos_long_put: DashMap<Instrument, OptionPosition>,
    pub option_pos_short_call: DashMap<Instrument, OptionPosition>,
    pub option_pos_short_put: DashMap<Instrument, OptionPosition>,
}

impl Serialize for AccountPositions
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let margin_pos_long: HashMap<_, _> = self.margin_pos_long.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let margin_pos_short: HashMap<_, _> = self.margin_pos_short.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let perpetual_pos_long: HashMap<_, _> = self.perpetual_pos_long.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let perpetual_pos_short: HashMap<_, _> = self.perpetual_pos_short.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let futures_pos_long: HashMap<_, _> = self.futures_pos_long.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let futures_pos_short: HashMap<_, _> = self.futures_pos_short.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let option_pos_long_call: HashMap<_, _> = self.option_pos_long_call.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let option_pos_long_put: HashMap<_, _> = self.option_pos_long_put.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let option_pos_short_call: HashMap<_, _> = self.option_pos_short_call.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let option_pos_short_put: HashMap<_, _> = self.option_pos_short_put.iter().map(|r| (r.key().clone(), r.value().clone())).collect();

        let mut state = serializer.serialize_struct("AccountPositions", 10)?;
        state.serialize_field("margin_pos_long", &margin_pos_long)?;
        state.serialize_field("margin_pos_short", &margin_pos_short)?;
        state.serialize_field("perpetual_pos_long", &perpetual_pos_long)?;
        state.serialize_field("perpetual_pos_short", &perpetual_pos_short)?;
        state.serialize_field("futures_pos_long", &futures_pos_long)?;
        state.serialize_field("futures_pos_short", &futures_pos_short)?;
        state.serialize_field("option_pos_long_call", &option_pos_long_call)?;
        state.serialize_field("option_pos_long_put", &option_pos_long_put)?;
        state.serialize_field("option_pos_short_call", &option_pos_short_call)?;
        state.serialize_field("option_pos_short_put", &option_pos_short_put)?;
        state.end()
    }
}
// Manually implement PartialEq for AccountPositions
impl PartialEq for AccountPositions
{
    fn eq(&self, other: &Self) -> bool
    {
        fn dashmap_eq<K, V>(a: &DashMap<K, V>, b: &DashMap<K, V>) -> bool
            where K: Eq + std::hash::Hash + Clone,
                  V: PartialEq + Clone
        {
            let a_map: HashMap<K, V> = a.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect();
            let b_map: HashMap<K, V> = b.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect();
            a_map == b_map
        }

        dashmap_eq(&self.margin_pos_long, &other.margin_pos_long)
        && dashmap_eq(&self.margin_pos_short, &other.margin_pos_short)
        && dashmap_eq(&self.perpetual_pos_long, &other.perpetual_pos_long)
        && dashmap_eq(&self.perpetual_pos_short, &other.perpetual_pos_short)
        && dashmap_eq(&self.futures_pos_long, &other.futures_pos_long)
        && dashmap_eq(&self.futures_pos_short, &other.futures_pos_short)
        && dashmap_eq(&self.option_pos_long_call, &other.option_pos_long_call)
        && dashmap_eq(&self.option_pos_long_put, &other.option_pos_long_put)
        && dashmap_eq(&self.option_pos_short_call, &other.option_pos_short_call)
        && dashmap_eq(&self.option_pos_short_put, &other.option_pos_short_put)
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
        }

        let data = AccountPositionsData::deserialize(deserializer)?;

        Ok(AccountPositions { margin_pos_long: DashMap::from_iter(data.margin_pos_long),
                              margin_pos_short: DashMap::from_iter(data.margin_pos_short),
                              perpetual_pos_long: DashMap::from_iter(data.perpetual_pos_long),
                              perpetual_pos_short: DashMap::from_iter(data.perpetual_pos_short),
                              futures_pos_long: DashMap::from_iter(data.futures_pos_long),
                              futures_pos_short: DashMap::from_iter(data.futures_pos_short),
                              option_pos_long_call: DashMap::from_iter(data.option_pos_long_call),
                              option_pos_long_put: DashMap::from_iter(data.option_pos_long_put),
                              option_pos_short_call: DashMap::from_iter(data.option_pos_short_call),
                              option_pos_short_put: DashMap::from_iter(data.option_pos_short_put) })
    }
}

impl AccountPositions
{
    /// 创建一个新的 `AccountPositions` 实例
    pub fn init() -> Self
    {
        Self { margin_pos_long: DashMap::new(),
               margin_pos_short: DashMap::new(),
               perpetual_pos_long: DashMap::new(),
               perpetual_pos_short: DashMap::new(),
               futures_pos_long: DashMap::new(),
               futures_pos_short: DashMap::new(),
               option_pos_long_call: DashMap::new(),
               option_pos_long_put: DashMap::new(),
               option_pos_short_call: DashMap::new(),
               option_pos_short_put: DashMap::new() }
    }

    /// TODO check init logic
    pub async fn build_new_perpetual_position(&self, config: &AccountConfig, trade: &ClientTrade, exchange_ts: i64) -> Result<PerpetualPosition, ExchangeError>
    {
        let maker_rate = config.get_maker_fee_rate(&trade.instrument.kind)?;
        let taker_rate = config.get_taker_fee_rate(&trade.instrument.kind)?;
        let position_mode = config.position_direction_mode.clone();
        let position_margin_mode = config.position_margin_mode.clone();
        // 计算初始保证金
        let initial_margin = trade.price * trade.quantity / config.account_leverage_rate;
        // 计算费用
        let maker_fee = trade.quantity * trade.price * maker_rate;
        let taker_fee = trade.quantity * trade.price * taker_rate;
        let funding_fee = trade.quantity * trade.price * config.funding_rate;

        // 根据 Instrument 和 Side 动态生成 position_id
        let position_meta = PositionMetaBuilder::new().position_id(PositionId::new(&trade.instrument.clone(), trade.timestamp))
                                                      .enter_ts(exchange_ts)
                                                      .update_ts(exchange_ts)
                                                      .exit_balance(TokenBalance { // 初始化为 exit_balance
                                                                                   token: trade.instrument.base.clone(),
                                                                                   balance: Balance { time: Utc::now(),
                                                                                                      current_price: trade.price,
                                                                                                      total: trade.quantity,
                                                                                                      available: trade.quantity } })
                                                      .exchange(Exchange::SandBox)
                                                      .instrument(trade.instrument.clone())
                                                      .side(trade.side)
                                                      .current_size(trade.quantity)
                                                      .current_fees_total(Fees::Perpetual(PerpetualFees { maker_fee,
                                                                                                          taker_fee, // 假设平仓费率与开仓费率相同
                                                                                                          funding_fee }))
                                                      .current_avg_price_gross(trade.price)
                                                      .current_symbol_price(trade.price)
                                                      .current_avg_price(trade.price)
                                                      .unrealised_pnl(0.0) // 初始化为 0.0
                                                      .realised_pnl(0.0) // 初始化为 0.0
                                                      .build()
                                                      .map_err(|err| ExchangeError::SandBox(format!("Failed to build position meta: {}", err)))?;

        // 计算 liquidation_price
        let liquidation_price = if trade.side == Side::Buy {
            trade.price * (1.0 - initial_margin / (trade.quantity * trade.price))
        }
        else {
            trade.price * (1.0 + initial_margin / (trade.quantity * trade.price))
        };
        let pos_config = PerpetualPositionConfig { pos_margin_mode: position_margin_mode,
                                                   leverage: config.account_leverage_rate,
                                                   position_mode };

        let new_position = PerpetualPositionBuilder::new().meta(position_meta)
                                                          .pos_config(pos_config)
                                                          .liquidation_price(liquidation_price)
                                                          .margin(initial_margin) // NOTE DOUBLE CHECK
                                                          .build()
                                                          .ok_or_else(|| ExchangeError::SandBox("Failed to build new position".to_string()))?;

        Ok(new_position)
    }

    /// 更新或添加新的仓位
    pub fn update_position(&mut self, new_position: Position)
    {
        match new_position {
            | Position::Perpetual(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &mut self.perpetual_pos_long;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &mut self.perpetual_pos_short;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
            },

            | Position::LeveragedToken(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &mut self.margin_pos_long;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &mut self.margin_pos_short;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
            },

            | Position::Future(p) => match p.meta.side {
                | Side::Buy => {
                    let positions = &mut self.futures_pos_long;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
                | Side::Sell => {
                    let positions = &mut self.futures_pos_short;
                    if let Some(mut existing_position) = positions.get_mut(&p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.insert(p.meta.instrument.clone(), p);
                    }
                }
            },

            | Position::Option(_p) => {
                todo!()
            }
        }
    }

    /// 检查账户中是否持有指定交易工具的多头仓位
    pub(crate) fn has_long_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            | InstrumentKind::Spot => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of Spot as of yet."),
            | InstrumentKind::CommodityOption => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityOption as of yet."),
            | InstrumentKind::CommodityFuture => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityFuture as of yet."),
            | InstrumentKind::Perpetual => self.perpetual_pos_long
                                               .iter() // 迭代 DashMap
                                               .any(|entry| entry.key() == instrument),
            | InstrumentKind::Future => self.futures_pos_long
                                            .iter() // 迭代 DashMap
                                            .any(|entry| entry.key() == instrument),
            | InstrumentKind::CryptoOption => self.option_pos_long_call
                                                  .iter() // 迭代 DashMap
                                                  .any(|entry| entry.key() == instrument),
            | InstrumentKind::CryptoLeveragedToken => self.margin_pos_long
                                                          .iter() // 迭代 DashMap
                                                          .any(|entry| entry.key() == instrument),
        }
    }

    /// 检查账户中是否持有指定交易工具的空头仓位
    pub(crate) fn has_short_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            | InstrumentKind::Spot => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of Spot as of yet."),
            | InstrumentKind::CommodityOption => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityOption as of yet."),
            | InstrumentKind::CommodityFuture => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityFuture as of yet."),
            | InstrumentKind::Perpetual => self.perpetual_pos_short
                                               .iter() // 迭代 DashMap
                                               .any(|entry| entry.key() == instrument),
            | InstrumentKind::Future => self.futures_pos_short
                                            .iter() // 迭代 DashMap
                                            .any(|entry| entry.key() == instrument),
            | InstrumentKind::CryptoOption => self.option_pos_short_put
                                                  .iter() // 迭代 DashMap
                                                  .any(|entry| entry.key() == instrument),
            | InstrumentKind::CryptoLeveragedToken => self.margin_pos_short
                                                          .iter() // 迭代 DashMap
                                                          .any(|entry| entry.key() == instrument),
        }
    }
}

/// NOTE : PositionMode 枚举定义了两种交易方向模式：
///  [NetMode] : 单向模式。在这种模式下，用户只能持有一个方向的仓位（多头或空头），而不能同时持有两个方向的仓位。
///  [LongShortMode] : 双向模式。在这种模式下，用户可以同时持有多头和空头仓位。这在一些复杂的交易策略中可能会有用，例如对冲策略。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionDirectionMode
{
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

///  NOTE : PositionMarginMode has defined two modes of margin consumption.
///
///  [Cross]: 交叉保证金模式。在这种模式下，所有仓位共享一个保证金池，盈亏共用。如果仓位的保证金不足，将从账户余额中提取以补充不足。
///  [Isolated]: 逐仓保证金模式。在这种模式下，每个仓位都有独立的保证金，盈亏互不影响。如果某个仓位的保证金不足，该仓位将被强制平仓，而不会影响到其他仓位。

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::token::Token;

    fn create_instrument(kind: InstrumentKind) -> Instrument
    {
        Instrument { base: Token::from("BTC"),
                     quote: Token::from("USDT"),
                     kind }
    }

    fn create_perpetual_position(instrument: &Instrument) -> PerpetualPosition
    {
        // 假设初始交易价格和仓位大小
        let initial_trade_price = 50000.0;
        let trade_size = 1.0;

        // 设置杠杆率
        let leverage = 10.0;

        // 计算初始保证金
        let initial_margin = initial_trade_price * trade_size / leverage;

        // 假设当前市场价格略有波动
        let current_market_price = 50500.0;

        // 计算当前仓位的未实现盈亏
        let unrealised_pnl = (current_market_price - initial_trade_price) * trade_size;

        // 计算清算价格 (liquidation_price)
        let liquidation_price = initial_trade_price * (1.0 - initial_margin / (trade_size * initial_trade_price));

        PerpetualPosition { meta: PositionMetaBuilder::new().position_id(PositionId(124124123412412))
                                                            .instrument(instrument.clone())
                                                            .side(Side::Buy)
                                                            .enter_ts(1625097600000)
                                                            .update_ts(1625097600000)
                                                            .exit_balance(TokenBalance { token: instrument.base.clone(),
                                                                                         balance: Balance { time: Utc::now(),
                                                                                                            current_price: current_market_price,
                                                                                                            total: trade_size,
                                                                                                            available: trade_size } })
                                                            .exchange(Exchange::Binance)
                                                            .current_size(trade_size)
                                                            .current_fees_total(Fees::Perpetual(PerpetualFees { maker_fee: 0.1 * trade_size,
                                                                                                                taker_fee: 0.1 * trade_size,
                                                                                                                funding_fee: 0.0 }))
                                                            .current_avg_price_gross(initial_trade_price)
                                                            .current_symbol_price(current_market_price)
                                                            .current_avg_price(initial_trade_price)
                                                            .unrealised_pnl(unrealised_pnl)
                                                            .realised_pnl(0.0)
                                                            .build()
                                                            .unwrap(),
                            liquidation_price,
                            margin: initial_margin,
                            pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                                  leverage,
                                                                  position_mode: PositionDirectionMode::NetMode } }
    }

    #[test]
    fn test_has_position()
    {
        let mut account_positions = AccountPositions::init();

        let perpetual_instrument = create_instrument(InstrumentKind::Perpetual);
        let future_instrument = create_instrument(InstrumentKind::Future);

        // 初始情况下，没有任何仓位
        assert!(!account_positions.has_long_position(&perpetual_instrument));
        assert!(!account_positions.has_short_position(&perpetual_instrument));
        assert!(!account_positions.has_long_position(&future_instrument));
        assert!(!account_positions.has_short_position(&future_instrument));

        // 创建并添加 PerpetualPosition 多头仓位
        let mut perpetual_position = create_perpetual_position(&perpetual_instrument);
        perpetual_position.meta.side = Side::Buy; // 设置为多头仓位
        account_positions.update_position(Position::Perpetual(perpetual_position.clone()));

        // 现在应该持有 PerpetualPosition 多头仓位，但不持有 FuturePosition
        assert!(account_positions.has_long_position(&perpetual_instrument));
        assert!(!account_positions.has_short_position(&perpetual_instrument));
        assert!(!account_positions.has_long_position(&future_instrument));
        assert!(!account_positions.has_short_position(&future_instrument));

        // 创建并添加 PerpetualPosition 空头仓位
        perpetual_position.meta.side = Side::Sell; // 设置为空头仓位
        account_positions.update_position(Position::Perpetual(perpetual_position.clone()));

        // 现在应该持有 PerpetualPosition 的空头和多头仓位
        assert!(account_positions.has_long_position(&perpetual_instrument));
        assert!(account_positions.has_short_position(&perpetual_instrument));
    }
    #[test]
    fn test_update_existing_position()
    {
        let mut account_positions = AccountPositions::init();

        let perpetual_instrument = create_instrument(InstrumentKind::Perpetual);

        // 添加初始的 PerpetualPosition 多头仓位
        let mut perpetual_position = create_perpetual_position(&perpetual_instrument);
        perpetual_position.meta.side = Side::Buy; // 设置为多头仓位
        account_positions.update_position(Position::Perpetual(perpetual_position.clone()));

        // 确保初始 PerpetualPosition 已正确添加
        assert!(account_positions.has_long_position(&perpetual_instrument));
        assert_eq!(account_positions.perpetual_pos_long.len(), 1);

        // 更新相同的 PerpetualPosition，修改 `margin`
        let mut updated_position = perpetual_position.clone();
        updated_position.margin = 2000.0; // 修改仓位的保证金

        account_positions.update_position(Position::Perpetual(updated_position.clone()));

        // 确保仓位已更新而不是新添加
        if !account_positions.perpetual_pos_long.is_empty() {
            assert_eq!(account_positions.perpetual_pos_long.len(), 1); // 确保仓位数量未增加
            let pos = account_positions.perpetual_pos_long.get(&perpetual_instrument).unwrap();
            assert_eq!(pos.margin, 2000.0); // 检查仓位是否已正确更新
        }
        else {
            panic!("PerpetualPosition should exist but was not found.");
        }
    }
    #[test]
    fn test_add_new_position()
    {
        let mut account_positions = AccountPositions::init();

        // 创建两个不同的 Instrument
        let perpetual_instrument_1 = Instrument { base: Token::from("BTC"),
                                                  quote: Token::from("USDT"),
                                                  kind: InstrumentKind::Perpetual };

        let perpetual_instrument_2 = Instrument { base: Token::from("ETH"),
                                                  quote: Token::from("USDT"),
                                                  kind: InstrumentKind::Perpetual };

        // 添加初始的 PerpetualPosition (多头仓位)
        let mut perpetual_position_1 = create_perpetual_position(&perpetual_instrument_1);
        perpetual_position_1.meta.side = Side::Buy; // 设置为多头仓位
        account_positions.update_position(Position::Perpetual(perpetual_position_1.clone()));

        // 添加新的 PerpetualPosition (多头仓位)
        let mut perpetual_position_2 = create_perpetual_position(&perpetual_instrument_2);
        perpetual_position_2.meta.side = Side::Buy; // 设置为多头仓位
        account_positions.update_position(Position::Perpetual(perpetual_position_2.clone()));

        // 确保新仓位已正确添加
        assert!(account_positions.has_long_position(&perpetual_instrument_1));
        assert!(account_positions.has_long_position(&perpetual_instrument_2));
        assert_eq!(account_positions.perpetual_pos_long.len(), 2);
    }
}
