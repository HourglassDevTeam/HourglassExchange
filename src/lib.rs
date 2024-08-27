use std::fmt::{Debug, Display, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    common_infrastructure::{
        balance::TokenBalance,
        event::AccountEvent,
        order::{Cancelled, Open, Order, Pending, RequestCancel, RequestOpen},
    },
    error::ExecutionError,
};

mod binance;
pub mod common_infrastructure;
pub mod error;
pub mod network;
pub mod okex;
pub mod sandbox;

/// 定义与交易所的通信。每个交易所集成都需要自己的实现。
#[async_trait]
pub trait ClientExecution
{
    const CLIENT_KIND: Exchange;
    // NOTE 这个类型关联项表示配置类型，不同的交易所可能需要不同的配置。例如，API 密钥、API 密码、或其他初始化参数等等。
    type Config;

    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>, local_timestamp: i64) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>;
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>;
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Pending>, ExecutionError>>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>;
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>;
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Exchange
{
    SandBox,
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
            | Exchange::SandBox => "sandbox",
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
        assert_eq!(Exchange::SandBox.as_str(), "sandbox");
        assert_eq!(Exchange::Binance.as_str(), "binance");
        assert_eq!(Exchange::Okex.as_str(), "okex");
    }
}
