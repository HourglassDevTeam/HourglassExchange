/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
/// CONSIDER: can these positions coexist, if so enums might not be ideal.
use serde::{Deserialize, Serialize};

use crate::common_infrastructure::{
    instrument::{Instrument, kind::InstrumentKind},
    position::{future::FuturesPosition, leveraged_token::LeveragedTokenPosition, option::OptionPosition, perpetual::PerpetualPosition},
};

mod future;
mod leveraged_token;
mod option;
pub mod perpetual;
pub(crate) mod position_meta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountPositions
{
    pub margin_pos: Option<Vec<LeveragedTokenPosition>>, // NOTE useless in backtest
    pub perpetual_pos: Option<Vec<PerpetualPosition>>,
    pub futures_pos: Option<Vec<FuturesPosition>>,
    pub option_pos: Option<Vec<OptionPosition>>,
}

impl AccountPositions {
    /// 检查账户中是否持有指定交易工具的仓位
    pub(crate) fn has_position(&self, instrument: &Instrument) -> bool {
        match instrument.kind {
            // 对于现货，检查余额而不是仓位
            InstrumentKind::Spot => todo!(),
            // 商品期权
            InstrumentKind::CommodityOption => todo!(),
            // 商品期货
            InstrumentKind::CommodityFuture => todo!(),
            // 永续合约
            InstrumentKind::Perpetual => self.perpetual_pos
                .as_ref() // 如果存在仓位列表
                .map_or(false, |positions| // 如果有任何一个 pos 满足条件，any 返回 true，否则返回 false。
                    positions.iter().any(|pos| pos.meta.instrument == *instrument)
                ),
            // 普通期货
            InstrumentKind::Future => self.futures_pos
                .as_ref()
                .map_or(false, |positions|
                    positions.iter().any(|pos| pos.meta.instrument == *instrument)
                ),
            // 加密期权
            InstrumentKind::CryptoOption => self.option_pos
                .as_ref()
                .map_or(false, |positions|
                    positions.iter().any(|pos| pos.meta.instrument == *instrument)
                ),
            // 加密杠杆代币
            InstrumentKind::CryptoLeveragedToken => self.margin_pos
                .as_ref()
                .map_or(false, |positions|
                    positions.iter().any(|pos| pos.meta.instrument == *instrument)
                ),
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
