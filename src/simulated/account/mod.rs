use std::fmt::Debug;

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use account_balances::AccountBalances;
use account_config::AccountConfig;
use account_orders::AccountOrders;

use crate::{
    common_skeleton::{
        balance::TokenBalance,
        data::event::MarketEvent,
        event::AccountEvent,
        instrument::Instrument,
        order::{Cancelled, Open, Order, OrderKind, RequestCancel, RequestOpen},
        position::AccountPositions,
        trade::Trade,
    },
    error::ExecutionError,
    simulated::account::account_latency::{AccountLatency, fluctuate_latency},
};
use crate::simulated::account::account_datafeed::AccountDataFeed;

pub mod account_balances;
pub mod account_config;
mod account_latency;
pub mod account_orders;
mod account_datafeed;


#[derive(Clone, Debug)]
pub struct Account<Data, Event>
{
    pub data: AccountDataFeed<Data>,                                // 帐户数据
    pub account_event_tx: mpsc::UnboundedSender<AccountEvent>,      // 帐户事件发送器
    pub market_event_tx: mpsc::UnboundedSender<MarketEvent<Event>>, // 市场事件发送器
    pub latency: AccountLatency,                                    // 帐户延迟
    pub config: AccountConfig,                                      // 帐户配置
    pub balances: AccountBalances,                                  // 帐户余额
    pub positions: Vec<AccountPositions>,                           // 帐户头寸
    pub orders: AccountOrders,                                      // 帐户订单
}

#[derive(Clone, Debug)]
pub struct AccountInitiator<Data, Event>
{
    data: Option<AccountDataFeed<Data>>,
    account_event_tx: Option<mpsc::UnboundedSender<AccountEvent>>,
    market_event_tx: Option<mpsc::UnboundedSender<MarketEvent<Event>>>,
    latency: Option<AccountLatency>,
    config: Option<AccountConfig>,
    balances: Option<AccountBalances>,
    positions: Option<Vec<AccountPositions>>,
    orders: Option<AccountOrders>,
}

impl<Data, Event> AccountInitiator<Data, Event>
{
    pub fn new() -> Self
    {
        AccountInitiator { data: None,
                           account_event_tx: None,
                           market_event_tx: None,
                           latency: None,
                           config: None,
                           balances: None,
                           positions: None,
                           orders: None }
    }

    pub fn data(mut self, value: AccountDataFeed<Data>) -> Self
    {
        self.data = Some(value);
        self
    }

    pub fn account_event_tx(mut self, value: mpsc::UnboundedSender<AccountEvent>) -> Self
    {
        self.account_event_tx = Some(value);
        self
    }

    pub fn market_event_tx(mut self, value: mpsc::UnboundedSender<MarketEvent<Event>>) -> Self
    {
        self.market_event_tx = Some(value);
        self
    }

    pub fn latency(mut self, value: AccountLatency) -> Self
    {
        self.latency = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self
    {
        self.config = Some(value);
        self
    }

    pub fn balances(mut self, value: AccountBalances) -> Self
    {
        self.balances = Some(value);
        self
    }

    pub fn positions(mut self, value: Vec<AccountPositions>) -> Self
    {
        self.positions = Some(value);
        self
    }

    pub fn orders(mut self, value: AccountOrders) -> Self
    {
        self.orders = Some(value);
        self
    }

    pub fn build(self) -> Result<Account<Data, Event>, String>
    {
        Ok(Account { data: self.data.ok_or("data is required")?,                                     // 检查并获取data
                     account_event_tx: self.account_event_tx.ok_or("account_event_tx is required")?, // 检查并获取account_event_tx
                     market_event_tx: self.market_event_tx.ok_or("market_event_tx is required")?,    // 检查并获取market_event_tx
                     latency: self.latency.ok_or("latency is required")?,                            // 检查并获取latency
                     config: self.config.ok_or("config is required")?,                               // 检查并获取config
                     balances: self.balances.ok_or("balances is required")?,                         // 检查并获取balances
                     positions: self.positions.ok_or("positions are required")?,                     // 检查并获取positions
                     orders: self.orders.ok_or("orders are required")?
                     /* 检查并获取orders */ })
    }
}

// NOTE 未完成
#[allow(dead_code)]
impl<Data, Event> Account<Data, Event>
{
    pub fn initiate() -> AccountInitiator<Data, Event>
    {
        AccountInitiator::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>)
    {
        respond_with_latency(self.latency.current_value, response_tx, Ok(self.orders.fetch_all()));
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>)
    {
        respond_with_latency(self.latency.current_value, response_tx, Ok(self.balances.fetch_all()));
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError>
    {
        match kind {
            | OrderKind::Market | OrderKind::Limit | OrderKind::ImmediateOrCancel | OrderKind::FillOrKill | OrderKind::GoodTilCancelled => Ok(()), /* NOTE 不同交易所支持的订单种类不同，如有需要过滤的OrderKind变种，我们要在此处特殊设计
                                                                                                                                                    * | unsupported => Err(ExecutionError::UnsupportedOrderKind(unsupported)), */
        }
    }

    pub fn fetch_positions(&self, response_tx: oneshot::Sender<Result<Vec<AccountPositions>, ExecutionError>>)
    {
        respond_with_latency(self.latency.current_value, response_tx, Ok(self.positions.clone()));
    }
    pub fn match_orders(&mut self, _instrument: Instrument, _trade: Trade)
    {
        todo!()
    }

    pub fn open_orders(&mut self,
                       open_requests: Vec<Order<RequestOpen>>,
                       response_tx: oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>,
                       current_timestamp: i64)
    {
        let open_results = open_requests.into_iter()
                                        .map(|request| self.try_open_order_atomic(request, current_timestamp))
                                        .collect();
        response_tx.send(open_results).unwrap_or_else(|_| {
                                          // Handle the error if sending fails
                                      });
    }

    pub fn try_open_order_atomic(&mut self, request: Order<RequestOpen>, _current_timestamp: i64) -> Result<Order<Open>, ExecutionError>
    {
        Self::order_validity_check(request.state.kind).unwrap();
        todo!()
    }

    pub fn cancel_orders(&mut self,
                         cancel_requests: Vec<Order<RequestCancel>>,
                         response_tx: oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
                         current_timestamp: i64)
    {
        let cancel_results = cancel_requests.into_iter()
                                            .map(|request| self.try_cancel_order_atomic(request, current_timestamp))
                                            .collect();
        response_tx.send(cancel_results).unwrap_or_else(|_| {
                                            // Handle the error if sending fails
                                        });
    }

    pub fn try_cancel_order_atomic(&mut self, _request: Order<RequestCancel>, _current_timestamp: i64) -> Result<Order<Cancelled>, ExecutionError>
    {
        todo!()
    }

    pub fn cancel_orders_all(&mut self, _response_tx: oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>, _current_timestamp: i64)
    {
        todo!()
    }

    pub fn update_latency(&mut self, current_time: i64)
    {
        fluctuate_latency(&mut self.latency, current_time);
    }
}

// send oneshot response to execution request
pub fn respond_with_latency<Response>(_latency: i64, response_tx: oneshot::Sender<Response>, response: Response)
    where Response: Debug + Send + 'static
{
    tokio::spawn(async move {
        response_tx.send(response)
                   .expect("[UnilinkExecution] : SimulatedExchange failed to send oneshot response to execution request")
    });
}
