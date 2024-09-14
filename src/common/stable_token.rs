use crate::common::token::Token;

#[allow(dead_code)]
pub enum StableToken
{
    Bitcoin,
    Tether,
    USD,           // USDC
    BinanceUSD,    // BUSD
    Dai,           // DAI
    PaxosStandard, // PAX
    TrueUSD,       // TUSD
    GeminiDollar,  // GUSD
    TerraUSD,      // UST
    Frax,          // FRAX
    NeutrinoUSD,   // USDN
}

#[allow(dead_code)]
impl StableToken
{
    /// 根据 `StableToken` 的变种生成对应的 `Token`
    pub fn to_token(&self) -> Token
    {
        match self {
            | StableToken::Bitcoin => Token::btc(),
            | StableToken::Tether => Token::usdt(),
            | StableToken::USD => Token::new("USDC"),
            | StableToken::BinanceUSD => Token::new("BUSD"),
            | StableToken::Dai => Token::new("DAI"),
            | StableToken::PaxosStandard => Token::new("PAX"),
            | StableToken::TrueUSD => Token::new("TUSD"),
            | StableToken::GeminiDollar => Token::new("GUSD"),
            | StableToken::TerraUSD => Token::new("UST"),
            | StableToken::Frax => Token::new("FRAX"),
            | StableToken::NeutrinoUSD => Token::new("USDN"),
        }
    }

    /// 判断给定的 `Token` 是否属于稳定币
    pub fn is_stable_token(token: &Token) -> bool
    {
        match token.as_ref() {
            | "BTC" => true,  // Bitcoin
            | "USDT" => true, // Tether
            | "USDC" => true, // USD Coin
            | "BUSD" => true, // BinanceUSD
            | "DAI" => true,  // Dai
            | "PAX" => true,  // Paxos Standard
            | "TUSD" => true, // TrueUSD
            | "GUSD" => true, // Gemini Dollar
            | "UST" => true,  // TerraUSD
            | "FRAX" => true, // Frax
            | "USDN" => true, // Neutrino USD
            | _ => false,     // 非稳定币
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_is_stable_token()
    {
        // 测试稳定币
        let token_btc = Token::new("BTC");
        let token_usdt = Token::new("USDT");
        let token_usdc = Token::new("USDC");
        let token_busd = Token::new("BUSD");
        let token_dai = Token::new("DAI");

        assert!(StableToken::is_stable_token(&token_btc)); // Bitcoin 是稳定币
        assert!(StableToken::is_stable_token(&token_usdt)); // Tether 是稳定币
        assert!(StableToken::is_stable_token(&token_usdc)); // USD Coin 是稳定币
        assert!(StableToken::is_stable_token(&token_busd)); // BinanceUSD 是稳定币
        assert!(StableToken::is_stable_token(&token_dai)); // Dai 是稳定币

        // 测试非稳定币
        let token_xrp = Token::new("XRP");
        let token_eth = Token::new("ETH");

        assert!(!StableToken::is_stable_token(&token_xrp)); // XRP 不是稳定币
        assert!(!StableToken::is_stable_token(&token_eth)); // ETH 不是稳定币
    }
}
