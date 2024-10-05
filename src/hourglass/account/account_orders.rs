use crate::{
    common::{
        instrument::Instrument,
        order::{
            identification::{machine_id::generate_machine_id, OrderId},
            order_instructions::OrderInstruction,
            states::{open::Open, request_open::RequestOpen},
            Order, OrderRole,
        },
        Side,
    },
    error::ExchangeError,
    hourglass::{
        account::account_latency::{fluctuate_latency, AccountLatency},
        clickhouse_api::datatype::single_level_order_book::SingleLevelOrderBook,
        open_orders_book::OpenOrdersBook,
    },
};
use async_trait::async_trait;
use dashmap::{mapref::one::RefMut, DashMap};
use rand::Rng;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct AccountOrders
{
    pub machine_id: u64,
    pub latency_generator: AccountLatency,
    pub selectable_latencies: [i64; 20],
    pub request_counter: AtomicU64,
    pub order_counter: AtomicU64,
    pub instrument_orders_map: DashMap<Instrument, OpenOrdersBook>,
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
    /// use hourglass::{
    ///     common::instrument::{kind::InstrumentKind, Instrument},
    ///     hourglass::account::{
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
               instrument_orders_map: instruments.into_iter().map(|instrument| (instrument, OpenOrdersBook::default())).collect(),
               latency_generator: account_latency,
               selectable_latencies }
    }

    /// 返回指定 [`Instrument`] 的 [`OpenOrdersBook`] 的可变引用。

    pub fn get_ins_orders_mut(&self, instrument: &Instrument) -> Result<RefMut<Instrument, OpenOrdersBook>, ExchangeError>
    {
        self.instrument_orders_map
            .get_mut(instrument)
            .ok_or_else(|| ExchangeError::Hourglass(format!("Hourglass exchange is not configured for Instrument: {instrument}")))
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

    /// 从提供的 [`Order<RequestOpen>`] 构建一个 [`Order<Open>`]。请求计数器递增，
    /// 在 increment_request_counter 方法中，使用 Ordering::Relaxed 进行递增。
    pub async fn build_order_open(&mut self, request: Order<RequestOpen>, role: OrderRole) -> Order<Open>
    {
        self.increment_order_counter();

        // 直接构建 Order<Open>
        Order { instruction: request.instruction,
                exchange: request.exchange,
                instrument: request.instrument,
                cid: request.cid,
                timestamp: request.timestamp,
                side: request.side,
                state: Open { id: self.order_id(),
                              price: request.state.price,
                              size: request.state.size,
                              filled_quantity: 0.0,
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

    /// 生成一个新的 [OrderId]。
    ///
    /// 该函数根据当前的系统时间戳和订单计数器生成一个唯一的 [OrderId]。
    /// 系统时间戳以毫秒为单位计算，并结合机器 ID 和计数器来确保生成的 ID 唯一。
    /// 由于计数器使用 [Ordering::SeqCst] 进行递增，确保了在多线程环境下的顺序一致性和原子性。
    ///
    /// # 返回值
    ///
    /// 返回一个唯一的 [OrderId]，该 ID 是基于当前的时间戳、机器 ID 和计数器生成的。
    ///
    /// # 错误处理
    ///
    /// 如果系统时间出现倒退，将导致程序崩溃并输出错误信息 "时间出现倒退"。
    pub fn order_id(&self) -> OrderId
    {
        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).expect("时间出现倒退").as_millis() as u64;
        let counter = self.order_counter.fetch_add(1, Ordering::SeqCst);
        OrderId::new(now_ts, self.machine_id, counter)
    }
}
#[async_trait]
impl OrderRoleClassifier for AccountOrders
{
    /// 根据订单类型和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的订单 (`Order<RequestOpen>`)。
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
    fn determine_maker_taker(&self, order: &Order<RequestOpen>, order_book: &SingleLevelOrderBook) -> Result<OrderRole, ExchangeError>
    {
        // 根据订单方向设置 current_price
        let current_price = match order.side {
            | Side::Buy => order_book.latest_ask,  // 买单参考最新卖价
            | Side::Sell => order_book.latest_bid, // 卖单参考最新买价
        };

        match order.instruction {
            | OrderInstruction::Market => Ok(OrderRole::Taker), // 市场订单总是 Taker

            | OrderInstruction::Limit => self.determine_limit_order_role(order, current_price), // 限价订单的判断逻辑

            // note that PostOnly is not affiliated to Limit mode in Hourglass, but regarded as a standalone mode.
            | OrderInstruction::PostOnlyLimit => self.determine_post_only_order_role(order, current_price), // 仅挂单的判断逻辑

            | OrderInstruction::ImmediateOrCancel | OrderInstruction::FillOrKill => {
                let is_immediate = match order.side {
                    | Side::Buy => order.state.price >= current_price,  // 买单判断是否立刻成交
                    | Side::Sell => order.state.price <= current_price, // 卖单判断是否立刻成交
                };

                if is_immediate {
                    Ok(OrderRole::Taker) // 可以立即成交
                }
                else {
                    Ok(OrderRole::Maker) // 不能立即成交
                }
            }

            | OrderInstruction::GoodTilCancelled => self.determine_limit_order_role(order, current_price), // GTC订单与限价订单处理类似

            | OrderInstruction::Cancel => {
                todo!() // 取消订单逻辑
            }
        }
    }

    /// 根据限价订单的价格和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的限价订单 (`Order<RequestOpen>`)。
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
    fn determine_limit_order_role(&self, order: &Order<RequestOpen>, current_price: f64) -> Result<OrderRole, ExchangeError>
    {
        match order.side {
            | Side::Buy => {
                if order.state.price < current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    Ok(OrderRole::Taker)
                }
            }
            | Side::Sell => {
                if order.state.price > current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    Ok(OrderRole::Taker)
                }
            }
        }
    }

    /// FIXME 这个逻辑提前到了 open_orders 中。所以可能产生重复。
    /// 判断 PostOnly 订单是否符合条件，并确定其是 Maker 还是被拒绝。
    ///
    /// 如果订单不符合 PostOnly 的条件（即买单价格低于当前市场价格，或卖单价格高于当前市场价格），
    /// 则会拒绝该订单，并将其从待处理订单中删除。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的 PostOnly 订单 (`Order<RequestOpen>`)。
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
    fn determine_post_only_order_role(&self, order: &Order<RequestOpen>, current_price: f64) -> Result<OrderRole, ExchangeError>
    {
        match order.side {
            | Side::Buy => {
                if order.state.price < current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    Err(ExchangeError::PostOnlyViolation("PostOnly order should be rejected".into()))
                }
            }
            | Side::Sell => {
                if order.state.price > current_price {
                    Ok(OrderRole::Maker)
                }
                else {
                    Err(ExchangeError::PostOnlyViolation("PostOnly order should be rejected".into()))
                    // 返回需要拒绝的错误，但不立即执行拒绝操作
                }
            }
        }
    }
}
#[async_trait]
pub trait OrderRoleClassifier
{
    /// 根据订单类型和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的订单 (`Order<RequestOpen>`)。
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
    fn determine_maker_taker(&self, order: &Order<RequestOpen>, order_book: &SingleLevelOrderBook) -> Result<OrderRole, ExchangeError>;
    /// 根据限价订单的价格和当前市场价格，确定订单是 Maker 还是 Taker。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的限价订单 (`Order<RequestOpen>`)。
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
    fn determine_limit_order_role(&self, order: &Order<RequestOpen>, current_price: f64) -> Result<OrderRole, ExchangeError>;
    /// FIXME 这个逻辑提前到了 open_orders 中。所以可能产生重复。
    /// 判断 PostOnly 订单是否符合条件，并确定其是 Maker 还是被拒绝。
    ///
    /// 如果订单不符合 PostOnly 的条件（即买单价格低于当前市场价格，或卖单价格高于当前市场价格），
    /// 则会拒绝该订单，并将其从待处理订单中删除。
    ///
    /// # 参数
    ///
    /// - `order`: 待处理的 PostOnly 订单 (`Order<RequestOpen>`)。
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
    fn determine_post_only_order_role(&self, order: &Order<RequestOpen>, current_price: f64) -> Result<OrderRole, ExchangeError>;
}

#[async_trait]
pub trait LatencySimulator
{
    async fn generate_latencies(latency_generator: &mut AccountLatency) -> [i64; 20];
    fn get_random_latency(&self) -> i64;
    async fn process_backtest_requestopen_with_a_simulated_latency(&mut self, order: Order<RequestOpen>) -> Order<RequestOpen>;
    fn update_latency(&mut self, current_time: i64);
}

#[async_trait]
impl LatencySimulator for AccountOrders
{
    /// 生成一个新的 `RequestId`
    /// NOTE 可能暂时永不上。但是在日后的web版本中很可能会被用到。
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
    // pub fn generate_request_id(&self, request: &Order<RequestOpen>) -> RequestId
    // {
    //     let counter = self.request_counter.fetch_add(1, Ordering::SeqCst);
    //     let request_ts = request.timestamp;
    //     RequestId::new(request_ts as u64, self.machine_id, counter)
    // }

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
        let mut seed = generate_machine_id().unwrap();
        let mut rng = rand::thread_rng();
        let mut latencies = [0; 20];

        for (i, latency) in latencies.iter_mut().enumerate() {
            // 增加种子的变化范围，确保不同种子之间有足够大的差异
            seed += rng.gen_range(1..1000000) as u64 + (i as u64 * 9999);
            fluctuate_latency(latency_generator, seed as i64);
            *latency = latency_generator.current_value;
            // 调试信息，输出种子和当前值
            // println!("Iteration: {}, Seed: {}, Current Value: {}", i, seed, latency_generator.current_value);
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

    /// # 参数
    ///
    /// - `order`: 要处理的订单请求 (`Order<RequestOpen>`)。
    ///
    /// # 返回值
    ///
    /// - 返回一个包含预测时间戳的待处理订单 (`Order<RequestOpen>`)。
    ///   注意 : 仅在回测场景下用这个方法！！！
    async fn process_backtest_requestopen_with_a_simulated_latency(&mut self, order: Order<RequestOpen>) -> Order<RequestOpen>
    {
        // 从预定义的延迟值数组中选择一个延迟值
        let latency = self.get_random_latency();
        let adjusted_client_ts = order.timestamp + latency;

        // 创建并返回新的 RequestOpen 订单
        Order { instruction: order.instruction,
                exchange: order.exchange,
                instrument: order.instrument,
                cid: order.cid,
                timestamp: adjusted_client_ts,
                side: order.side,
                state: RequestOpen { reduce_only: order.state.reduce_only,
                                     price: order.state.price,
                                     size: order.state.size } }
    }

    /// 更新账户的延迟值。
    ///
    /// 该函数通过调用 [fluctuate_latency] 方法来更新账户的延迟生成器，并根据当前的时间进行动态调整。
    /// 更新后的延迟值将用于模拟订单的延迟，适用于回测或模拟环境。
    ///
    /// # 参数
    ///
    /// - current_time: 当前时间，以微秒为单位，作为调整延迟值的参考点。
    fn update_latency(&mut self, current_time: i64)
    {
        fluctuate_latency(&mut self.latency_generator, current_time);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{
            instrument::{kind::InstrumentKind, Instrument},
            order::identification,
        },
        hourglass::account::{
            account_latency::{AccountLatency, FluctuationMode},
            account_orders::{LatencySimulator, OrderRoleClassifier},
        },
        Exchange,
    };
    use client_order_id::ClientOrderId;
    use identification::client_order_id;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_generate_latencies()
    {
        let account_latency = AccountLatency::new(FluctuationMode::Cosine, 100, 10);

        let latency_generator = Arc::new(RwLock::new(account_latency));

        // 获取可变引用
        let mut latency_generator = latency_generator.write().await;

        // 传递给 generate_latencies 函数
        let latencies = AccountOrders::generate_latencies(&mut latency_generator).await;

        // println!("{:?}", latencies);
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
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);

        let account_orders = AccountOrders::new(123124, instruments.clone(), account_latency).await;

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
        let account_latency = AccountLatency::new(FluctuationMode::Cosine, 100, 10);

        let account_orders = AccountOrders::new(1231, instruments, account_latency).await;

        let orders = account_orders.fetch_all();
        assert!(orders.is_empty());
    }

    #[tokio::test]
    async fn test_increment_request_counter()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Cosine, 100, 10);
        let account_orders = AccountOrders::new(09890, instruments, account_latency).await;

        assert_eq!(account_orders.order_counter.load(Ordering::Acquire), 0);
        account_orders.increment_order_counter();
        assert_eq!(account_orders.order_counter.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn test_order_id()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
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

    #[tokio::test]
    async fn test_process_backtest_requestopen_with_a_simulated_latency()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let mut account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::Binance,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: Some(ClientOrderId("unit_test".to_string())),
                            timestamp: 1625232523000,
                            side: Side::Buy,
                            state: RequestOpen { reduce_only: false,
                                                 price: 35000.0,
                                                 size: 0.1 } };

        let simulated_order = account_orders.process_backtest_requestopen_with_a_simulated_latency(order).await;
        assert!(simulated_order.timestamp >= 1625232523000 + 10); // Assuming latency is at least 10
        assert!(simulated_order.timestamp <= 1625232523000 + 100); // Assuming latency is at most 100
    }

    #[tokio::test]
    async fn test_determine_maker_taker()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::Binance,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: Some(ClientOrderId("unit_test".to_string())),
                            timestamp: 1625232523000,
                            side: Side::Buy,
                            state: RequestOpen { reduce_only: false,
                                                 price: 35000.0,
                                                 size: 0.1 } };

        // 构建模拟的订单簿
        let order_book = SingleLevelOrderBook { latest_bid: 34900.0,
                                                latest_ask: 35100.0,
                                                latest_price: 0.0 };

        // 将订单簿传递给 determine_maker_taker
        let result = account_orders.determine_maker_taker(&order, &order_book);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OrderRole::Maker); // 这里可能是 Maker 或 Taker, 取决于测试的订单簿逻辑
    }

    #[tokio::test]
    async fn test_determine_post_only_order_role()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        let order = Order { instruction: OrderInstruction::PostOnlyLimit,
                            exchange: Exchange::Binance,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: Some(ClientOrderId("unit_test".to_string())),
                            timestamp: 1625232523000,
                            side: Side::Buy,
                            state: RequestOpen { reduce_only: false,
                                                 price: 35000.0, // 买单价格
                                                 size: 0.1 } };

        // 成功场景：Post-Only 买单，挂单价格低于市场价格，成为 Maker
        let result = account_orders.determine_post_only_order_role(&order, 35001.0);
        assert_eq!(result.unwrap(), OrderRole::Maker);

        // 失败场景：Post-Only 买单，挂单价格高于市场价格，违反条件，应该返回错误
        let reject_result = account_orders.determine_post_only_order_role(&order, 34999.0);
        assert_eq!(reject_result.unwrap_err().to_string(), "PostOnlyViolation");
    }

    #[tokio::test]
    async fn test_build_order_open()
    {
        let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
        let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
        let mut account_orders = AccountOrders::new(123123, instruments, account_latency).await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::Binance,
                            instrument: Instrument::new("BTC", "USD", InstrumentKind::Spot),
                            cid: Some(ClientOrderId("unit_test".to_string())),
                            timestamp: 1625232523000,
                            side: Side::Buy,
                            state: RequestOpen { reduce_only: false,
                                                 price: 35000.0,
                                                 size: 0.1 } };

        let open_order = account_orders.build_order_open(order, OrderRole::Maker).await;

        assert_eq!(open_order.state.price, 35000.0);
        assert_eq!(open_order.state.size, 0.1);
        assert_eq!(open_order.state.filled_quantity, 0.0);
        assert_eq!(open_order.state.order_role, OrderRole::Maker);
        assert!(open_order.state.id.value() > 0);
    }
}
