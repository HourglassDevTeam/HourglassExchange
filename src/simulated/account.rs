use std::{fmt::Debug, time::Duration};

use rand::{thread_rng, Rng};
use serde_json::ser::State;
use tokio::sync::oneshot;

use crate::{
    error::ExecutionError,
    universal::{
        balance::TokenBalance,
        order::{Cancelled, Opened, Order, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
    },
};

#[derive(Clone, Debug)]
pub struct AccountInfo<State> {
    pub latency: Duration,
    pub config: AccountConfig,
    pub balances: AccountBalances,
    pub positions: AccountPositions,
    pub orders: Vec<Order<State>>,
}

#[derive(Clone, Debug)]
pub struct AccountConfig {
    pub margin_mode: MarginMode,
    pub position_mode: PositionMode,
    pub commission_level: CommissionLevel,
}

#[derive(Clone, Debug)]
pub enum MarginMode {
    SimpleMode,
    SingleCurrencyMargin,
    MultiCurrencyMargin,
    PortfolioMargin,
}

#[derive(Clone, Debug)]
pub enum PositionMode {
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

#[derive(Clone, Debug)]
pub enum CommissionLevel {
    Lv1,
    Lv2,
    // ..........
}


#[derive(Clone, Debug)]
// NOTE wrap fields with option<> to yield support for initiation in a chained fashion
pub struct AccountBuilder {
    config: Option<AccountConfig>,
    balances: Option<AccountBalances>,
    positions: Option<AccountPositions>,
    latency: Option<Duration>,
}

impl AccountBuilder {
    pub fn new() -> Self {
        AccountBuilder {
            config: None,
            balances: None,
            positions: None,
            latency: None,
        }
    }

    pub fn latency(self, value: Duration) -> Self {
        Self {
            latency: Some(value),
            ..self
        }
    }

    pub fn config(self, value: AccountConfig) -> Self {
        Self { config: Some(value), ..self }
    }

    pub fn balances(self, value: Vec<TokenBalance>) -> Self {
        Self {
            balances: Some(value),
            ..self
        }
    }

    pub fn positions(self, value: AccountPositions) -> Self {
        Self {
            positions: Some(value),
            ..self
        }
    }


}

impl AccountInfo<State> {
    pub fn initiator() -> AccountBuilder {
        AccountBuilder::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Opened>>, ExecutionError>>) {
        todo!()
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        todo!()
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError> {
        todo!()
    }

    pub fn try_open_order_atomic(&mut self, request: Order<RequestOpen>) -> Result<Order<Opened>, ExecutionError> {
        Self::order_validity_check(request.state.kind).unwrap();
        todo!()
    }

    pub fn cancel_orders(
        &mut self,
        cancel_requests: Vec<Order<RequestCancel>>,
        response_tx: oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
    ) {
        let cancel_results = cancel_requests.into_iter().map(|request| self.try_cancel_order_atomic(request)).collect();
        response_tx.send(cancel_results).unwrap_or_else(|_| {
            // Handle the error if sending fails
        });
    }

    pub fn try_cancel_order_atomic(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExecutionError> {
        todo!()
    }

    pub fn cancel_orders_all(&mut self, response_tx: oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>) {
        todo!()
    }
}

// send oneshot response to execution request
pub fn respond_with_latency<Response>(latency: Duration, response_tx: oneshot::Sender<Response>, response: Response)
                                      where
                                          Response: Debug + Send + 'static,
{
    tokio::spawn(async move {
        response_tx
            .send(response)
            .expect("[TideBroker] : SimulatedExchange failed to send oneshot response to execution request")
    });
}



// Generate a random duration between min_millis and max_millis (inclusive)
pub fn random_duration(min_millis: u64, max_millis: u64) -> Duration {
    let mut rng = thread_rng();
    let random_millis = rng.gen_range(min_millis..=max_millis);
    Duration::from_millis(random_millis)
}