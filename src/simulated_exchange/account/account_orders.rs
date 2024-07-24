use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use tokio::sync::RwLock;

use crate::{
    common_skeleton::{
        instrument::Instrument,
        order::{Open, Order, OrderId, RequestOpen},
    },
    error::ExecutionError,
    simulated_exchange::{
        account::account_latency::{fluctuate_latency, AccountLatency},
        instrument_orders::InstrumentOrders,
    },
};

// NOTE as a matter of fact this is only usable in SimulatedExchange.
impl From<(OrderId, Order<RequestOpen>)> for Order<Open>
{
    fn from((id, request): (OrderId, Order<RequestOpen>)) -> Self
    {
        Self { kind: request.kind,
               exchange: request.exchange.clone(),
               instrument: request.instrument.clone(),
               cid: request.cid,
               client_ts: request.client_ts,
               side: request.side,
               state: Open { id,
                             price: request.state.price,
                             size: request.state.size,
                             filled_quantity: 0.0,
                             received_ts: request.client_ts /* add the delay to the client_ts */ } }
    }
}

#[derive(Debug)]
pub struct AccountOrders
{
    pub latency: Arc<RwLock<AccountLatency>>,
    pub request_counter: AtomicU64, // 用来生成一个唯一的 [`OrderId`]。
    pub instrument_orders_map: HashMap<Instrument, InstrumentOrders>,
}

impl AccountOrders
{
    /// 从提供的 [`Instrument`] 选择构造一个新的 [`AccountOrders`]。
    pub fn new(instruments: Vec<Instrument>, account_latency: AccountLatency) -> Self
    {
        Self { request_counter: AtomicU64::new(0),
               instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect(),
               latency: Arc::new(RwLock::new(account_latency)) }
    }

    /// 返回指定 [`Instrument`] 的客户端 [`InstrumentOrders`] 的可变引用。
    pub fn orders_mut(&mut self, instrument: &Instrument) -> Result<&mut InstrumentOrders, ExecutionError>
    {
        self.instrument_orders_map
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange 没有为 Instrument: {instrument} 配置")))
    }

    /// 为每个 [`Instrument`] 获取出价和要价 [`Order<Open>`]。
    pub fn fetch_all(&self) -> Vec<Order<Open>>
    {
        self.instrument_orders_map
            .values()
            .flat_map(|market| [&market.bids, &market.asks])
            .flatten()
            .cloned()
            .collect()
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub fn build_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open>
    {
        self.increment_request_counter();
        Order::from((self.order_id(), request))
    }

    pub fn increment_request_counter(&self)
    {
        self.request_counter.fetch_add(1, Ordering::Relaxed);
    }

    // 在 order_id 方法中，使用 Ordering::Acquire 确保读取到最新的计数器值。
    pub fn order_id(&self) -> OrderId
    {
        OrderId(self.request_counter.load(Ordering::Acquire).to_string())
    }

    pub async fn update_latency(&mut self, current_time: i64)
    {
        let mut latency = self.latency.write().await;
        fluctuate_latency(&mut *latency, current_time);
    }
}
