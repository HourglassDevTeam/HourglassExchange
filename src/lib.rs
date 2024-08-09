use std::fmt::{Debug, Display, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    common_skeleton::{
        balance::TokenBalance,
        event::AccountEvent,
        order::{Cancelled, Open, Order, Pending, RequestCancel, RequestOpen},
    },
    error::ExecutionError,
};

mod binance_exchange;
pub mod common_skeleton;
pub mod error;
pub mod okex_exchange;
pub mod simulated;

/// 定义与交易所的通信。每个交易所集成都需要自己的实现。
#[async_trait]
pub trait ClientExecution
{
    const CLIENT_KIND: ExchangeVariant;
    // NOTE 这个类型关联项表示配置类型，不同的交易所可能需要不同的配置。例如，API 密钥、API 密码、或其他初始化参数等等。
    type Config;

    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>, local_timestamp: i64) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>;
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>;
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Pending>, ExecutionError>>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>;
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Debug)]
pub struct ExchangeID(String);

impl<E> From<E> for ExchangeID where E: Into<String>
{
    fn from(exchange: E) -> Self
    {
        ExchangeID(exchange.into())
    }
}

impl Display for ExchangeID
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum ExchangeVariant
{
    Simulated,
    Binance,
    Okex,
}

impl From<ExchangeVariant> for ExchangeID
{
    fn from(execution_kind: ExchangeVariant) -> Self
    {
        ExchangeID::from(execution_kind.as_str())
    }
}

impl Display for ExchangeVariant
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeVariant
{
    pub fn as_str(&self) -> &'static str
    {
        match self {
            | ExchangeVariant::Simulated => "simulated",
            | ExchangeVariant::Okex => "ftx",
            | ExchangeVariant::Binance => "binance",
        }
    }
}
