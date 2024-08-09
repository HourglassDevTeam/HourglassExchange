use std::collections::BinaryHeap;
use serde::{Deserialize, Serialize};
use crate::common_infrastructure::order::{Open, Order};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub orders: BinaryHeap<Order<Open>>,
}

impl PriceLevel {
    fn new(price: f64) -> Self {
        PriceLevel {
            price,
            orders: BinaryHeap::new(),
        }
    }

    fn add_order(&mut self, order: Order<Open>) {
        self.orders.push(order);
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub levels: Vec<PriceLevel>, // A vector of levels sorted by price
}


impl OrderBook {
    pub fn insert_order(&mut self, order: Order<Open>) {
        match self.levels.iter_mut().find(|level| level.price == order.state.price) {
            Some(level) => level.add_order(order),
            None => {
                let mut new_level = PriceLevel::new(order.state.price);
                new_level.add_order(order);
                self.levels.push(new_level);
                self.levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            }
        }
    }
}