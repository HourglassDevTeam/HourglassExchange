use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    common_skeleton::{
        balance::TokenBalance,
        event::AccountEvent,
        order::{Cancelled, Open, Order, RequestCancel, RequestOpen},
    },
    error::ExecutionError,
};

pub mod error;
pub mod simulated_exchange;
mod binance_exchange;
pub mod common_skeleton;
pub mod data_subscriber;
pub mod okex_exchange;

/// 定义与交易所的通信。每个交易所集成都需要自己的实现。
#[async_trait]
pub trait ClientExecution
{
    const CLIENT_KIND: ExchangeKind;
    // NOTE 这个类型关联项表示配置类型，不同的交易所可能需要不同的配置。例如，API 密钥、API 密码、或其他初始化参数等等。
    type Config;

    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>,local_timestamp:i64) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>;
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>;
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>;
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Debug)]
pub struct Exchange(Cow<'static, str>);

impl<E> From<E> for Exchange where E: Into<Cow<'static, str>>
{
    fn from(exchange: E) -> Self
    {
        Exchange(exchange.into())
    }
}

impl Display for Exchange
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum ExchangeKind
{
    Simulated,
    Binance,
    Okex,
}

impl From<ExchangeKind> for Exchange
{
    fn from(execution_kind: ExchangeKind) -> Self
    {
        Exchange::from(execution_kind.as_str())
    }
}

impl Display for ExchangeKind
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeKind
{
    pub fn as_str(&self) -> &'static str
    {
        match self {
            | ExchangeKind::Simulated => "simulated_exchange",
            | ExchangeKind::Okex => "ftx",
            | ExchangeKind::Binance => "binance",
        }
    }
}
