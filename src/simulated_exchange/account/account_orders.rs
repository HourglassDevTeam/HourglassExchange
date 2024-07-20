use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::{
    common_skeleton::{
        instrument::Instrument,
        order::{Open, Order, OrderId, RequestOpen},
    },
    error::ExecutionError,
    simulated_exchange::instrument_orders::InstrumentOrders,
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AccountOrders
{
    pub request_counter: AtomicU64, // 用来生成一个唯一的 [`OrderId`]。
    pub instrument_orders_map: HashMap<Instrument, InstrumentOrders>,
}


/// NOTE Relaxed 是 Rust 中的一个内存排序选项，代表“顺序一致性”（Sequential Consistency）。
///     这是在使用原子操作（例如原子计数器、原子指针）时用于指定操作之间的内存顺序的一种模式。
///     Relaxed 是最强的内存排序，它保证了所有线程看到的操作顺序是一致的，即所有线程按照相同的顺序看到内存操作。
impl AccountOrders
{
    /// 从提供的 [`Instrument`] 选择构造一个新的 [`AccountOrders`]。
    pub fn new(instruments: Vec<Instrument>) -> Self
    {
        Self { request_counter: AtomicU64::new(0),
            instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect() }
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
        self.instrument_orders_map.values().flat_map(|market| [&market.bids, &market.asks]).flatten().cloned().collect()
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub fn initiate_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open>
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
}
