use serde::{Deserialize, Serialize};

// NOTE 滑点和摩擦成本的设计放在这里
use crate::common_infrastructure::instrument::kind::InstrumentKind;

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
    pub maker_fee: f64, // 制造流动性的费率
    pub taker_fee: f64, // 消耗流动性的费率
}

// 永续合约交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct PerpetualFees
{
    pub maker_fee: f64,   // 开仓费率
    pub taker_fee: f64,   // 平仓费率
    pub funding_fee: f64, // 资金费率
}

// 期货合约交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FutureFees
{
    pub maker_fee: f64,   // 开仓费率
    pub taker_fee: f64,   // 平仓费率
    pub funding_fee: f64, // 资金费率
}

// 期权交易费用结构体
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct OptionFees
{
    pub trade_fee: f64,
    // 交易费率
    // 期权可能还有其他费用，如行权费等，根据需要添加
}

#[derive(Debug, Copy, Clone, PartialOrd, Serialize, Deserialize, PartialEq)]
pub enum Fees
{
    Spot(SpotFees),
    Future(FutureFees),
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

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn instrument_fees_new_should_create_instrument_fees()
    {
        let fees = Fees::Spot(SpotFees { maker_fee: 0.1, taker_fee: 0.2 });
        let instrument_fees = InstrumentFees::new(InstrumentKind::Spot, fees.clone());
        assert_eq!(instrument_fees.instrument_kind, InstrumentKind::Spot);
        assert_eq!(instrument_fees.fees, fees);
    }

    #[test]
    fn spot_fees_should_serialize_and_deserialize_correctly()
    {
        let fees = SpotFees { maker_fee: 0.1, taker_fee: 0.2 };
        let serialized = serde_json::to_string(&fees).unwrap();
        let deserialized: SpotFees = serde_json::from_str(&serialized).unwrap();
        assert_eq!(fees, deserialized);
    }

    #[test]
    fn perpetual_fees_should_serialize_and_deserialize_correctly()
    {
        let fees = PerpetualFees { maker_fee: 0.1,
                                   taker_fee: 0.2,
                                   funding_fee: 0.01 };
        let serialized = serde_json::to_string(&fees).unwrap();
        let deserialized: PerpetualFees = serde_json::from_str(&serialized).unwrap();
        assert_eq!(fees, deserialized);
    }

    #[test]
    fn future_fees_should_serialize_and_deserialize_correctly()
    {
        let fees = FutureFees { maker_fee: 0.1,
                                taker_fee: 0.2,
                                funding_fee: 0.01 };
        let serialized = serde_json::to_string(&fees).unwrap();
        let deserialized: FutureFees = serde_json::from_str(&serialized).unwrap();
        assert_eq!(fees, deserialized);
    }

    #[test]
    fn option_fees_should_serialize_and_deserialize_correctly()
    {
        let fees = OptionFees { trade_fee: 0.1 };
        let serialized = serde_json::to_string(&fees).unwrap();
        let deserialized: OptionFees = serde_json::from_str(&serialized).unwrap();
        assert_eq!(fees, deserialized);
    }

    #[test]
    fn fees_should_serialize_and_deserialize_correctly()
    {
        let fees_variants = vec![Fees::Spot(SpotFees { maker_fee: 0.1, taker_fee: 0.2 }),
                                 Fees::Perpetual(PerpetualFees { maker_fee: 0.1,
                                                                 taker_fee: 0.2,
                                                                 funding_fee: 0.01 }),
                                 Fees::Future(FutureFees { maker_fee: 0.1,
                                                           taker_fee: 0.2,
                                                           funding_fee: 0.01 }),
                                 Fees::Option(OptionFees { trade_fee: 0.1 }),];
        for fees in fees_variants {
            let serialized = serde_json::to_string(&fees).unwrap();
            let deserialized: Fees = serde_json::from_str(&serialized).unwrap();
            assert_eq!(fees, deserialized);
        }
    }
}
