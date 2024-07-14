use std::fmt::Debug;

use num_traits::FromPrimitive;
use rand::{thread_rng, Rng};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::common_skeleton::data::event::MarketEvent;
use crate::{
    error::ExecutionError,
    common_skeleton::{
        balance::TokenBalance,
        event::AccountEvent,
        order::{Cancelled, Open, Order, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
    },
    ExchangeKind,
};
use account_balances::AccountBalances;
use account_config::AccountConfig;
use account_orders::AccountOrders;
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::trade::Trade;

mod account_balances;
mod account_config;
mod account_orders;

// 订阅方式
#[derive(Clone, Debug)]
pub enum SubscriptionKind {
    Database,
    // Kafka,
    // DolphinDB,
    // RedisCache
    WebSocket,
    HTTP,
}

#[derive(Clone, Debug)]
pub struct DataSource {
    pub subscription: SubscriptionKind,
    pub exchange_kind: ExchangeKind,
}

#[derive(Clone, Debug)]
pub struct AccountFeedData<Data> {
    pub data_source: DataSource,
    pub batch_id: Uuid,
    pub data: Vec<MarketEvent<Data>>,
}
#[derive(Clone, Debug)]
pub struct Account<Data, Event> {
    pub data: AccountFeedData<Data>,
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub market_event_tx: mpsc::UnboundedSender<MarketEvent<Event>>,
    // changing latency in milliseconds, specific to each account.
    pub latency: i64,
    pub config: AccountConfig,
    pub balances: AccountBalances,
    pub positions: Vec<AccountPositions>,
    pub orders: AccountOrders,
}

#[derive(Clone, Debug)]
pub struct AccountInitiator<Data, Event> {
    data: Option<AccountFeedData<Data>>,
    account_event_tx: Option<mpsc::UnboundedSender<AccountEvent>>,
    market_event_tx: Option<mpsc::UnboundedSender<MarketEvent<Event>>>,
    latency: Option<i64>,
    config: Option<AccountConfig>,
    balances: Option<AccountBalances>,
    positions: Option<Vec<AccountPositions>>,
    orders: Option<AccountOrders>,
}

impl<Data, Event> AccountInitiator<Data, Event> {
    pub fn new() -> Self {
        AccountInitiator {
            data: None,
            account_event_tx: None,
            market_event_tx: None,
            latency: None,
            config: None,
            balances: None,
            positions: None,
            orders: None,
        }
    }

    pub fn data(mut self, value: AccountFeedData<Data>) -> Self {
        self.data = Some(value);
        self
    }

    pub fn account_event_tx(mut self, value: mpsc::UnboundedSender<AccountEvent>) -> Self {
        self.account_event_tx = Some(value);
        self
    }

    pub fn market_event_tx(mut self, value: mpsc::UnboundedSender<MarketEvent<Event>>) -> Self {
        self.market_event_tx = Some(value);
        self
    }

    pub fn latency(mut self, value: i64) -> Self {
        self.latency = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self {
        self.config = Some(value);
        self
    }

    pub fn balances(mut self, value: AccountBalances) -> Self {
        self.balances = Some(value);
        self
    }

    pub fn positions(mut self, value: Vec<AccountPositions>) -> Self {
        self.positions = Some(value);
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self {
        self.orders = Some(value);
        self
    }

    pub fn build(self) -> Result<Account<Data, Event>, String> {
        Ok(Account {
            data: self.data.ok_or("data is required")?,
            account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?,
            market_event_tx: self.market_event_tx.ok_or("market_event_tx is required")?,
            latency: self.latency.ok_or("latency is required")?,
            config: self.config.ok_or("config is required")?,
            balances: self.balances.ok_or("balances is required")?,
            positions: self.positions.ok_or("positions are required")?,
            orders: self.orders.ok_or("orders are required")?,
        })
    }
}

// NOTE 未完成
impl<Data, Event> Account<Data, Event> {
    pub fn initiate() -> AccountInitiator {
        AccountInitiator::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.orders.fetch_all()));
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.balances.fetch_all()));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError> {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                    * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    pub fn fetch_positions(&self, response_tx: oneshot::Sender<Result<Vec<AccountPositions>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.positions.clone()));
    }

    pub fn match_orders(&mut self, instrument: Instrument, trade: Trade) {
        todo!()
    }

    pub fn open_orders(
        &mut self,
        open_requests: Vec<Order<RequestOpen>>,
        response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>,
    ) {
        let open_results = open_requests.into_iter().map(|request| self.try_open_order_atomic(request)).collect();
        response_tx.send(open_results).unwrap_or_else(|_| {
            // Handle the error if sending fails
        });
    }

    pub fn try_open_order_atomic(&mut self, request: Order<RequestOpen>) -> Result<Order<Open>, ExecutionError> {
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

    pub fn update_latency(&mut self, min_millis: u64, max_millis: u64) {
        self.latency = random_duration(min_millis, max_millis);
    }
}

// send oneshot response to execution request
pub fn respond_with_latency<Response>(latency: i64, response_tx: oneshot::Sender<Response>, response: Response)
where
    Response: Debug + Send + 'static,
{
    tokio::spawn(async move {
        response_tx
            .send(response)
            .expect("[UnilinkExecution] : SimulatedExchange failed to send oneshot response to execution request")
    });
}

// Generate a random duration between min_millis and max_millis (inclusive)
// NOTE Exercise careful handling with timestamp unit conversions! digits！
pub fn random_duration(min_millis: u64, max_millis: u64) -> i64 {
    let mut rng = thread_rng();
    let random_millis = rng.gen_range(min_millis..=max_millis);
    i64::from_u64(random_millis).unwrap()
}
