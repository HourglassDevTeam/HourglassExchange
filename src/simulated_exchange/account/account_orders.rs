use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    common_skeleton::{
        instrument::Instrument,
        order::{Open, Order, OrderId, RequestOpen},
    },
    error::ExecutionError,
    simulated_exchange::instrument_orders::InstrumentOrders,
};

#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct AccountOrders
{
    pub request_counter: u64, // 用来生成一个唯一的 [`OrderId`]。 NOTE：注意原子性
    pub all: HashMap<Instrument, InstrumentOrders>,
}
impl AccountOrders
{
    /// 从提供的 [`Instrument`] 选择构造一个新的 [`AccountOrders`]。
    /// NOTE 在新的场景下怎么初始化比较好？
    pub fn new(instruments: Vec<Instrument>) -> Self
    {
        Self { request_counter: 0,
               all: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect() }
    }

    /// 返回指定 [`Instrument`] 的客户端 [`InstrumentOrders`] 的可变引用。
    pub fn orders_mut(&mut self, instrument: &Instrument) -> Result<&mut InstrumentOrders, ExecutionError>
    {
        self.all
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange 没有为 Instrument: {instrument} 配置")))
    }

    /// 为每个 [`Instrument`] 获取出价和要价 [`Order<Open>`]。
    pub fn fetch_all(&self) -> Vec<Order<Open>>
    {
        self.all.values().flat_map(|market| [&market.bids, &market.asks]).flatten().cloned().collect()
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 并且新的总数被用作唯一的 [`OrderId`]。
    pub fn initiate_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open>
    {
        self.increment_request_counter();
        Order::from((self.order_id(), request))
    }

    /// 将 [`Order<RequestOpen>`] 计数器递增一以确保下一个生成的 [`OrderId`] 是唯一的。
    pub fn increment_request_counter(&mut self)
    {
        self.request_counter += 1;
    }

    /// 生成一个唯一的 [`OrderId`]。
    pub fn order_id(&self) -> OrderId
    {
        OrderId(self.request_counter.to_string())
    }
}
