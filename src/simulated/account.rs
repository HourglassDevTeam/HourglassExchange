use std::{collections::HashMap, fmt::Debug, time::i64};

use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use ExchangeKind::Simulated;

use crate::universal::data::event::MarketEvent;
use crate::{
    error::ExecutionError,
    simulated::instrument_orders::InstrumentOrders,
    universal::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::Instrument,
        order::{Cancelled, Open, Order, OrderId, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
        token::Token,
        trade::Trade,
        Side,
    },
    Exchange, ExchangeKind,
};

#[derive(Clone, Debug)]
pub enum SubscriptionKind {
    ClickHouse,
    // Kafka,
    // DolphinDB,
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
pub struct Account<Data> {
    pub data: AccountFeedData<Data>,
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub market_event_tx: mpsc::UnboundedSender<AccountEvent>,
    pub latency: u64,
    pub config: AccountConfig,
    pub balances: AccountBalances,
    pub positions: Vec<AccountPositions>,
    pub orders: AccountOrders,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct AccountBalances(pub HashMap<Token, Balance>);

impl AccountBalances {
    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError> {
        self.get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError> {
        self.get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 获取每个[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance> {
        self.0
            .clone()
            .into_iter()
            .map(|(token, balance)| TokenBalance::new(token, balance))
            .collect()
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError> {
        let available = self.balance(token)?.available;
        match available >= required_balance {
            true => Ok(()),
            false => Err(ExecutionError::InsufficientBalance(token.clone())),
        }
    }

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> AccountEvent {
        let updated_balance = match open.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&open.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence is questionable");

                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&open.instrument.base)
                    .expect("[UniLinkExecution] : Balance existence is questionable");

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

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> TokenBalance {
        match cancelled.side {
            Side::Buy => {
                let balance = self
                    .balance_mut(&cancelled.instrument.quote)
                    .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            Side::Sell => {
                let balance = self
                    .balance_mut(&cancelled.instrument.base)
                    .expect("[UniLinkExecution] : Balance existence checked when opening Order");

                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// 当client[`Trade`]发生时，会导致base和quote[`Token`]的[`Balance`]发生变化。
    /// 每个[`Balance`]变化的性质取决于匹配的[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    /// [`Side::Buy`]匹配会导致基础[`Token`] [`Balance`]增加`trade_quantity`，报价[`Token`]减少`trade_quantity * price`。
    /// [`Side::Sell`]匹配会导致基础[`Token`] [`Balance`]减少`trade_quantity`，报价[`Token`]增加`trade_quantity * price`。

    pub fn update_from_trade(&mut self, trade: &Trade) -> AccountEvent {
        let Instrument { base, quote, .. } = &trade.instrument;

        // Calculate the base & quote Balance deltas
        let (base_delta, quote_delta) = match trade.side {
            Side::Buy => {
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
            Side::Sell => {
                // Base total decreases by trade.size
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta {
                    total: -trade.size,
                    available: 0.0,
                };

                // Quote total & available increase by (trade.size * price) minus quote fees
                let quote_increase = (trade.size * trade.price) - trade.fees;
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
    pub all: HashMap<Instrument, InstrumentOrders>,
}
impl AccountOrders {
    /// 从提供的 [`Instrument`] 选择构造一个新的 [`AccountOrders`]。
    pub fn new(instruments: Vec<Instrument>) -> Self {
        Self {
            request_counter: 0,
            all: instruments
                .into_iter()
                .map(|instrument| (instrument, InstrumentOrders::default()))
                .collect(),
        }
    }

    /// 返回指定 [`Instrument`] 的客户端 [`InstrumentOrders`] 的可变引用。
    pub fn orders_mut(&mut self, instrument: &Instrument) -> Result<&mut InstrumentOrders, ExecutionError> {
        self.all
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange 没有为 Instrument: {instrument} 配置")))
    }

    /// 为每个 [`Instrument`] 获取出价和要价 [`Order<Open>`]。
    pub fn fetch_all(&self) -> Vec<Order<Open>> {
        self.all
            .values()
            .flat_map(|market| [&market.bids, &market.asks])
            .flatten()
            .cloned()
            .collect()
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 并且新的总数被用作唯一的 [`OrderId`]。
    pub fn build_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open> {
        self.increment_request_counter();
        Order::from((self.order_id(), request))
    }

    /// 将 [`Order<RequestOpen>`] 计数器递增一以确保下一个生成的 [`OrderId`] 是唯一的。
    pub fn increment_request_counter(&mut self) {
        self.request_counter += 1;
    }

    /// 生成一个唯一的 [`OrderId`]。
    pub fn order_id(&self) -> OrderId {
        OrderId(self.request_counter.to_string())
    }
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
    latency: Option<i64>,
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

    pub fn latency(self, value: i64) -> Self {
        Self {
            latency: Some(value),
            ..self
        }
    }

    pub fn config(self, value: AccountConfig) -> Self {
        Self { config: Some(value), ..self }
    }

    pub fn balances(self, value: AccountBalances) -> Self {
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

impl<Data> Account<Data> {
    pub fn initiator() -> AccountBuilder {
        AccountBuilder::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.orders.fetch_all()));
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.balances.fetch_all()));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError> {
        match kind {
            OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                  * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
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
}

// send oneshot response to execution request
pub fn respond_with_latency<Response>(latency: i64, response_tx: oneshot::Sender<Response>, response: Response)
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
pub fn random_duration(min_millis: u64, max_millis: u64) -> u64 {
    let mut rng = thread_rng();
    let random_millis = rng.gen_range(min_millis..=max_millis);
    i64::from_millis(random_millis)
}
