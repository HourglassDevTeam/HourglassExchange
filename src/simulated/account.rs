use std::{collections::HashMap, fmt::Debug, time::Duration};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use ExchangeKind::Simulated;

use crate::{
    error::ExecutionError,
    universal::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::Instrument,
        order::{Cancelled, Opened, Order, OrderId, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
        token::Token,
        Side,
    },
    Exchange, ExchangeKind,
};
use crate::universal::trade::Trade;

#[derive(Clone, Debug)]
pub struct AccountInfo {
    pub event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    pub latency: Duration,
    pub config: AccountConfig,
    pub balances: AccountBalances,
    pub positions: Vec<AccountPositions>,
    pub orders: AccountOrders,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct AccountBalances(pub HashMap<Token, Balance>);

impl AccountBalances {
    /// Return a reference to the [`Balance`] of the specified [`Token`].
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError> {
        self.get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// Return a mutable reference to the [`Balance`] of the specified [`Token`].
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError> {
        self.get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// Fetch the client [`Balance`] for every [`Token``].
    pub fn fetch_all(&self) -> Vec<TokenBalance> {
        self.0
            .clone()
            .into_iter()
            .map(|(token, balance)| TokenBalance::new(token, balance))
            .collect()
    }

    /// Determine if the client has sufficient available [`Balance`] to execute an
    /// [`Order<RequestOpen>`].
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError> {
        let available = self.balance(token)?.available;
        match available >= required_balance {
            | true => Ok(()),
            | false => Err(ExecutionError::InsufficientBalance(token.clone())),
        }
    }

    /// Updates the associated [`Token`] [`Balance`] when a client creates an [`Order<Opened>`]. The
    /// nature of the [`Balance`] change will depend on if the [`Order<Opened>`] is a
    /// [`Side::Buy`] or [`Side::Sell`].
    pub fn update_from_open(&mut self, open: &Order<Opened>, required_balance: f64) -> AccountEvent {
        let updated_balance = match open.side {
            | Side::Buy => {
                let balance = self
                    .balance_mut(&open.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence checked in has_sufficient_available_balance");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self
                    .balance_mut(&open.instrument.base)
                    .expect("[UniLinkExecution] : Balance existence checked in has_sufficient_available_balance");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent {
            client_ts: todo!(),
            exchange: Exchange::from(Simulated),
            kind: AccountEventKind::Balance(updated_balance),
        }
    }

    /// Updates the associated [`Token`] [`Balance`] when a client cancels an [`Order<Opened>`]. The
    /// nature of the [`Balance`] change will depend on if the [`Order<Opened>`] was a
    /// [`Side::Buy`] or [`Side::Sell`].
    pub fn update_from_cancel(&mut self, cancelled: &Order<Opened>) -> TokenBalance {
        match cancelled.side {
            | Side::Buy => {
                let balance = self
                    .balance_mut(&cancelled.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self
                    .balance_mut(&cancelled.instrument.base)
                    .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// When a client [`Trade`] occurs, it causes a change in the [`Balance`] of the base & quote
    /// [`Token`]. The nature of each [`Balance`] change will depend on if the matched
    /// [`Order<Opened>`] was a [`Side::Buy`] or [`Side::Sell`].
    ///
    /// A [`Side::Buy`] match causes the [`Token`] [`Balance`] of the base to increase by the
    /// `trade_quantity`, and the quote to decrease by the `trade_quantity * price`.
    ///
    /// A [`Side::Sell`] match causes the [`Token`] [`Balance`] of the base to decrease by the
    /// `trade_quantity`, and the quote to increase by the `trade_quantity * price`.
    pub fn update_from_trade(&mut self, trade: &Trade) -> AccountEvent {
        let Instrument { base, quote, .. } = &trade.instrument;

        // Calculate the base & quote Balance deltas
        let (base_delta, quote_delta) = match trade.side {
            | Side::Buy => {
                // Base total & available increase by trade.size minus base trade.fees
                let base_increase = trade.size - trade.fees;
                let base_delta = BalanceDelta {
                    total: base_increase,
                    available: base_increase,
                };

                // Quote total decreases by (trade.size * price)
                // Note: available was already decreased by the opening of the Side::Buy order
                let quote_delta = BalanceDelta {
                    total: -trade.size * trade.price,
                    available: 0.0,
                };

                (base_delta, quote_delta)
            }
            | Side::Sell => {
                // Base total decreases by trade.size
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta {
                    total: -trade.size,
                    available: 0.0,
                };

                // Quote total & available increase by (trade.size * price) minus quote fees
                let quote_increase = (trade.size * trade.price) - trade.fees.fees;
                let quote_delta = BalanceDelta {
                    total: quote_increase,
                    available: quote_increase,
                };

                (base_delta, quote_delta)
            }
        };

        // Apply BalanceDelta & return updated Balance
        let base_balance = self.update(base, base_delta);
        let quote_balance = self.update(quote, quote_delta);

        AccountEvent {
            client_ts: todo!(),
            exchange: Exchange::from(Simulated),
            kind: AccountEventKind::Balances(vec![
                TokenBalance::new(base.clone(), base_balance),
                TokenBalance::new(quote.clone(), quote_balance),
            ]),
        }
    }

    /// Apply the [`BalanceDelta`] to the [`Balance`] of the specified [`Token`], returning a
    /// `Copy` of the updated [`Balance`].
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl std::ops::Deref for AccountBalances {
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AccountBalances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct AccountOrders {
    pub request_counter: u64,
    pub all: HashMap<Instrument, Orders>,
}

impl AccountOrders {
    /// Construct a new [`AccountOrders`] from the provided selection of [`Instrument`]s.
    pub fn new(instruments: Vec<Instrument>) -> Self {
        Self {
            request_counter: 0,
            all: instruments.into_iter().map(|instrument| (instrument, Orders::default())).collect(),
        }
    }

    /// Return a mutable reference to the client [`Orders`] of the specified [`Instrument`].
    pub fn orders_mut(&mut self, instrument: &Instrument) -> Result<&mut Orders, ExecutionError> {
        self.all
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Instrument: {instrument}")))
    }

    /// Fetch the bid and ask [`Order<Opened>`]s for every [`Instrument`].
    pub fn fetch_all(&self) -> Vec<Order<Opened>> {
        self.all
            .values()
            .flat_map(|market| [&market.bids, &market.asks])
            .flatten()
            .cloned()
            .collect()
    }

    /// Build an [`Order<Opened>`] from the provided [`Order<RequestOpen>`]. The request counter
    /// is incremented and the new total is used as a unique [`OrderId`].
    pub fn build_order_open(&mut self, request: Order<RequestOpen>) -> Order<Opened> {
        self.increment_request_counter();
        Order::from((self.order_id(), request))
    }

    /// Increment the [`Order<RequestOpen>`] counter by one to ensure the next generated
    /// [`OrderId`] is unique.
    pub fn increment_request_counter(&mut self) {
        self.request_counter += 1;
    }

    /// Generate a unique [`OrderId`].
    pub fn order_id(&self) -> OrderId {
        OrderId(self.request_counter.to_string())
    }
}

/// Client [`Orders`] for an [`Instrument`]. Simulates client orders in an real multi-participant OrderBook.
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct Orders {
    pub trade_counter: u64,
    pub bids: Vec<Order<Opened>>,
    pub asks: Vec<Order<Opened>>,
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

impl AccountInfo {
    pub fn initiator() -> AccountBuilder {
        AccountBuilder::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Opened>>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.orders.fetch_all()));
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.balances.fetch_all()));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError> {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::GoodTilCancelled =>
            // Add logic to validate market order
            {
                Ok(())
            } /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
               * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
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