use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]

pub enum InstrumentKind
{
    Spot,      // [NOTE] 注意：Spot 指的是即期合约，此处现在缺乏合约细节字段，不适合MarketID的唯一识别。
    Perpetual, // [NOTE] 注意：Perpetual 指的是永续合约，此处现缺乏合约细节字段，不适合MarketID的唯一识别。
    Future(FutureContract),
    Option(OptionContract),
}

impl Default for InstrumentKind
{
    fn default() -> Self
    {
        Self::Spot
    }
}

impl Display for InstrumentKind
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self {
            | InstrumentKind::Spot => write!(f, "spot"),
            | InstrumentKind::Future(future) => {
                write!(f, "{}_{}", future.future_code, future.multiplier,)
            }
            | InstrumentKind::Perpetual => write!(f, "perpetual"),
            | InstrumentKind::Option(option) => {
                write!(f, "{}_{}", option.option_code, option.multiplier,)
            }
        }
    }
}

/// [InstrumentKind::Option] 合约的配置。
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]

pub struct OptionContract
{
    pub option_code: String,
    pub direction: OptionSide,    // call或者put
    pub exercise: OptionExercise, // 美式或者欧式
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub maturity: DateTime<Utc>, // 到期日
    pub strike: Decimal,          // 行权价格
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub listeddate: DateTime<Utc>, // 上市日期
    pub multiplier: u32,
}

/// [InstrumentKind::Future] 合约的配置。
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]

pub struct FutureContract
{
    pub future_code: String,
    // pub maturity: DateTime<Utc>, //到期日, not necessary currently
    pub multiplier: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]

pub enum OptionSide
{
    #[serde(alias = "CALL", alias = "Call", alias = "C")]
    Call,
    #[serde(alias = "PUT", alias = "Put", alias = "P")]
    Put,
}

impl Display for OptionSide
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | OptionSide::Call => "call",
            | OptionSide::Put => "put",
        })
    }
}

/// [OptionContract] 行权方式。
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]

pub enum OptionExercise
{
    #[serde(alias = "AMERICAN", alias = "American", alias = "美式", alias = "美式期权")]
    American,
    #[serde(alias = "EUROPEAN", alias = "European", alias = "欧式", alias = "欧式期权")]
    European,
}

impl Display for OptionExercise
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", match self {
            | OptionExercise::American => "american",
            | OptionExercise::European => "european",
        })
    }
}
