use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

/// eg/ "btc", "eth", "usdt", etc
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize,Deserialize)]

pub struct Token(String);


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


impl<S> From<S> for Token
where
    S: Into<String>,
{
    fn from(input: S) -> Self {
        Token::new(input)
    }
}

impl Token {
    /// Construct a new [`Token`] new type using the provided `Into<Token>` value.

    pub fn new<S>(input: S) -> Self
    where
        S: Into<String>,
    {
        Self(input.into().to_uppercase())
    }
}
