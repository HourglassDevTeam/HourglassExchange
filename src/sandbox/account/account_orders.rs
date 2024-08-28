use crate::{
    common::{
        instrument::Instrument,
        order::{
            identification::{ request_order_id::RequestId, OrderId},
            order_instructions::OrderInstruction,
            states::{open::Open, pending::Pending, request_open::RequestOpen},
            Order, OrderRole,
        },
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
    pub machine_id: u64,
    pub latency_generator: AccountLatency,
    pub selectable_latencies: [i64; 20],
    pub request_counter: AtomicU64,
    pub order_counter: AtomicU64,
    pub pending_order_registry: DashMap<RequestId, Order<Pending>>,
    pub instrument_orders_map: DashMap<Instrument, InstrumentOrders>,
}

impl AccountOrders
{
    /// 从给定的 [`Instrument`] 列表选择构造一个新的 [`AccountOrders`]。
    /// 创建一个新的 [`AccountOrders`] 实例。
    ///
    /// 该函数接受一组预先定义的金融工具（`Instrument`）和一个账户延迟生成器（`AccountLatency`），
    /// 并返回一个初始化的 `AccountOrders` 实例，用于管理这些金融工具的订单和延迟模拟。
    ///
    /// # 参数
    ///
    /// * `instruments` - 一个 `Vec<Instrument>` 类型的列表，包含所有需要管理的金融工具。
    /// * `account_latency` - 一个 `AccountLatency` 实例，用于生成和管理请求延迟的波动情况。
    ///
    /// # 返回值
    ///
    /// 返回一个初始化好的 `AccountOrders` 实例，其中包含了给定的金融工具、
    /// 生成的延迟值数组，以及用于管理挂单的相关结构。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use unilink_execution::{
    ///     common::instrument::{kind::InstrumentKind, Instrument},
    ///     sandbox::account::{
    ///         account_latency::{AccountLatency, FluctuationMode},
    ///         account_orders::AccountOrders,
    ///     },
    /// };
    ///
    /// #[tokio::main]
    /// async fn main()
    /// {
    ///     let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
    ///     let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
    ///     let account_orders = AccountOrders::new(123124124124, instruments, account_latency).await;
    ///     println!("新建的 AccountOrders 实例: {:?}", account_orders);
    /// }
    /// ```
    pub async fn new(machine_id: u64, instruments: Vec<Instrument>, mut account_latency: AccountLatency) -> Self
    {
        let selectable_latencies = Self::generate_latencies(&mut account_latency).await;

        Self { machine_id,
               order_counter: AtomicU64::new(0),
               request_counter: AtomicU64::new(0),
               pending_order_registry: DashMap::new(),
               instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, InstrumentOrders::default())).collect(),
               latency_generator: account_latency,
               selectable_latencies }
    }

    /// 生成一个新的 `RequestId`
    ///
    /// # 参数
    ///
    /// - `machine_id`: 用于标识生成 ID 的机器，最大值为 1023。
    /// - If the machine ID is represented as a 64-bit unsigned integer (u64).
    /// - This number equals 18,446,744,073,709,551,616, which is over 18 quintillion unique machine IDs.
    ///
    /// # 返回值
    ///
    /// 返回一个唯一的 `RequestId`。
    /// NOTE that the client's login PC might change frequently. This method is not web-compatible now.
    pub fn generate_request_id(&self) -> RequestId
    {
        let counter = self.request_counter.fetch_add(1, Ordering::SeqCst);
        RequestId::new(self.machine_id, counter)
    }

    /// 生成一组预定义的延迟值数组，用于模拟订单延迟。
    ///
    /// 该函数通过调用 `fluctuate_latency` 函数来动态调整延迟值，并将结果存储在一个数组中。
    ///
    /// # 参数
    ///
    /// * `latency_generator` - 一个可变引用，指向 `AccountLatency` 实例，用于生成和调整延迟值。
    ///
    /// # 返回值
    ///
    /// 返回一个包含 20 个延迟值的数组 `[i64; 20]`，每个延迟值是通过 `AccountLatency` 计算得到的。
    async fn generate_latencies(latency_generator: &mut AccountLatency) -> [i64; 20]
    {
        let mut latencies = [0; 20];
        for latency in &mut latencies {
            fluctuate_latency(latency_generator, 0); // NOTE 这里的占位符 0 可能需要调整。
            *latency = latency_generator.current_value;
        }
        latencies
    }

    /// 从预定义的延迟值数组中随机选择一个延迟值。
    ///
    /// 该函数用于从 `selectable_latencies` 数组中随机选择一个延迟值，
    /// 用于模拟不同请求的延迟情况，增强测试或模拟的真实性。
    ///
    /// # 返回值
    ///
    /// 返回一个随机选择的延迟值 `i64`。
    fn get_random_latency(&self) -> i64
    {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.selectable_latencies.len());
        self.selectable_latencies[idx]
    }

    /// 返回指定 [`Instrument`] 的客户端 [`InstrumentOrders`] 的可变引用。

    pub fn get_ins_orders_mut(&mut self, instrument: &Instrument) -> Result<RefMut<Instrument, InstrumentOrders>, ExecutionError>
    {
        self.instrument_orders_map
            .get_mut(instrument)
            .ok_or_else(|| ExecutionError::SandBox(format!("Sandbox exchange is not configured for Instrument: {instrument}")))
    }

    /// 为每个 [`Instrument`] 获取出价和要价 [`Order<Open>`]。
    ///
    /// 该函数在以下情况下会被使用:
    ///
    /// 1. **查询订单状态**: 用户或系统需要查询当前账户的所有挂单，例如在界面上显示当前的挂单状态，或在数据分析时，了解有哪些订单还未成交。
    /// 2. **取消所有挂单**: 在需要一次性取消所有挂单的情况下，首先可以通过 `fetch_all()` 获取所有挂单的列表，然后逐一取消这些订单。
    /// 3. **定期检查或清理**: 系统可能会定期检查账户的挂单情况，确保所有挂单都在合理状态，或者在清理过程中使用该函数获取需要清理的订单。
    /// 4. **系统恢复或重启后重建状态**: 如果交易系统因某种原因重启，系统可能需要重建账户的内部状态，此时 `fetch_all()` 可以用于获取所有挂单，以便在内存中重新建立订单簿的状态。
    /// 5. **监控和日志记录**: 在监控或日志记录系统中，记录当前账户所有挂单的状态，有助于在出问题时追踪系统中未成交订单的详细信息。
    ///
    /// # 返回值
    ///
    /// 返回一个包含所有未完成订单的 `Vec<Order<Open>>`。
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

    /// 从 `pending_registry` 中删除指定的订单。
    ///
    /// # 参数
    ///
    /// - `request_id`: 要删除的订单的 `RequestId`。
    ///
    /// # 返回值
    ///
    /// - 如果删除成功，返回 `Ok(())`。
    /// - 如果未找到订单，返回 `Err(ExecutionError::OrderNotFound)`。
    pub fn remove_order_from_pending_registry(&mut self, request_id: RequestId) -> Result<(), ExecutionError>
    {
        if self.pending_order_registry.remove(&request_id).is_some() {
            Ok(())
        }
        else {
            Err(ExecutionError::RequestNotFound(request_id))
        }
    }

    /// 将请求转换为一个待处理的订单 (`Pending`)。
    ///
    /// # 参数
    ///
    /// - `order`: 要处理的订单请求 (`Order<RequestOpen>`)。
    ///
    /// # 返回值
    ///
    /// - 返回一个包含预测时间戳的待处理订单 (`Order<Pending>`)。
    pub async fn process_request_as_pending(&mut self, order: Order<RequestOpen>) -> Order<Pending> {
        // 生成一个新的 RequestId
        let request_id = self.generate_request_id();

        // 从预定义的延迟值数组中选择一个延迟值
        let latency = self.get_random_latency();
        let adjusted_client_ts = order.client_ts + latency;

        // 创建并返回新的 Pending 订单
        Order {
            kind: order.kind,
            exchange: order.exchange,
            instrument: order.instrument,
            cid: order.cid,
            client_ts: order.client_ts,
            side: order.side,
            state: Pending {
                reduce_only: order.state.reduce_only,
                price: order.state.price,
                size: order.state.size,
                predicted_ts: adjusted_client_ts,
                request_id, // 分配生成的 RequestId
            },
        }
    }

    /// 将请求注册为待处理订单。
    ///
    /// # 参数
    ///
    /// - `request`: 要注册的订单请求 (`Order<RequestOpen>`)。
    ///
    /// # 返回值
    ///
    /// - 如果订单成功注册，返回 `Ok(())`。
    /// - 如果订单已存在，返回 `Err(ExecutionError::OrderAlreadyExists)`。
    pub async fn register_pending_order(&mut self, request: Order<RequestOpen>) -> Result<(), ExecutionError> {
        // Check if an entry with this ClientOrderId already exists
        if self.pending_order_registry.iter().any(|entry| entry.value().cid == request.cid) {
            return Err(ExecutionError::OrderAlreadyExists(request.cid));
        }

        // Process the request to create a pending order
        let pending_order = self.process_request_as_pending(request.clone()).await;
        self.pending_order_registry.insert(pending_order.state.request_id, pending_order);
        Ok(())
    }

    /// 根据订单类型和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的订单 (`Order<Pending>`)。
    /// - `current_price`: 当前市场价格。
    ///
    /// # 返回值
    ///
    /// - 返回 `Ok(OrderRole::Maker)` 或 `Ok(OrderRole::Taker)`，分别表示订单是 Maker 或 Taker。
    /// - 如果订单类型无法判断，返回 `Err(ExecutionError)`。
    ///
    /// # 逻辑
    ///
    /// - 对于 `Market` 类型的订单，总是返回 `OrderRole::Taker`，因为`Market`订单总是`Taker`订单。
    /// - 对于 `Limit` 类型的订单，调用 `determine_limit_order_role` 来确定订单角色。
    /// - 对于 `PostOnly` 类型的订单，调用 `determine_post_only_order_role` 来判断订单是否能作为 Maker，否则拒绝该订单。
    /// - 对于 `ImmediateOrCancel` 和 `FillOrKill` 类型的订单，总是返回 `OrderRole::Taker`，因为这些订单需要立即成交。
    /// - 对于 `GoodTilCancelled` 类型的订单，按照限价订单的逻辑来判断角色。
    pub fn determine_maker_taker(&mut self, order: &Order<Pending>, current_price: f64) -> Result<OrderRole, ExecutionError>
    {
        match order.kind {
            | OrderInstruction::Market => Ok(OrderRole::Taker), // 市场订单总是 Taker

            | OrderInstruction::Limit => self.determine_limit_order_role(order, current_price), // 限价订单的判断逻辑

            | OrderInstruction::PostOnly => self.determine_post_only_order_role(order, current_price), // 仅挂单的判断逻辑

            | OrderInstruction::ImmediateOrCancel | OrderInstruction::FillOrKill => Ok(OrderRole::Taker), // 立即成交或取消的订单总是 Taker

            | OrderInstruction::GoodTilCancelled => self.determine_limit_order_role(order, current_price), // GTC订单与限价订单处理类似
        }
    }

    /// 根据限价订单的价格和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的限价订单 (`Order<Pending>`)。
    /// - `current_price`: 当前市场价格。
    ///
    /// # 返回值
    ///
    /// - `Ok(OrderRole::Maker)`: 如果订单价格与当前市场价格相比，具有优势（买单价格高于或等于市场价格，或卖单价格低于或等于市场价格），则订单作为 Maker 角色。
    /// - `Ok(OrderRole::Taker)`: 如果订单价格与当前市场价格相比，处于劣势（买单价格低于市场价格，或卖单价格高于市场价格），则订单作为 Taker 角色。
    ///
    /// # 逻辑
    ///
    /// - 对于买单 (`Side::Buy`):
    ///   - 如果订单价格 (`order.state.price`) 大于或等于当前市场价格 (`current_price`)，则返回 `OrderRole::Maker`。
    ///   - 否则，返回 `OrderRole::Taker`。
    ///
    /// - 对于卖单 (`Side::Sell`):
    ///   - 如果订单价格 (`order.state.price`) 小于或等于当前市场价格 (`current_price`)，则返回 `OrderRole::Maker`。
    ///   - 否则，返回 `OrderRole::Taker`。
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

    /// 判断 PostOnly 订单是否符合条件，并确定其是 Maker 还是被拒绝。
    ///
    /// 如果订单不符合 PostOnly 的条件（即买单价格低于当前市场价格，或卖单价格高于当前市场价格），
    /// 则会拒绝该订单，并将其从待处理订单中删除。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的 PostOnly 订单 (`Order<Pending>`)。
    /// - `current_price`: 当前市场价格。
    ///
    /// # 返回值
    ///
    /// - `Ok(OrderRole::Maker)`: 如果订单价格符合 PostOnly 的条件，
    ///   即买单价格高于或等于市场价格，或者卖单价格低于或等于市场价格，则订单作为 Maker 角色。
    /// - `Err(ExecutionError::OrderRejected)`: 如果订单价格不符合 PostOnly 的条件，
    ///   即买单价格低于市场价格，或者卖单价格高于市场价格，则订单会被拒绝并从待处理订单中删除。
    ///
    /// # 逻辑
    ///
    /// - 对于买单 (`Side::Buy`):
    ///   - 如果订单价格 (`order.state.price`) 大于或等于当前市场价格 (`current_price`)，则返回 `OrderRole::Maker`。
    ///   - 否则，调用 `self.reject_post_only_order(order)` 拒绝订单，并返回错误。
    ///
    /// - 对于卖单 (`Side::Sell`):
    ///   - 如果订单价格 (`order.state.price`) 小于或等于当前市场价格 (`current_price`)，则返回 `OrderRole::Maker`。
    ///   - 否则，调用 `self.reject_post_only_order(order)` 拒绝订单，并返回错误。
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

    /// 拒绝不符合条件的 PostOnly 订单，并将其从待处理订单注册表中移除。
    ///
    /// 当一个 PostOnly 订单的价格不符合条件时（例如，买单价格低于市场价格，或卖单价格高于市场价格），
    /// 该函数将拒绝此订单，并将其从 `pending_registry` 中删除。
    ///
    /// # 参数
    ///
    /// - `order`: 待拒绝的 PostOnly 订单 (`Order<Pending>`)。
    ///
    /// # 返回值
    fn reject_post_only_order(&mut self, order: &Order<Pending>) -> Result<OrderRole, ExecutionError>
    {
        self.remove_order_from_pending_registry(order.state.request_id)?; // 移除订单
        Err(ExecutionError::OrderRejected("PostOnly order rejected".into())) // 返回拒绝错误
    }

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub async fn build_order_open(&mut self, request: Order<Pending>, role: OrderRole) -> Order<Open>
    {
        self.increment_order_counter();

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

    /// 增加请求计数器的值。
    ///
    /// 该函数使用 [`Ordering::Relaxed`] 来递增请求计数器 `request_counter` 的值，
    /// 不保证线程同步的顺序一致性。这意味着多个线程可以并发调用此函数，
    /// 但不保证对其他线程的立即可见性或顺序一致性。
    ///
    /// # 注意
    ///
    /// - 此函数的主要用途是在每次接收到新的订单请求时递增计数器，以确保订单 ID 的唯一性。
    /// - 由于使用了 `Relaxed` 顺序，这种递增操作的结果可能对其他线程不可见。
    pub fn increment_order_counter(&self)
    {
        self.order_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// 在 order_id 方法中，使用 [Ordering::Acquire] 确保读取到最新的计数器值。
    pub fn order_id(&self) -> OrderId {
        let counter = self.order_counter.fetch_add(1, Ordering::SeqCst);
        OrderId::new(self.machine_id, counter)
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
        common::instrument::{kind::InstrumentKind, Instrument},
        sandbox::account::account_latency::{AccountLatency, FluctuationMode},
        Exchange,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_new_account_orders()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot), Instrument::new("ETH", "USD", InstrumentKind::Spot),];

        // 手动创建一个 AccountLatency 实例
        let account_latency = AccountLatency::new(FluctuationMode::Sine, // 设置波动模式
                                                  100,                   // 设置最大延迟
                                                  10                     /* 设置最小延迟 */);

        let account_orders = AccountOrders::new(999, instruments.clone(), account_latency).await;

        assert_eq!(account_orders.order_counter.load(Ordering::Acquire), 0);
        assert_eq!(account_orders.instrument_orders_map.len(), instruments.len());
        assert!(account_orders.pending_order_registry.is_empty());
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

        let account_orders = AccountOrders::new(123, instruments, account_latency).await;

        let latency = account_orders.get_random_latency();
        assert!(latency >= 10 && latency <= 100);
    }

    #[tokio::test]
    async fn test_ins_orders_mut()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::LinearIncrease, 100, 10);

        let mut account_orders = AccountOrders::new(123124, instruments.clone(), account_latency).await;

        {
            // 创建一个作用域，使用完 `result` 后自动释放它
            let result = account_orders.get_ins_orders_mut(&instruments[0]);
            assert!(result.is_ok());
        } // `result` 在这里被释放

        let invalid_instrument = Instrument::new("INVALID", "USD", InstrumentKind::Spot);
        let invalid_result = account_orders.get_ins_orders_mut(&invalid_instrument);
        assert!(invalid_result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_all()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::LinearDecrease, 100, 10);

        let account_orders = AccountOrders::new(1231, instruments, account_latency).await;

        let orders = account_orders.fetch_all();
        assert!(orders.is_empty());
    }
    #[tokio::test]
    async fn test_remove_order_from_pending_registry()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::StepFunction, 100, 10);
        let client_order_id = crate::common::order::identification::client_order_id::ClientOrderId(Option::from("OJBK".to_string()));
        let mut account_orders = AccountOrders::new(12341234, instruments, account_latency).await;

        let order = Order { kind: OrderInstruction::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: client_order_id.clone(), // Clone here to retain ownership
                            client_ts: 0,
                            side: Side::Buy,
                            state: Pending { reduce_only: false,
                                             price: 50.0,
                                             size: 1.0,
                                             predicted_ts: 0,
                                request_id: RequestId(34534),
                            } };

        account_orders.pending_order_registry.insert(order.state.request_id.clone(), order.clone()); // Clone here as well
        let remove_result = account_orders.remove_order_from_pending_registry(order.state.request_id);
        assert!(remove_result.is_ok());
        assert!(account_orders.pending_order_registry.is_empty());

        let remove_invalid_result = account_orders.remove_order_from_pending_registry(order.state.request_id);
        assert!(remove_invalid_result.is_err());
    }

    #[tokio::test]
    async fn test_process_request_as_pending()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::RandomWalk, 100, 10);

        let client_order_id = crate::common::order::identification::client_order_id::ClientOrderId(Option::from("OJBK".to_string()));

        let mut account_orders = AccountOrders::new(123124, instruments, account_latency).await;

        let request_order = Order { kind: OrderInstruction::Limit,
                                    exchange: Exchange::SandBox,
                                    instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                                    cid: client_order_id.clone(),
                                    client_ts: 1000,
                                    side: Side::Buy,
                                    state: RequestOpen { reduce_only: false,
                                                         price: 50.0,
                                                         size: 1.0 } };

        let pending_order = account_orders.process_request_as_pending(request_order).await;
        assert_eq!(pending_order.cid, client_order_id);
        assert!(pending_order.state.predicted_ts > 1000);
    }

    #[tokio::test]
    async fn test_register_pending_order()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let client_order_id = crate::common::order::identification::client_order_id::ClientOrderId(Option::from("OJBK".to_string()));

        let mut account_orders = AccountOrders::new(234523, instruments, account_latency).await;

        let request_order = Order { kind: OrderInstruction::Limit,
                                    exchange: Exchange::SandBox,
                                    instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                                    cid: client_order_id,
                                    client_ts: 1000,
                                    side: Side::Buy,
                                    state: RequestOpen { reduce_only: false,
                                                         price: 50.0,
                                                         size: 1.0 } };

        let result = account_orders.register_pending_order(request_order.clone()).await;
        assert!(result.is_ok());
        assert_eq!(account_orders.pending_order_registry.len(), 1);

        let duplicate_result = account_orders.register_pending_order(request_order).await;
        assert!(duplicate_result.is_err());
    }

    #[tokio::test]
    async fn test_determine_maker_taker()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::None, 100, 10);
        let mut account_orders = AccountOrders::new(2351235, instruments, account_latency).await;

        let order = Order { kind: OrderInstruction::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: crate::common::order::identification::client_order_id::ClientOrderId(Option::from("OJBK".to_string())),
                            client_ts: 1000,
                            side: Side::Buy,
                            state: Pending { reduce_only: false,
                                             price: 50.0,
                                             size: 1.0,
                                             predicted_ts: 1000,
                                request_id: RequestId(435),
                            } };

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
        let account_orders = AccountOrders::new(09890, instruments, account_latency).await;

        assert_eq!(account_orders.order_counter.load(Ordering::Acquire), 0);
        account_orders.increment_order_counter();
        assert_eq!(account_orders.order_counter.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn test_order_id() {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::None, 100, 10);
        let account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        let first_order_id = account_orders.order_id();
        let second_order_id = {
            account_orders.increment_order_counter();
            account_orders.order_id()
        };

        // 获取前 51 位 (即 [timestamp:41 bits] [machine_id:10 bits]) 的值
        let first_order_id_prefix = first_order_id.value() >> 13;
        let second_order_id_prefix = second_order_id.value() >> 13;

        assert_eq!(first_order_id_prefix, second_order_id_prefix);
        assert!(second_order_id > first_order_id);
    }

    #[tokio::test]
    async fn test_update_latency()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let mut account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        account_orders.update_latency(1000);

        let latency = account_orders.latency_generator;
        assert!(latency.current_value >= 10 && latency.current_value <= 100);
    }
}
