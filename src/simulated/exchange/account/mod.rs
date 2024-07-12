use cerebro_data::subscription::trade::PublicTrade;
use cerebro_integration::model::{instrument::Instrument, Exchange, Side};
use chrono::Utc;
use std::{fmt::Debug, time::Duration};
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::{
    universal::{
        balance::{Balance, TokenBalance},
        event_and_status::{AccountEventKind, ClientAccountEvent},
        order::OrderKind,
    },
    Cancelled, ExchangeKind, ExecutionError, Open, Order, RequestCancel, RequestOpen,
};

use self::{balance::ClientBalances, order::ClientOrders};

/// [`AccountModule`] 每个 [`Symbol`](cerebro_integration::model::Symbol) 的 [`Balance`] 和
/// 相关的余额管理逻辑。
pub mod balance;

/// [`ClientAccount`] [`ClientOrders`] management & matching logic.
pub mod order;

#[derive(Clone, Debug)]
pub struct AccountModule {
    pub latency: Duration,
    pub fees_percent: f64,
    pub event_account_tx: mpsc::UnboundedSender<ClientAccountEvent>,
    pub balances: ClientBalances,
    pub orders: ClientOrders,
}

impl AccountModule {
    pub fn builder() -> AccountBuilder {
        AccountBuilder::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.orders.fetch_all()));
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        respond_with_latency(self.latency, response_tx, Ok(self.balances.fetch_all()));
    }

    /// 用于处理一组开仓订单请求 (open_requests)，并通过 oneshot 通道发送响应。
    /// 可模拟网络延迟（通过 respond_with_latency），提供更加真实的交易环境模拟。
    pub fn open_orders(&mut self, open_requests: Vec<Order<RequestOpen>>, response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>) {
        let open_results = open_requests.into_iter().map(|request| self.try_open_order_atomic(request)).collect();
        respond_with_latency(self.latency, response_tx, open_results);
    }

    /// try_open_order_atomic 是底层开仓执行函数，用于执行单个开仓订单请求。
    pub fn try_open_order_atomic(&mut self, request: Order<RequestOpen>) -> Result<Order<Open>, ExecutionError> {
        Self::check_order_kind_support(request.state.kind)?;

        // 计算开仓订单所需的可用余额
        let (symbol, required_balance) = request.required_available_balance();

        // 计算开仓订单所需的可用余额，并检查余额是否充足。
        self.balances.has_sufficient_available_balance(symbol, required_balance)?;

        // 构建 Open<Order>
        let open = self.orders.build_generate_order_open(request);

        // 检索客户端的 Instrument Orders
        let orders = self.orders.orders_mut(&open.instrument)?;

        // 由于易出错操作已成功，修改 ClientBalances 和 ClientOrders
        orders.add_generate_order_open(open.clone());
        let balance_event = self.balances.update_from_open(&open, required_balance);

        // 向客户端发送 AccountEvents
        self.event_account_tx
            .send(balance_event)
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Balance");

        self.event_account_tx
            .send(ClientAccountEvent {
                client_ts: Utc::now(),
                exchange: Exchange::from(ExchangeKind::Simulated),
                kind: AccountEventKind::OrdersNew(vec![open.clone()]),
            })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Trade");

        Ok(open)
    }

    /// Check if the [`Order<RequestOpen>`] [`OrderKind`] is supported.
    pub fn check_order_kind_support(kind: OrderKind) -> Result<(), ExecutionError> {
        match kind {
            | OrderKind::Limit | OrderKind::PostOnly => Ok(()),
            | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)),
        }
    }

    /// Execute cancel order requests and send the response via the provided [`oneshot::Sender`].
    pub fn cancel_orders(
        &mut self,
        cancel_requests: Vec<Order<RequestCancel>>,
        response_tx: oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
    ) {
        let cancel_results = cancel_requests.into_iter().map(|request| self.try_cancel_order_atomic(request)).collect();

        respond_with_latency(self.latency, response_tx, cancel_results);
    }

    /// Execute a cancel order request, removing it from the [`ClientOrders`] and updating the
    /// associated [`Balance`]. Sends an [`ClientAccountEvent`] for both the order cancel and balance
    /// update.
    pub fn try_cancel_order_atomic(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExecutionError> {
        // Retrieve client Instrument Orders
        let orders = self.orders.orders_mut(&request.instrument)?;

        // Find & remove Order<Open> associated with the Order<RequestCancel>
        let removed = match request.side {
            | Side::Buy => {
                // Search for Order<Open> using OrderId
                let index = orders
                    .bids
                    .iter()
                    .position(|bid| bid.state.id == request.state.id)
                    .ok_or(ExecutionError::OrderNotFound(request.cid))?;
                orders.bids.remove(index)
            }
            | Side::Sell => {
                // Search for Order<Open> using OrderId
                let index = orders
                    .asks
                    .iter()
                    .position(|ask| ask.state.id == request.state.id)
                    .ok_or(ExecutionError::OrderNotFound(request.cid))?;

                orders.asks.remove(index)
            }
        };

        // Now that fallible operations have succeeded, mutate ClientBalances
        let balance_event = self.balances.update_from_cancel(&removed);

        // Map Order<Open> to Order<Cancelled>
        let cancelled = Order::from(removed);

        // Send AccountEvents to client
        self.event_account_tx
            .send(ClientAccountEvent {
                client_ts: Utc::now(),
                exchange: Exchange::from(ExchangeKind::Simulated),
                kind: AccountEventKind::OrdersCancelled(vec![cancelled.clone()]),
            })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Trade");

        self.event_account_tx
            .send(ClientAccountEvent {
                client_ts: Utc::now(),
                exchange: Exchange::from(ExchangeKind::Simulated),
                kind: AccountEventKind::Balance(balance_event),
            })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Balance");

        Ok(cancelled)
    }

    /// Execute a cancel all orders request and send the response via the provided
    /// [`oneshot::Sender`].
    pub fn cancel_orders_all(&mut self, response_tx: oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>) {
        let removed_orders = self
            .orders
            .orders_by_instrument
            .values_mut()
            .flat_map(|orders| {
                let bids = orders.bids.drain(..);
                let asks = orders.asks.drain(..);

                bids.chain(asks)
            })
            .collect::<Vec<Order<Open>>>();

        let balance_updates = removed_orders
            .iter()
            .map(|cancelled| self.balances.update_from_cancel(cancelled))
            .collect();

        let cancelled_orders = removed_orders.into_iter().map(Order::from).collect::<Vec<Order<Cancelled>>>();

        // Send AccountEvents to client
        self.event_account_tx
            .send(ClientAccountEvent {
                client_ts: Utc::now(),
                exchange: Exchange::from(ExchangeKind::Simulated),
                kind: AccountEventKind::OrdersCancelled(cancelled_orders.clone()),
            })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::OrdersCancelled");

        self.event_account_tx
            .send(ClientAccountEvent {
                client_ts: Utc::now(),
                exchange: Exchange::from(ExchangeKind::Simulated),
                kind: AccountEventKind::Balances(balance_updates),
            })
            .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Balances");

        respond_with_latency(self.latency, response_tx, Ok(cancelled_orders))
    }

    /// Determine if the incoming [`PublicTrade`] liquidity matches any [`ClientOrders`] relating
    /// to the [`Instrument`]. If there are matches, trades are simulated by client orders being
    /// taken.
    pub fn match_orders(&mut self, instrument: Instrument, trade: PublicTrade) {
        // Client fees
        let fees_percent = self.fees_percent;

        // Access the ClientOrders relating to the Instrument of the PublicTrade
        let orders = match self.orders.orders_mut(&instrument) {
            | Ok(orders) => orders,
            | Err(error) => {
                warn!(
                    ?error, %instrument, ?trade, "cannot match orders with unrecognised Instrument"
                );
                return;
            }
        };

        // Match client Order<Open>s to incoming PublicTrade if the liquidity intersects
        let trades = match orders.has_matching_order(&trade) {
            | Some(Side::Buy) => orders.match_bids(&trade, fees_percent),
            | Some(Side::Sell) => orders.match_asks(&trade, fees_percent),
            | None => return,
        };

        // Apply Balance updates for each client Trade and send AccountEvents to client
        for trade in trades {
            // Update Balances
            let balances_event = self.balances.update_from_trade(&trade);

            self.event_account_tx
                .send(balances_event)
                .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Balances");

            self.event_account_tx
                .send(ClientAccountEvent {
                    client_ts: Utc::now(),
                    exchange: Exchange::from(ExchangeKind::Simulated),
                    kind: AccountEventKind::Trade(trade),
                })
                .expect("[UniLinkExecution] : Client is offline - failed to send AccountEvent::Trade");
        }
    }
}

/// Sends the provided `Response` via the [`oneshot::Sender`] after waiting for the latency
/// [`Duration`]. Used to simulate network latency between the exchange and client.
pub fn respond_with_latency<Response>(latency: Duration, response_tx: oneshot::Sender<Response>, response: Response)
where
    Response: Debug + Send + 'static,
{
    tokio::spawn(async move {
        tokio::time::sleep(latency).await;
        response_tx
            .send(response)
            .expect("[UniLinkExecution] : SimulatedExchange failed to send oneshot response to execution request")
    });
}

#[derive(Debug, Default)]
pub struct AccountBuilder {
    latency: Option<Duration>,
    fees_percent: Option<f64>,
    event_account_tx: Option<mpsc::UnboundedSender<ClientAccountEvent>>,
    instruments: Option<Vec<Instrument>>,
    balances: Option<ClientBalances>,
}

impl AccountBuilder {
    fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn latency(self, value: Duration) -> Self {
        Self {
            latency: Some(value),
            ..self
        }
    }

    pub fn fees_percent(self, value: f64) -> Self {
        Self {
            fees_percent: Some(value),
            ..self
        }
    }

    pub fn event_account_tx(self, value: mpsc::UnboundedSender<ClientAccountEvent>) -> Self {
        Self {
            event_account_tx: Some(value),
            ..self
        }
    }

    pub fn instruments(self, value: Vec<Instrument>) -> Self {
        Self {
            instruments: Some(value),
            ..self
        }
    }

    pub fn balances(self, value: ClientBalances) -> Self {
        Self {
            balances: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<AccountModule, ExecutionError> {
        // Construct ClientAccount
        let client_account = AccountModule {
            latency: self.latency.ok_or_else(|| ExecutionError::BuilderIncomplete("latency".to_string()))?,
            fees_percent: self
                .fees_percent
                .ok_or_else(|| ExecutionError::BuilderIncomplete("fees_percent".to_string()))?,
            event_account_tx: self
                .event_account_tx
                .ok_or_else(|| ExecutionError::BuilderIncomplete("event_account_tx".to_string()))?,
            balances: self.balances.ok_or_else(|| ExecutionError::BuilderIncomplete("balances".to_string()))?,
            orders: self
                .instruments
                .map(ClientOrders::new)
                .ok_or_else(|| ExecutionError::BuilderIncomplete("instruments".to_string()))?,
        };

        // Validate each Instrument base & quote Symbol has an associated Balance
        client_account
            .orders
            .orders_by_instrument
            .keys()
            .flat_map(|instrument| [&instrument.base, &instrument.quote])
            .map(|symbol| client_account.balances.balance(symbol))
            .collect::<Result<Vec<&Balance>, ExecutionError>>()?;

        Ok(client_account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_order_kind_support() {
        struct TestCase {
            kind: OrderKind,
            expected: Result<(), ExecutionError>,
        }

        let tests = vec![
            TestCase {
                // TC0: Market
                kind: OrderKind::Market,
                expected: Err(ExecutionError::UnsupportedOrderKind(OrderKind::Market)),
            },
            TestCase {
                // TC1: Limit
                kind: OrderKind::Limit,
                expected: Ok(()),
            },
            TestCase {
                // TC2: PostOnly
                kind: OrderKind::PostOnly,
                expected: Ok(()),
            },
            TestCase {
                // TC3: Immediate Or Cancel
                kind: OrderKind::ImmediateOrCancel,
                expected: Err(ExecutionError::UnsupportedOrderKind(OrderKind::ImmediateOrCancel)),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = AccountModule::check_order_kind_support(test.kind);
            match test.expected {
                | Ok(()) => assert!(actual.is_ok(), "TC{} failed", index),
                | Err(_) => assert!(actual.is_err(), "TC{} failed", index),
            }
        }
    }
}
