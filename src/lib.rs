// 高性能和标准化的交易接口，能够在多个金融场所执行交易。同时提供一个功能丰富的模拟交易所，以协助进行回测和干式交易。通过初始化其关联的ClientExecution实例与交易所进行通信。
// ClientExecution trait 提供了一个统一且简单的语言，用于与交易所进行交互。

#![allow(clippy::type_complexity)]

use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    error::ExecutionError,
    universal::{
        balance::TokenBalance,
        event::AccountEvent,
        order::{Cancelled, Opened, Order, RequestCancel, RequestOpen},
    },
};

/// 在实时、干运行或模拟执行过程中产生的错误。
pub mod error;
// pub simulated simulated;
pub mod simulated;
/// 支持在交易所执行操作的核心数据结构。
/// 例如：`Order`（订单）、`Balance`（余额）、`Trade`（交易）等。
pub mod universal;

/// 定义与交易所的通信。每个交易所集成都需要自己的实现。
#[async_trait]
pub trait ClientExecution {
    const CLIENT: ExchangeKind;
    type Config;

    /// 使用提供的[`Self::Config`]和[`AccountEvent`]发送器初始化一个新的[`ClientExecution`]。
    /// 通常包括启动一个异步WebSocket事件循环以从交易所接收[`AccountEvent`]，
    /// 同时返回HTTP客户端`Self`。
    async fn init(config: Self::Config, event_tx: mpsc::UnboundedSender<AccountEvent>) -> Self;
    async fn fetch_orders_open(&self) -> Result<Vec<Order<Opened>>, ExecutionError>;
    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>;
    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Opened>, ExecutionError>>;
    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>;

    /// 取消所有账户中的[`Order<Opened>`]（未完成订单）。
    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Debug)]
pub struct Exchange(Cow<'static, str>);

impl<E> From<E> for Exchange
where
    E: Into<Cow<'static, str>>,
{
    fn from(exchange: E) -> Self {
        Exchange(exchange.into())
    }
}

impl Display for Exchange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an [`ClientExecution`] implementation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "execution", rename_all = "snake_case")]
pub enum ExchangeKind {
    Simulated,
    Ftx,
}

impl From<ExchangeKind> for Exchange {
    fn from(execution_kind: ExchangeKind) -> Self {
        Exchange::from(execution_kind.as_str())
    }
}

impl Display for ExchangeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            | ExchangeKind::Simulated => "simulated",
            | ExchangeKind::Ftx => "ftx",
        }
    }
}
