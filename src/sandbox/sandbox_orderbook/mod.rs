use crate::common::order::{identification::OrderId, states::open::Open, Order};
/// NOTE MODULE CODE BELOW IS UNDER CONSTRUCTION
///
/// ### 1. **高级撮合逻辑**
///    - **部分成交 (Partial Fill)**: 目前的代码已经考虑了部分成交的情况，但你可以进一步优化部分成交的逻辑。例如，当一个订单被部分成交后，其剩余部分是否应该立即与下一个层级的订单继续撮合，或者应该优先处理其他等待中的订单。
///    - **优先级撮合**: 当有多个订单在同一价格层级时，可以实现基于时间戳的优先级撮合（即更早提交的订单优先成交），以更接近真实市场的逻辑。
///
/// ### 2. **订单过期 (Order Expiration)**
///   - **限时订单**: 增加订单过期时间的概念，某些订单可能只在一段时间内有效（如5分钟内有效），如果在此期间未成交则自动撤销。你可以在 Order 结构体中增加一个过期时间字段，并在 process_trades 方法中检查并处理过期订单。
///
/// ### 3. **订单取消 (Order Cancellation)**
///    - **取消功能**: 增加订单取消的功能，允许用户在订单未完全成交之前撤销订单。你可以实现一个 cancel_order 方法，通过订单ID查找并移除对应的订单。
///
/// ### 4. **交易手续费 (Transaction Fees)**
///    - **手续费计算**: 在成交时计算并记录交易手续费。手续费可以是固定的，也可以是根据交易量的百分比计算的。你需要在 process_trades 方法中加入手续费的计算，并更新订单或账户余额。
///
/// ### 5. **订单簿快照 (Order Book Snapshot)** [DONE]
///    - **快照功能**: 允许用户在任意时刻获取当前订单簿的快照，便于分析和调试。你可以实现一个 snapshot 方法，返回当前 bid_levels 和 ask_levels 的深拷贝。
///
/// ### 6. **成交记录 (Trade History)**
///    - **记录成交历史**: 增加一个数据结构，用于记录所有的成交记录（如成交价格、成交量、时间等）。你可以在 process_trades 方法中，每次成交时将相关信息记录到历史列表中，供后续查询和分析。
///
/// ### 7. **订单簿清理 (Order Book Cleanup)**
///    - **定期清理无效订单**: 定期检查并清理已经过期或取消的订单，以确保订单簿的整洁和高效。你可以在 process_trades 或其他合适的时机进行这些清理操作。
///
/// ### 8. **滑点模拟 (Slippage Simulation)** [DONE]
///    - **模拟滑点**: 为了更接近真实市场，可以模拟滑点，即当大量订单进入市场时，价格可能会有小幅波动，导致成交价格与预期价格不完全一致。这可以通过在 process_trades 方法中加入随机扰动实现。
///
/// ### 9. **订单簿深度限制 (Order Book Depth Limitation)** [DONE]
///    - **限制展示深度**: 实现一个功能，用于限制订单簿中可见的价格层级数量（例如只展示前5个买单和卖单层级），这可以用来模拟不同市场的深度和流动性。
///
/// ### 10. **多线程和并发处理**
///    - **多线程处理**: 如果你期望订单簿在高并发情况下运行，考虑使用多线程或异步处理订单的插入和撮合。这可以提升系统的性能，但需要小心处理数据竞争和同步问题。
use crate::common::Side;
use rayon::{iter::IntoParallelRefIterator, prelude::IndexedParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel
{
    pub price: f64,                    // 价格层级
    pub orders: VecDeque<Order<Open>>, // 使用VecDeque保证FIFO顺序的订单队列
}

impl PriceLevel
{
    fn new(price: f64) -> Self
    {
        PriceLevel { price, orders: VecDeque::new() }
    }

    fn add_order(&mut self, order: Order<Open>)
    {
        self.orders.push_back(order); // 先进先出，插入到队列尾部
    }

    fn remove_order(&mut self) -> Option<Order<Open>>
    {
        self.orders.pop_front() // 从队列头部移除并返回最早的订单
    }

    fn remove_expired_orders(&mut self, expiration_times: &HashMap<OrderId, i64>, current_time: i64)
    {
        self.orders.retain(|order| {
            if let Some(&expire_time) = expiration_times.get(&order.state.id) {
                expire_time > current_time // 保留未过期的订单
            } else {
                true // 如果订单没有指定过期时间，则保留
            }
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandBoxOrderBook
{
    pub bid_levels: Vec<PriceLevel>,                // 买单簿
    pub ask_levels: Vec<PriceLevel>,                // 卖单簿
    pub max_levels: usize,                          // 允许的最大层级数量
    pub expiration_registry: HashMap<OrderId, i64>, // 订单ID与过期时间的映射
}

impl SandBoxOrderBook
{
    pub fn new(max_levels: usize) -> Self
    {
        Self {
            bid_levels: Vec::new(),
            ask_levels: Vec::new(),
            max_levels,
            expiration_registry: HashMap::new(),
        }
    }

    pub fn set_order_expiration(&mut self, order_id: OrderId, expire_ts: i64)
    {
        self.expiration_registry.insert(order_id, expire_ts); // 设置订单的过期时间
    }

    pub fn insert_order(&mut self, order: Order<Open>)
    {
        // 根据订单的买卖方向，选择合适的价格层级列表（买单簿或卖单簿）
        let levels = match order.side {
            | Side::Buy => &mut self.bid_levels,  // 买单簿
            | Side::Sell => &mut self.ask_levels, // 卖单簿
        };

        // 尝试在现有的价格层级中找到与订单价格匹配的层级
        match levels.iter_mut().find(|level| level.price == order.state.price) {
            // 如果找到相同价格的层级，直接将订单添加到该层级
            | Some(level) => level.add_order(order),
            // 如果没有找到相同价格的层级，创建一个新的价格层级
            | None => {
                let mut new_level = PriceLevel::new(order.state.price); // 创建新价格层级
                new_level.add_order(order); // 将订单添加到新层级

                // 检查是否超过最大层级数量限制
                if levels.len() >= self.max_levels {
                    // 根据需要处理：例如，可以选择删除最低优先级的层级
                    levels.pop(); // 这里我们简单地删除最后一个层级，假设它是优先级最低的
                }

                levels.push(new_level); // 将新层级添加到价格层级列表中
                levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // 将价格层级列表按照价格排序
            }
        }
    }

    // NOTE 注意和Account模块的兼容性
    pub fn process_trades(&mut self, current_time: i64)
    {
        // 清理过期的订单
        for level in &mut self.bid_levels {
            level.remove_expired_orders(&self.expiration_registry, current_time);
        }
        for level in &mut self.ask_levels {
            level.remove_expired_orders(&self.expiration_registry, current_time);
        }

        // 如果买单簿或卖单簿为空，则无法进行撮合
        if self.bid_levels.is_empty() || self.ask_levels.is_empty() {
            return;
        }

        // 当买单簿和卖单簿都不为空时，进行订单撮合
        while !self.bid_levels.is_empty() && !self.ask_levels.is_empty() {
            let buy_level = &mut self.bid_levels[0]; // 获取买单簿的最优价格层级（价格最高）
            let sell_level = &mut self.ask_levels[0]; // 获取卖单簿的最优价格层级（价格最低）

            // 循环尝试撮合买单和卖单，直到其中一个层级的订单无法继续撮合
            while let (Some(mut buy_order), Some(mut sell_order)) = (buy_level.remove_order(), sell_level.remove_order()) {
                // 如果买单价格大于或等于卖单价格，则可以成交
                if buy_order.state.price >= sell_order.state.price {
                    // 计算成交量，取买单和卖单的最小剩余数量
                    let executed_quantity = if buy_order.state.size < sell_order.state.size {
                        buy_order.state.size
                    } else {
                        sell_order.state.size
                    };

                    // 更新订单的剩余数量
                    buy_order.state.size -= executed_quantity;
                    sell_order.state.size -= executed_quantity;

                    // 如果买单还有剩余数量，将其重新添加到买单簿的相应层级
                    if buy_order.state.size > 0.0 {
                        buy_level.add_order(buy_order);
                    }
                    // 如果卖单还有剩余数量，将其重新添加到卖单簿的相应层级
                    if sell_order.state.size > 0.0 {
                        sell_level.add_order(sell_order);
                    }
                } else {
                    // 如果买单价格小于卖单价格，则无法成交，将订单重新放回原层级
                    buy_level.add_order(buy_order);
                    sell_level.add_order(sell_order);
                    break; // 跳出循环，尝试下一个价格层级
                }
            }

            // 如果买单层级的订单全部成交完毕，则移除该价格层级
            if buy_level.orders.is_empty() {
                self.bid_levels.remove(0);
            }
            // 如果卖单层级的订单全部成交完毕，则移除该价格层级
            if sell_level.orders.is_empty() {
                self.ask_levels.remove(0);
            }
        }
    }

    // 获取订单簿快照 NOTE 注意和Account模块的兼容性
    pub fn snapshot(&self) -> (Vec<PriceLevel>, Vec<PriceLevel>)
    {
        (self.bid_levels.clone(), self.ask_levels.clone())
    }

    // 取消订单 NOTE 注意和Account模块的兼容性
    pub fn cancel_order(&mut self, order_id: OrderId) -> Option<Order<Open>>
    {
        for levels in [&mut self.bid_levels, &mut self.ask_levels].iter_mut() {
            for level in levels.iter_mut() {
                if let Some(pos) = level.orders.par_iter().position_any(|order| order.state.id == order_id) {
                    return level.orders.remove(pos);
                }
            }
        }
        None
    }
}
