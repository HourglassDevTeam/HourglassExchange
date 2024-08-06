/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
/// CONSIDER: can these positions coexist, if so enums might not be ideal.
use serde::{Deserialize, Serialize};

use crate::common_skeleton::{
    instrument::{kind::InstrumentKind, Instrument},
    position::{future::FuturesPosition, leveraged_token::LeveragedTokenPosition, option::OptionPosition, perpetual::PerpetualPosition},
};

mod future;
mod leveraged_token;
pub(crate) mod meta;
mod option;
pub mod perpetual;
/// This struct is generic and thus placed here in the common_skeleton.
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountPositions
{
    pub margin_pos: Option<Vec<LeveragedTokenPosition>>, // NOTE useless in backtest
    pub perpetual_pos: Option<Vec<PerpetualPosition>>,
    pub futures_pos: Option<Vec<FuturesPosition>>,
    pub option_pos: Option<Vec<OptionPosition>>,
}

impl AccountPositions
{
    pub(crate) fn has_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            // NOTE : For Spot， check balance rather than position
            | InstrumentKind::Spot => todo!(),
            | InstrumentKind::Perpetual => self.perpetual_pos
                                               .as_ref()
                                               .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Future => self.futures_pos
                                            .as_ref()
                                            .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Option => self.option_pos
                                            .as_ref()
                                            .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
            | InstrumentKind::Margin => self.margin_pos
                                            .as_ref()
                                            .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
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

/// NOTE : PositionMarginMode has defined two modes of margin consumption.
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
pub enum PositionKind
{
    Perpetual(PerpetualPosition),
    LeveragedToken(LeveragedTokenPosition),
    Future(FuturesPosition),
    Option(OptionPosition),
}
