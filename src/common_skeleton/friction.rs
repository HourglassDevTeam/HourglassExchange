use serde::{Deserialize, Serialize};

// NOTE 滑点和摩擦成本的设计放在这里
use crate::common_skeleton::instrument::kind::InstrumentKind;

#[allow(dead_code)]
/// 以 [`Instrument`]（符号）表示的 [`Trade`]（交易）费用。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct InstrumentFees
{
    pub instrument_kind: InstrumentKind,
    pub fees: Fees,
}

// NOTE 根据 OKEx 交易所的 API 和交易费用结构，我们可以为每种费用类型创建单独的结构体来表示不同的费用属性。
//      以下是构建 SpotFees、PerpetualFees 和 OptionFees 变种的一个示例：
// 现货交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct SpotFees
{
    // 这里假设费用为固定值，实际情况可能更复杂。
    pub maker_fee_rate: f64, // 制造流动性的费率
    pub taker_fee_rate: f64, // 消耗流动性的费率
}

// 永续合约交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct PerpetualFees
{
    pub open_fee_rate: f64,  // 开仓费率
    pub close_fee_rate: f64, // 平仓费率
    pub funding_rate: f64,   // 资金费率
}

// 期权交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct OptionFees
{
    pub trade_fee_rate: f64,
    // 交易费率
    // 期权可能还有其他费用，如行权费等，根据需要添加
}

#[derive(Debug, Copy, Clone, PartialOrd, Serialize, Deserialize, PartialEq)]
pub enum Fees
{
    Spot(SpotFees),
    Perpetual(PerpetualFees),
    Option(OptionFees),
}

impl InstrumentFees
{
    /// 构造一个新的 [`InstrumentFees`]。
    pub fn new<S>(instrument: S, fees: Fees) -> Self
        where S: Into<InstrumentKind>
    {
        Self { instrument_kind: instrument.into(),
               fees }
    }
}
