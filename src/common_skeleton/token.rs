use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

/// 表示加密货币或其他代币，例如 "btc", "eth", "usdt" 等
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Token(String);

impl Display for Token
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Token
{
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl<S> From<S> for Token where S: Into<String>
{
    fn from(input: S) -> Self
    {
        Token::new(input)
    }
}

impl Token
{
    /// 使用提供的 `Into<String>` 值构造一个新的 [`Token`]。
    pub fn new<S>(input: S) -> Self
        where S: Into<String>
    {
        Self(input.into().to_uppercase())
    }
}
