use crate::common_infrastructure::Side;
use std::collections::{VecDeque, HashMap};
use serde::{Deserialize, Serialize};
use crate::common_infrastructure::order::{Open, Order, OrderId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub orders: VecDeque<Order<Open>>, // 使用VecDeque保证FIFO顺序
}

impl PriceLevel {
    fn new(price: f64) -> Self {
        PriceLevel {
            price,
            orders: VecDeque::new(),
        }
    }

    fn add_order(&mut self, order: Order<Open>) {
        self.orders.push_back(order); // 先进先出，插入到队列尾部
    }

    fn remove_order(&mut self) -> Option<Order<Open>> {
        self.orders.pop_front() // 从队列头部移除并返回最早的订单
    }

    fn remove_expired_orders(&mut self, expiration_times: &HashMap<OrderId, i64>, current_time: i64) {
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
pub struct SandBoxOrderBook {
    pub bid_levels: Vec<PriceLevel>, // 买单簿
    pub ask_levels: Vec<PriceLevel>, // 卖单簿
    pub max_levels: usize,           // 允许的最大层级数量
    pub expiration_registry: HashMap<OrderId, i64>, // 订单ID与过期时间的映射
}

impl SandBoxOrderBook {
    pub fn new(max_levels: usize) -> Self {
        Self {
            bid_levels: Vec::new(),
            ask_levels: Vec::new(),
            max_levels,
            expiration_registry: HashMap::new(),
        }
    }

    pub fn set_order_expiration(&mut self, order_id: OrderId, expire_ts: i64) {
        self.expiration_registry.insert(order_id, expire_ts);
    }

    pub fn insert_order(&mut self, order: Order<Open>) {
        // 根据订单的买卖方向，选择合适的价格层级列表（买单簿或卖单簿）
        let levels = match order.side {
            Side::Buy => &mut self.bid_levels,  // 买单簿
            Side::Sell => &mut self.ask_levels, // 卖单簿
        };

        // 尝试在现有的价格层级中找到与订单价格匹配的层级
        match levels.iter_mut().find(|level| level.price == order.state.price) {
            // 如果找到相同价格的层级，直接将订单添加到该层级
            Some(level) => level.add_order(order),
            // 如果没有找到相同价格的层级，创建一个新的价格层级
            None => {
                let mut new_level = PriceLevel::new(order.state.price); // 创建新价格层级
                new_level.add_order(order); // 将订单添加到新层级

                // 检查是否超过最大层级数量限制
                if levels.len() >= self.max_levels {
                    // 根据需要处理：例如，可以选择删除最低优先级的层级
                    // 这里我们简单地删除最后一个层级，假设它是优先级最低的
                    levels.pop();
                }

                levels.push(new_level); // 将新层级添加到价格层级列表中
                // 将价格层级列表按照价格排序
                levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            }
        }
    }


    // NOTE!! 注意和Account订单处理模块的兼容性
    pub fn process_trades(&mut self, current_time: i64) {
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
            let buy_level = &mut self.bid_levels[0];  // 获取买单簿的最优价格层级（价格最高）
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
}
