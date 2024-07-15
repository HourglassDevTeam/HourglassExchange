// NOTE 滑点和摩擦成本的设计放在这里

use serde::{Deserialize, Serialize};
use crate::common_skeleton::instrument::kind::InstrumentKind;

#[allow(dead_code)]
/// 以 [`Instrument`]（符号）表示的 [`Trade`]（交易）费用。
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct InstrumentFees
{
    pub instrument_kind: InstrumentKind,
    pub fees: f64,
}

impl InstrumentFees
{
    /// 构造一个新的 [`InstrumentFees`]。
    pub fn new<S>(instrument: S, fees: f64) -> Self
        where S: Into<InstrumentKind>
    {
        Self { instrument_kind: instrument.into(), fees }
    }
}
