use rand::Rng;
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
        order::{Open, Order, OrderId, Pending, RequestOpen},
    },
    error::ExecutionError,
    simulated_exchange::{
        account::account_latency::{fluctuate_latency, AccountLatency},
        instrument_orders::InstrumentOrders,
    },
};

#[derive(Debug)]
pub struct AccountOrders
{
    pub latency_generator: Arc<RwLock<AccountLatency>>,
    pub selectable_latencies: [i64; 20],
    pub request_counter: AtomicU64,            // 用来生成一个唯一的 [`OrderId`]。
    pub pending_registry: Vec<Order<Pending>>, // Pending订单的寄存器。
    pub instrument_orders_map: HashMap<Instrument, InstrumentOrders>,
}

impl AccountOrders
{
    /// 从提供的 [`Instrument`] 选择构造一个新的 [`AccountOrders`]。
    pub async fn new(instruments: Vec<Instrument>, account_latency: AccountLatency) -> Self
    {
        let latency_generator = Arc::new(RwLock::new(account_latency));
        let selectable_latencies = Self::generate_latencies(&latency_generator).await;

        Self { request_counter: AtomicU64::new(0),
               pending_registry: vec![],
               instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect(),
               latency_generator,
               selectable_latencies }
    }

    async fn generate_latencies(latency_generator: &Arc<RwLock<AccountLatency>>) -> [i64; 20]
    {
        let mut latencies = [0; 20];
        let mut generator = latency_generator.write().await;
        for latency in &mut latencies {
            fluctuate_latency(&mut generator, 0); // 这里0只是一个占位，可以根据需求调整
            *latency = generator.current_value;
        }
        latencies
    }

    fn get_random_latency(&self) -> i64
    {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.selectable_latencies.len());
        self.selectable_latencies[idx]
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

    pub async fn keep_request_as_pending(&mut self, request: Order<RequestOpen>) -> Order<Pending>
    {
        // turn the request into an pending order with a predicted timestamp
        let latency = self.get_random_latency();
        let adjusted_client_ts = request.client_ts + latency;
        let pending = Order { kind: request.kind,
                              exchange: request.exchange,
                              instrument: request.instrument,
                              cid: request.cid,
                              client_ts: request.client_ts,
                              side: request.side,
                              state: Pending { predicted_ts: adjusted_client_ts } };
        self.pending_registry.push(pending.clone());
        pending
    }


    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub async fn build_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open>
    {
        self.increment_request_counter();

        // 获取当前的 AccountLatency 值并加到 client_ts 上
        let latency = self.get_random_latency();
        let adjusted_client_ts = request.client_ts + latency;

        // 直接构建 Order<Open>
        Order { kind: request.kind,
                exchange: request.exchange,
                instrument: request.instrument,
                cid: request.cid,
                client_ts: request.client_ts,
                side: request.side,
                state: Open { id: self.order_id(),
                              price: request.state.price,
                              size: request.state.size,
                              filled_quantity: 0.0,
                              received_ts: adjusted_client_ts } }
    }

    pub fn increment_request_counter(&self)
    {
        self.request_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// 在 order_id 方法中，使用 [Ordering::Acquire] 确保读取到最新的计数器值。
    pub fn order_id(&self) -> OrderId
    {
        OrderId(self.request_counter.load(Ordering::Acquire).to_string())
    }

    pub async fn update_latency(&mut self, current_time: i64)
    {
        let mut latency = self.latency_generator.write().await;
        fluctuate_latency(&mut *latency, current_time);
    }
}
