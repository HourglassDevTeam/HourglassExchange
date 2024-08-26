use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_display_should_format_correctly() {
        let token = Token::from("btc");
        assert_eq!(format!("{}", token), "BTC");
    }

    #[test]
    fn token_as_ref_should_return_inner_string() {
        let token = Token::from("eth");
        assert_eq!(token.as_ref(), "ETH");
    }

    #[test]
    fn token_from_should_create_token_from_string() {
        let token = Token::from("usdt");
        assert_eq!(token, Token::new("usdt"));
    }

    #[test]
    fn token_new_should_create_token_with_uppercase() {
        let token = Token::new("xrp");
        assert_eq!(token.as_ref(), "XRP");
    }

    #[test]
    fn token_equality_should_work_correctly() {
        let token1 = Token::from("btc");
        let token2 = Token::from("BTC");
        assert_eq!(token1, token2);
    }

    #[test]
    fn token_ordering_should_work_correctly() {
        let token1 = Token::from("btc");
        let token2 = Token::from("eth");
        assert!(token1 < token2);
    }
}