use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use rand::Rng;
use tokio::sync::RwLock;

use crate::{
    common_infrastructure::{
        event::ClientOrderId,
        instrument::Instrument,
        order::{Open, Order, OrderId, OrderKind, OrderRole, Pending, RequestOpen},
        Side,
    },
    error::ExecutionError,
    sandbox::{
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
    pub fn ins_orders_mut(&mut self, instrument: &Instrument) -> Result<&mut InstrumentOrders, ExecutionError>
    {
        self.instrument_orders_map
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::SandBox(format!("SandBoxExchange 没有为 Instrument: {instrument} 配置")))
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

    // 从PendingRegistry中删除订单的函数
    pub fn remove_order_from_pending_registry(&mut self, order_id: ClientOrderId) -> Result<(), ExecutionError>
    {
        // 假设你有方法来找到并删除PendingRegistry中的订单
        // 这里只是一个简单的示例
        if let Some(index) = self.pending_registry.iter().position(|x| x.cid == order_id) {
            self.pending_registry.remove(index);
            Ok(())
        }
        else {
            Err(ExecutionError::OrderNotFound(order_id))
        }
    }

    pub async fn process_request_as_pending(&mut self, order: Order<RequestOpen>) -> Order<Pending>
    {
        // turn the request into an pending order with a predicted timestamp
        let latency = self.get_random_latency();
        let adjusted_client_ts = order.client_ts + latency;
        let pending = Order { kind: order.kind,
                              exchange: order.exchange,
                              instrument: order.instrument,
                              cid: order.cid,
                              client_ts: order.client_ts,
                              side: order.side,
                              state: Pending { reduce_only: order.state.reduce_only,
                                               price: order.state.price,
                                               size: order.state.size,
                                               predicted_ts: adjusted_client_ts } };
        self.pending_registry.push(pending.clone());
        pending
    }

    pub async fn keep_new_pending_order(&mut self, request: Order<RequestOpen>) -> Result<(), ExecutionError>
    {
        // 检查请求是否有效 NOTE 这里或许可以添加更多的验证逻辑
        if self.pending_registry.iter().any(|pending| pending.cid == request.cid) {
            return Err(ExecutionError::OrderAlreadyExists(request.cid));
        }

        // 尝试转换请求为挂起订单
        let pending_order = self.process_request_as_pending(request).await;

        // 将挂起订单添加到注册表
        self.pending_registry.push(pending_order.clone());

        // 返回成功结果
        Ok(())
    }

    // 判断是Maker还是Taker单
    pub fn determine_maker_taker(&mut self, order: &Order<Pending>, current_price: f64) -> Result<OrderRole, ExecutionError>
    {
        match order.kind {
            | OrderKind::Market => Ok(OrderRole::Taker),
            | OrderKind::Limit => match order.side {
                | Side::Buy => {
                    // 对于买单，限价单的价格应高于或等于当前价格才为Maker
                    if order.state.price >= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        Ok(OrderRole::Taker)
                    }
                }
                | Side::Sell => {
                    // 对于卖单，限价单的价格应低于或等于当前价格才为Maker
                    if order.state.price <= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        Ok(OrderRole::Taker)
                    }
                }
            },
            // PostOnly: 只能作为挂单进入市场。如果无法作为挂单（即成为 Taker），则订单会被取消。
            | OrderKind::PostOnly => match order.side {
                | Side::Buy => {
                    // PostOnly订单如果无法作为挂单（即成为Taker），则被取消
                    if order.state.price >= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        self.remove_order_from_pending_registry(order.cid)?; // 处理删除操作
                        Err(ExecutionError::OrderRejected("PostOnly order rejected".into()))
                    }
                }
                | Side::Sell => {
                    if order.state.price <= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        self.remove_order_from_pending_registry(order.cid)?; // 处理删除操作
                        Err(ExecutionError::OrderRejected("PostOnly order rejected".into()))
                    }
                }
            },
            | OrderKind::ImmediateOrCancel => Ok(OrderRole::Taker), // IOC订单总是作为Taker
            | OrderKind::FillOrKill => Ok(OrderRole::Taker),        // FOK订单总是作为Taker
            | OrderKind::GoodTilCancelled => match order.side {
                | Side::Buy => {
                    // GTC订单和Limit订单相似
                    if order.state.price >= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        Ok(OrderRole::Taker)
                    }
                }
                | Side::Sell => {
                    if order.state.price <= current_price {
                        Ok(OrderRole::Maker)
                    }
                    else {
                        Ok(OrderRole::Taker)
                    }
                }
            },
        }
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub async fn build_order_open(&mut self, request: Order<Pending>, role: OrderRole) -> Order<Open>
    {
        self.increment_request_counter();

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
                              received_ts: request.state.predicted_ts,
                              order_role: role } }
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
