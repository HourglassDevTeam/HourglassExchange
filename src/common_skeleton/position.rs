/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
/// CONSIDER: can these positions coexist, if so enums might not be ideal.
use serde::{Deserialize, Serialize};

use crate::common_skeleton::instrument::kind::InstrumentKind;
use crate::{
    common_skeleton::{balance::TokenBalance, friction::Fees, instrument::Instrument, Side},
    ExchangeVariant,
};

/// This struct is generic and thus placed here in the common_skeleton.
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountPositions {
    pub spot_pos: Option<Vec<SpotPosition>>,
    pub margin_pos: Option<Vec<MarginPosition>>, // NOTE useless in backtest
    pub perpetual_pos: Option<Vec<PerpetualPosition>>,
    pub futures_pos: Option<Vec<FuturesPosition>>,
    pub option_pos: Option<Vec<OptionPosition>>,
}

impl AccountPositions {
    pub(crate) fn has_position(&self, instrument: &Instrument) -> bool {
        match instrument.kind {
            | InstrumentKind::Spot => self
                .spot_pos
                .as_ref()
                .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Perpetual => self
                .perpetual_pos
                .as_ref()
                .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Future => self
                .futures_pos
                .as_ref()
                .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Option => self
                .option_pos
                .as_ref()
                .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Margin => self
                .margin_pos
                .as_ref()
                .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
        }
    }
}

/// NOTE : PositionMode 枚举定义了两种交易方向模式：
///  [NetMode] : 单向模式。在这种模式下，用户只能持有一个方向的仓位（多头或空头），而不能同时持有两个方向的仓位。
/// [LongShortMode] : 双向模式。在这种模式下，用户可以同时持有多头和空头仓位。这在一些复杂的交易策略中可能会有用，例如对冲策略。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionMode {
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

/// NOTE : PositionMarginMode has defined two modes of margin consumption.
///  [Cross]: 交叉保证金模式。在这种模式下，所有仓位共享一个保证金池，盈亏共用。如果仓位的保证金不足，将从账户余额中提取以补充不足。
///  [Isolated]: 逐仓保证金模式。在这种模式下，每个仓位都有独立的保证金，盈亏互不影响。如果某个仓位的保证金不足，该仓位将被强制平仓，而不会影响到其他仓位。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionMarginMode {
    Cross,
    Isolated,
}

/// NOTE: 可能需要多种头寸类型共存
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PositionKind {
    Perpetual(PerpetualPosition),
    Margin(MarginPosition),
    Futures(FuturesPosition),
    Option(OptionPosition),
}

/// TODO : the below code is under construction
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SpotPosition {
    pub meta: PositionMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MarginPosition {
    pub meta: PositionMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FuturesPosition {
    pub meta: PositionMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OptionPosition {
    pub meta: PositionMeta,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PerpetualPosition {
    pub meta: PositionMeta,
    pub pos_config: PerpetualPositionConfig,
    pub liquidation_price: f64,
    pub margin: f64,
    pub funding_fee: f64,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PerpetualPositionConfig {
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
    position_mode: PositionMode,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta {
    pub position_id: String,
    pub enter_ts: i64,
    pub update_ts: i64,
    pub exit_balance: TokenBalance, // NOTE necessary but unsure currently
    pub account_exchange_ts: i64,
    pub exchange: ExchangeVariant,
    pub instrument: Instrument,
    pub side: Side, // NOTE 注意和DirectionMode之间的兼容性
    pub current_size: f64,
    pub current_fees_total: Fees,
    pub current_avg_price_gross: f64,
    pub current_symbol_price: f64,
    pub current_avg_price: f64,
    pub unrealised_pnl: f64,
    pub realised_pnl: f64,
}
