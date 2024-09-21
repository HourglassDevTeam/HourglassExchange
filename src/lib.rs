use crate::{
    common::{
        account_positions::{AccountPositions, Position},
        balance::TokenBalance,
        event::AccountEvent,
        instrument::Instrument,
        order::{
            states::{cancelled::Cancelled, request_cancel::RequestCancel, request_open::RequestOpen},
            Order,
        },
        token::Token,
    },
    error::ExchangeError,
};
use async_trait::async_trait;
use common::order::states::open::Open;
use mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use tokio::sync::mpsc;

pub mod common;
pub mod dashboard;
pub mod error;
pub mod hourglass;
pub mod network;
pub mod test_utils;
pub mod vault;

/// 定义与交易所的通信。每个交易所集成都需要自己的实现。
#[async_trait]
pub trait ClientExecution
{
    const CLIENT_KIND: Exchange;
    // NOTE 这个类型关联项表示配置类型，不同的交易所可能需要不同的配置。例如，API 密钥、API 密码、或其他初始化参数等等。
    type Config;

    async fn init(config: Self::Config, event_tx: UnboundedSender<AccountEvent>) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExchangeError>;
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExchangeError>; // 补全 FetchAllPositions 的实现
    async fn fetch_all_positions(&self) -> Result<AccountPositions, ExchangeError>; // 补全 FetchLongPosition 的实现
    async fn fetch_long_position(&self, instrument: Instrument) -> Result<Option<Position>, ExchangeError>; // 补全 FetchShortPosition 的实现
    async fn fetch_short_position(&self, instrument: Instrument) -> Result<Option<Position>, ExchangeError>;
    // async fn fetch_balance(&self) -> Result<TokenBalance, ExchangeError>; // TODO
    // async fn fetch_positions(&self) -> Result<AccountPositions, ExchangeError>;  // TODO
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExchangeError>>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExchangeError>>;
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExchangeError>; // 实现 DepositTokens 的处理逻辑
    async fn deposit_tokens(&self, deposits: Vec<(Token, f64)>) -> Result<Vec<TokenBalance>, ExchangeError>;
    // 发送 LetItRoll 命令的函数
    async fn let_it_roll(&self) -> Result<(), ExchangeError>;
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Exchange
{
    Hourglass,
    Binance,
    Okex,
}

impl Display for Exchange
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.as_str())
    }
}

impl Exchange
{
    pub fn as_str(&self) -> &'static str
    {
        match self {
            | Exchange::Hourglass => "hourglass",
            | Exchange::Okex => "okex",
            | Exchange::Binance => "binance",
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_exchange_variant_display()
    {
        let variant = Exchange::Okex;
        assert_eq!(format!("{}", variant), "okex");
    }

    #[test]
    fn test_exchange_variant_as_str()
    {
        assert_eq!(Exchange::Hourglass.as_str(), "hourglass");
        assert_eq!(Exchange::Binance.as_str(), "binance");
        assert_eq!(Exchange::Okex.as_str(), "okex");
    }
}
