use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::common::{instrument::kind::InstrumentKind, token::Token};

pub mod kind;

// 定义Instrument结构体，用于表示金融工具。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Instrument
{
    pub base: Token,  // 基础货币
    pub quote: Token, // 报价货币
    #[serde(rename = "instrument_kind")]
    pub kind: InstrumentKind, // 金融工具的类型
}

// 为Instrument实现Display trait，方便打印显示。
impl Display for Instrument
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "({}_{}, {})", self.base, self.quote, self.kind)
    }
}

// 允许从元组直接构造Instrument。
impl<S> From<(S, S, InstrumentKind)> for Instrument where S: Into<Token>
{
    fn from((base, quote, kind): (S, S, InstrumentKind)) -> Self
    {
        Self { base: base.into(),
               quote: quote.into(),
               kind }
    }
}

// 提供一个构造函数来创建新的Instrument实例。
impl Instrument
{
    pub fn new<S>(base: S, quote: S, kind: InstrumentKind) -> Self
        where S: Into<Token>
    {
        Self { base: base.into(),
               quote: quote.into(),
               kind }
    }
}

// InstrumentInitiator结构用于通过步骤方式构建Instrument。
pub struct InstrumentInitiator
{
    base: Option<Token>,
    quote: Option<Token>,
    kind: Option<InstrumentKind>,
}

impl Default for InstrumentInitiator
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl InstrumentInitiator
{
    // 初始化构建器，所有字段均为None。
    pub fn new() -> Self
    {
        InstrumentInitiator { base: None,
                              quote: None,
                              kind: None }
    }

    // 设置基础货币。
    pub fn base<S>(mut self, base: S) -> Self
        where S: Into<Token>
    {
        self.base = Some(base.into());
        self
    }

    // 设置报价货币。
    pub fn quote<S>(mut self, quote: S) -> Self
        where S: Into<Token>
    {
        self.quote = Some(quote.into());
        self
    }

    // 设置金融工具类型。
    pub fn kind(mut self, kind: InstrumentKind) -> Self
    {
        self.kind = Some(kind);
        self
    }

    // 结束构建，并尝试生成Instrument。如果任何字段未设置，将返回错误。
    pub fn initiate(self) -> Result<Instrument, &'static str>
    {
        Ok(Instrument { base: self.base.ok_or("Base is missing")?,
                        quote: self.quote.ok_or("Quote is missing")?,
                        kind: self.kind.ok_or("Instrument kind is missing")? })
    }
}

#[cfg(test)]
mod tests
{
    use crate::sandbox::ExchangeInitiator;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn initiator_should_create_exchange_initiator_with_default_values()
    {
        let initiator = ExchangeInitiator::new();
        assert!(initiator.event_sandbox_rx.is_none());
        assert!(initiator.account.is_none());
    }

    #[tokio::test]
    async fn initiator_should_set_event_sandbox_rx()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let initiator = ExchangeInitiator::new().event_sandbox_rx(rx);
        assert!(initiator.event_sandbox_rx.is_some());
    }

    #[tokio::test]
    async fn initiator_should_return_error_if_account_is_missing()
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        let initiator = ExchangeInitiator::new().event_sandbox_rx(rx);
        let result = initiator.initiate();
        assert!(result.is_err());
    }
}
