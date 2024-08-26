use crate::{
    common_infrastructure::{
        event::ClientOrderId,
        instrument::Instrument,
        order::{Open, Order, OrderId, OrderExecutionType, OrderRole, Pending, RequestOpen},
        Side,
    },
    error::ExecutionError,
    sandbox::{
        account::account_latency::{fluctuate_latency, AccountLatency},
        instrument_orders::InstrumentOrders,
    },
};
use dashmap::{mapref::one::RefMut, DashMap};
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Debug)]
pub struct AccountOrders
{
    pub latency_generator: AccountLatency,
    pub selectable_latencies: [i64; 20],
    pub request_counter: AtomicU64,                               // 用来生成一个唯一的 [`OrderId`]。
    pub pending_registry: DashMap<ClientOrderId, Order<Pending>>, // 使用 HashMap
    pub instrument_orders_map: DashMap<Instrument, InstrumentOrders>,
}

impl AccountOrders
{
    /// 从给定的 [`Instrument`] 列表选择构造一个新的 [`AccountOrders`]。
    pub async fn new(instruments: Vec<Instrument>, mut account_latency: AccountLatency) -> Self
    {
        let selectable_latencies = Self::generate_latencies(&mut account_latency).await;

        Self { request_counter: AtomicU64::new(0),
               pending_registry: DashMap::new(),
               instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect(),
               latency_generator: account_latency,
               selectable_latencies }
    }

    async fn generate_latencies(latency_generator: &mut AccountLatency) -> [i64; 20]
    {
        let mut latencies = [0; 20];
        for latency in &mut latencies {
            fluctuate_latency(latency_generator, 0); // 这里0只是一个占位，可以根据需求调整
            *latency = latency_generator.current_value;
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

    pub fn ins_orders_mut(&mut self, instrument: &Instrument) -> Result<RefMut<Instrument, InstrumentOrders>, ExecutionError>
    {
        self.instrument_orders_map
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::SandBox(format!("Sandbox exchange is not configured for Instrument: {instrument}")))
    }

    /// 为每个 [`Instrument`] 获取出价和要价 [`Order<Open>`]。
    pub fn fetch_all(&self) -> Vec<Order<Open>>
    {
        self.instrument_orders_map
            .iter()
            .flat_map(|entry| {
                let orders = entry.value();
                orders.bids.iter().chain(orders.asks.iter()).cloned().collect::<Vec<_>>()
            })
            .collect()
    }

    // 从PendingRegistry中删除订单的函数
    pub fn remove_order_from_pending_registry(&mut self, order_id: ClientOrderId) -> Result<(), ExecutionError>
    {
        if self.pending_registry.remove(&order_id).is_some() {
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
        Order { kind: order.kind,
                exchange: order.exchange,
                instrument: order.instrument,
                client_order_id: order.client_order_id,
                client_ts: order.client_ts,
                side: order.side,
                state: Pending { reduce_only: order.state.reduce_only,
                                 price: order.state.price,
                                 size: order.state.size,
                                 predicted_ts: adjusted_client_ts } }
    }

    pub async fn register_pending_order(&mut self, request: Order<RequestOpen>) -> Result<(), ExecutionError>
    {
        if self.pending_registry.contains_key(&request.client_order_id) {
            return Err(ExecutionError::OrderAlreadyExists(request.client_order_id));
        }
        let pending_order = self.process_request_as_pending(request.clone()).await;
        self.pending_registry.insert(request.client_order_id, pending_order);
        Ok(())
    }

    // 判断是Maker还是Taker单
    /// 确定订单是 Maker 还是 Taker
    pub fn determine_maker_taker(&mut self, order: &Order<Pending>, current_price: f64) -> Result<OrderRole, ExecutionError>
    {
        match order.kind {
            | OrderExecutionType::Market => Ok(OrderRole::Taker), // 市场订单总是 Taker

            | OrderExecutionType::Limit => self.determine_limit_order_role(order, current_price), // 限价订单的判断逻辑

            | OrderExecutionType::PostOnly => self.determine_post_only_order_role(order, current_price), // 仅挂单的判断逻辑

            | OrderExecutionType::ImmediateOrCancel | OrderExecutionType::FillOrKill => Ok(OrderRole::Taker), // 立即成交或取消的订单总是 Taker

            | OrderExecutionType::GoodTilCancelled => self.determine_limit_order_role(order, current_price), // GTC订单与限价订单处理类似
        }
    }

    /// 判断限价订单是 Maker 还是 Taker
    fn determine_limit_order_role(&self, order: &Order<Pending>, current_price: f64) -> Result<OrderRole, ExecutionError>
    {
        match order.side {
            | Side::Buy => {
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
        }
    }

    /// 判断 PostOnly 订单是否符合条件，如果不符合则删除并返回错误
    fn determine_post_only_order_role(&mut self, order: &Order<Pending>, current_price: f64) -> Result<OrderRole, ExecutionError>
    {
        match order.side {
            | Side::Buy => {
                if order.state.price >= current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    self.reject_post_only_order(order)
                }
            }
            | Side::Sell => {
                if order.state.price <= current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    self.reject_post_only_order(order)
                }
            }
        }
    }

    /// 拒绝不符合条件的 PostOnly 订单并移除
    fn reject_post_only_order(&mut self, order: &Order<Pending>) -> Result<OrderRole, ExecutionError>
    {
        self.remove_order_from_pending_registry(order.client_order_id)?; // 移除订单
        Err(ExecutionError::OrderRejected("PostOnly order rejected".into())) // 返回拒绝错误
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
                client_order_id: request.client_order_id,
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

    pub fn update_latency(&mut self, current_time: i64)
    {
        fluctuate_latency(&mut self.latency_generator, current_time);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common_infrastructure::instrument::{kind::InstrumentKind, Instrument},
        sandbox::account::account_latency::{AccountLatency, FluctuationMode},
        Exchange,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_new_account_orders()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot), Instrument::new("ETH", "USD", InstrumentKind::Spot),];

        // 手动创建一个 AccountLatency 实例
        let account_latency = AccountLatency::new(FluctuationMode::Sine, // 设置波动模式
                                                  100,                   // 设置最大延迟
                                                  10                     /* 设置最小延迟 */);

        let account_orders = AccountOrders::new(instruments.clone(), account_latency).await;

        assert_eq!(account_orders.request_counter.load(Ordering::Acquire), 0);
        assert_eq!(account_orders.instrument_orders_map.len(), instruments.len());
        assert!(account_orders.pending_registry.is_empty());
    }

    #[tokio::test]
    async fn test_generate_latencies()
    {
        let account_latency = AccountLatency::new(FluctuationMode::NormalDistribution, 100, 10);

        let latency_generator = Arc::new(RwLock::new(account_latency));

        // 获取可变引用
        let mut latency_generator = latency_generator.write().await;

        // 传递给 generate_latencies 函数
        let latencies = AccountOrders::generate_latencies(&mut latency_generator).await;

        assert_eq!(latencies.len(), 20);
        for latency in &latencies {
            assert!(*latency >= 10 && *latency <= 100);
        }
    }

    #[tokio::test]
    async fn test_get_random_latency()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Uniform, 100, 10);

        let account_orders = AccountOrders::new(instruments, account_latency).await;

        let latency = account_orders.get_random_latency();
        assert!(latency >= 10 && latency <= 100);
    }

    #[tokio::test]
    async fn test_ins_orders_mut()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::LinearIncrease, 100, 10);

        let mut account_orders = AccountOrders::new(instruments.clone(), account_latency).await;

        {
            // 创建一个作用域，使用完 `result` 后自动释放它
            let result = account_orders.ins_orders_mut(&instruments[0]);
            assert!(result.is_ok());
        } // `result` 在这里被释放

        let invalid_instrument = Instrument::new("INVALID", "USD", InstrumentKind::Spot);
        let invalid_result = account_orders.ins_orders_mut(&invalid_instrument);
        assert!(invalid_result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_all()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::LinearDecrease, 100, 10);

        let account_orders = AccountOrders::new(instruments, account_latency).await;

        let orders = account_orders.fetch_all();
        assert!(orders.is_empty());
    }
    #[tokio::test]
    async fn test_remove_order_from_pending_registry()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::StepFunction, 100, 10);
        // 使用特定的UUID来创建一个ClientOrderId实例，模拟一个不存在的订单
        let client_order_id = ClientOrderId(Uuid::from_u128(999));

        let mut account_orders = AccountOrders::new(instruments, account_latency).await;

        let order = Order { kind: OrderExecutionType::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            client_order_id,
                            client_ts: 0,
                            side: Side::Buy,
                            state: Pending { reduce_only: false,
                                             price: 50.0,
                                             size: 1.0,
                                             predicted_ts: 0 } };

        account_orders.pending_registry.insert(order.client_order_id, order.clone()); // 使用 insert 方法
        let remove_result = account_orders.remove_order_from_pending_registry(order.client_order_id);
        assert!(remove_result.is_ok());
        assert!(account_orders.pending_registry.is_empty());

        let remove_invalid_result = account_orders.remove_order_from_pending_registry(client_order_id);
        assert!(remove_invalid_result.is_err());
    }
    #[tokio::test]
    async fn test_process_request_as_pending()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::RandomWalk, 100, 10);

        let client_order_id = ClientOrderId(Uuid::from_u128(999));

        let mut account_orders = AccountOrders::new(instruments, account_latency).await;

        let request_order = Order { kind: OrderExecutionType::Limit,
                                    exchange: Exchange::SandBox,
                                    instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                                    client_order_id,
                                    client_ts: 1000,
                                    side: Side::Buy,
                                    state: RequestOpen { reduce_only: false,
                                                         price: 50.0,
                                                         size: 1.0 } };

        let pending_order = account_orders.process_request_as_pending(request_order).await;
        assert_eq!(pending_order.client_order_id, client_order_id);
        assert!(pending_order.state.predicted_ts > 1000);
    }

    #[tokio::test]
    async fn test_register_pending_order()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let client_order_id = ClientOrderId(Uuid::from_u128(999));

        let mut account_orders = AccountOrders::new(instruments, account_latency).await;

        let request_order = Order { kind: OrderExecutionType::Limit,
                                    exchange: Exchange::SandBox,
                                    instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                                    client_order_id,
                                    client_ts: 1000,
                                    side: Side::Buy,
                                    state: RequestOpen { reduce_only: false,
                                                         price: 50.0,
                                                         size: 1.0 } };

        let result = account_orders.register_pending_order(request_order.clone()).await;
        assert!(result.is_ok());
        assert_eq!(account_orders.pending_registry.len(), 1);

        let duplicate_result = account_orders.register_pending_order(request_order).await;
        assert!(duplicate_result.is_err());
    }

    #[tokio::test]
    async fn test_determine_maker_taker()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::None, 100, 10);
        let mut account_orders = AccountOrders::new(instruments, account_latency).await;

        let order = Order { kind: OrderExecutionType::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            client_order_id: ClientOrderId(Uuid::new_v4()),
                            client_ts: 1000,
                            side: Side::Buy,
                            state: Pending { reduce_only: false,
                                             price: 50.0,
                                             size: 1.0,
                                             predicted_ts: 1000 } };

        let role_maker = account_orders.determine_maker_taker(&order, 50.0).unwrap();
        assert_eq!(role_maker, OrderRole::Maker);

        let role_taker = account_orders.determine_maker_taker(&order, 60.0).unwrap();
        assert_eq!(role_taker, OrderRole::Taker);
    }

    #[tokio::test]
    async fn test_increment_request_counter()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::None, 100, 10);
        let account_orders = AccountOrders::new(instruments, account_latency).await;

        assert_eq!(account_orders.request_counter.load(Ordering::Acquire), 0);
        account_orders.increment_request_counter();
        assert_eq!(account_orders.request_counter.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn test_order_id()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::None, 100, 10);
        let account_orders = AccountOrders::new(instruments, account_latency).await;

        let first_order_id = account_orders.order_id();
        assert_eq!(first_order_id, OrderId("0".to_string()));

        account_orders.increment_request_counter();
        let second_order_id = account_orders.order_id();
        assert_eq!(second_order_id, OrderId("1".to_string()));
    }

    #[tokio::test]
    async fn test_update_latency()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let mut account_orders = AccountOrders::new(instruments, account_latency).await;

        account_orders.update_latency(1000);

        let latency = account_orders.latency_generator;
        assert!(latency.current_value >= 10 && latency.current_value <= 100);
    }
}
