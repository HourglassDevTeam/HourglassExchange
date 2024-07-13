use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

/// eg/ "btc", "eth", "usdt", etc
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]

pub struct Token(String);

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Token::new)
    }
}

impl<S> From<S> for Token
where
    S: Into<String>,
{
    fn from(input: S) -> Self {
        Token::new(input)
    }
}

impl Token {
    /// Construct a new [`Token`] new type using the provided `Into<Symbol>` value.

    pub fn new<S>(input: S) -> Self
    where
        S: Into<String>,
    {
        Self(input.into().to_lowercase())
    }
}
