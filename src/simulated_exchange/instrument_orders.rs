// use std::cmp::Ordering;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::common_skeleton::{
    order::{Open, Order},
    Side,
};

// use crate::simulated_exchange::load_from_clickhouse::queries_operations::ClickhouseTrade;

/// 客户端针对一个 [`Instrument`] 的 [`InstrumentOrders`]。模拟客户端订单簿。
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InstrumentOrders {
    pub batch_id: u64,
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}
/// 添加一个 [`Order<Open>`] 到买单或卖单中，取决于它的 [`Side`]。
impl InstrumentOrders {
    pub fn add_order_open(&mut self, new_open_order: Order<Open>) {
        match new_open_order.side {
            | Side::Buy => {
                // 添加 Order<Open> 到买单
                self.bids.push(new_open_order);
                self.bids.sort();
            }
            | Side::Sell => {
                // 添加 Order<Open> 到卖单
                self.asks.push(new_open_order);
                self.asks.sort();
            }
        }
    }
    // 检查输入的 [`ClickhouseTrade`] 是否匹配买单或卖单的客户 [`Order<Open>`]
    //
    // 注意:
    //  - 如果客户在同一价格同时开了买单和卖单 [`Order<Open>`]，优先选择剩余数量较大的
    //    Order<Open> 进行匹配。
    // pub fn has_matching_order(&self, trade: &ClickhouseTrade) -> Option<Side> {
    //     match (self.bids.last(), self.asks.last()) {
    //         // 检查最佳买单和卖单的 Order<Open> 是否匹配
    //         | (Some(best_bid), Some(best_ask)) => {
    //             // 注意:
    //             // 在极少数情况下: best_bid.price == best_ask.price == trade.price
    //             // 优先选择剩余数量较大的 Order<Open> 进行匹配
    //             if best_bid.state.price == trade.price && best_ask.state.price == trade.price {
    //                 let best_bid_quantity = best_bid.state.remaining_quantity();
    //                 let best_ask_quantity = best_ask.state.remaining_quantity();
    //                 match best_bid_quantity.partial_cmp(&best_ask_quantity) {
    //                     | Some(Ordering::Greater) => Some(Side::Buy),
    //                     | _ => Some(Side::Sell),
    //                 }
    //             }
    //             // 最佳买单匹配
    //             else if best_bid.state.price >= trade.price {
    //                 Some(Side::Buy)
    //             }
    //             // 最佳卖单匹配
    //             else if best_ask.state.price <= trade.price {
    //                 Some(Side::Sell)
    //             }
    //             // 无匹配
    //             else {
    //                 None
    //             }
    //         }
    //
    //         // 最佳买单 Order<Open> 匹配输入的 ClickhouseTrade
    //         | (Some(best_bid), None) if best_bid.state.price >= trade.price => Some(Side::Buy),
    //
    //         // 最佳卖单 Order<Open> 匹配输入的 ClickhouseTrade
    //         | (None, Some(best_ask)) if best_ask.state.price <= trade.price => Some(Side::Sell),
    //
    //         // 要么没有买单或卖单 Order<Open>，要么没有匹配
    //         | _ => None,
    //     }
    // }
    //
    // /// Simulates [`Side::Buy`] trades by using the [`ClickhouseTrade`] liquidity to match on open
    // /// client bid [`Order<Open>`]s.
    // pub fn match_bids(&mut self, trade: &ClickhouseTrade, fees_percent: f64) -> Vec<ClickhouseTrade> {
    //     // Keep track of how much trade liquidity is remaining to match with
    //     let mut remaining_liquidity = trade.amount;
    //
    //     // Collection of execution Trades generated from Order<Open> matches
    //     let mut trades = vec![];
    //
    //     let remaining_best_bid = loop {
    //         // Pop the best bid Order<Open>
    //         let mut best_bid = match self.bids.pop() {
    //             | Some(best_bid) => best_bid,
    //             | None => break None,
    //         };
    //
    //         // Break with remaining best bid if it's not a match, or trade liquidity is exhausted
    //         if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
    //             break Some(best_bid);
    //         }
    //
    //         // Remaining liquidity is either a full-fill or a partial-fill
    //         self.batch_id += 1;
    //         match OrderFill::kind(&best_bid, remaining_liquidity) {
    //             // Full Order<Open> fill
    //             | OrderFill::Full => {
    //                 // Remove trade quantity from remaining liquidity
    //                 let trade_quantity = best_bid.state.remaining_quantity();
    //                 remaining_liquidity -= trade_quantity;
    //
    //                 // Generate execution Trade from full Order<Open> fill
    //                 trades.push(self.generate_trade(best_bid, trade_quantity, fees_percent));
    //
    //                 // If exact full fill with zero remaining liquidity (highly unlikely), break
    //                 if remaining_liquidity == 0.0 {
    //                     break None;
    //                 }
    //             }
    //
    //             // Partial Order<Open> fill with zero remaining trade liquidity
    //             | OrderFill::Partial => {
    //                 // Partial-fill means trade quantity is all the remaining trade liquidity
    //                 let trade_quantity = remaining_liquidity;
    //
    //                 // Generate execution Trade from partial Order<Open> fill
    //                 best_bid.state.filled_quantity += trade_quantity;
    //                 trades.push(self.generate_trade(best_bid.clone(), trade_quantity, fees_percent));
    //
    //                 break Some(best_bid);
    //             }
    //         }
    //     };
    //
    //     // If remaining best bid had a partial-fill, or is not a match, put it back as the best bid
    //     if let Some(remaining_best_bid) = remaining_best_bid {
    //         self.bids.push(remaining_best_bid);
    //     }
    //
    //     trades
    // }
    //
    //     /// Generate a client [`Trade`] with a unique [`TradeId`] for this [`Instrument`] market.
    //     pub fn generate_trade(&self, order: Order<Open>, trade_quantity: f64, fees_percent: f64) -> ClickhouseTrade {
    //         // Calculate the trade fees (denominated in base or quote depending on Order Side)
    //         let fees = calculate_fees(&order, trade_quantity, fees_percent);
    //
    //         // Generate execution Trade from the Order<Open> match
    //         ClickhouseTrade {
    //             id: self.trade_id(),
    //             order_id: order.state.id,
    //             instrument: order.instrument,
    //             side: order.side,
    //             price: order.state.price,
    //             quantity: trade_quantity,
    //             fees,
    //         }
    //     }}
    //
    //     /// Use the `batch_id` value to generate a unique [`TradeId`] for this [`Instrument`]
    //     /// market.
    //     pub fn trade_id(&self) -> TradeId {
    //         TradeId(self.batch_id.to_string())
    //     }
    //
    //     /// Simulates [`Side::Sell`] trades by using the [`ClickhouseTrade`] liquidity to match on open
    //     /// client bid [`Order<Open>`]s.
    //     pub fn match_asks(&mut self, trade: &ClickhouseTrade, fees_percent: f64) -> Vec<Trade> {
    //         // Keep track of how much trade liquidity is remaining to match with
    //         let mut remaining_liquidity = trade.amount;
    //
    //         // Collection of execution Trades generated from Order<Open> matches
    //         let mut trades = vec![];
    //
    //         let remaining_best_ask = loop {
    //             // Pop the best Order<Open>
    //             let mut best_ask = match self.asks.pop() {
    //                 | Some(best_ask) => best_ask,
    //                 | None => break None,
    //             };
    //
    //             // Break with remaining best ask if it's not a match, or trade liquidity is exhausted
    //             if best_ask.state.price > trade.price || remaining_liquidity <= 0.0 {
    //                 break Some(best_ask);
    //             }
    //
    //             // Remaining liquidity is either a full-fill or a partial-fill
    //             self.batch_id += 1;
    //             match OrderFill::kind(&best_ask, remaining_liquidity) {
    //                 // Full Order<Open> fill
    //                 | OrderFill::Full => {
    //                     // Remove trade quantity from remaining liquidity
    //                     let trade_quantity = best_ask.state.remaining_quantity();
    //                     remaining_liquidity -= trade_quantity;
    //
    //                     // Generate execution Trade from full Order<Open> fill
    //                     trades.push(self.generate_trade(best_ask, trade_quantity, fees_percent));
    //
    //                     // If exact full fill with zero remaining liquidity (highly unlikely), break
    //                     if remaining_liquidity == 0.0 {
    //                         break None;
    //                     }
    //                 }
    //
    //                 // Partial Order<Open> fill with zero remaining trade liquidity
    //                 | OrderFill::Partial => {
    //                     // Partial-fill means trade quantity is all the remaining trade liquidity
    //                     let trade_quantity = remaining_liquidity;
    //
    //                     // Generate execution Trade from partial Order<Open> fill
    //                     best_ask.state.filled_quantity += trade_quantity;
    //                     trades.push(self.generate_trade(best_ask.clone(), trade_quantity, fees_percent));
    //
    //                     break Some(best_ask);
    //                 }
    //             }
    //         };
    //
    //         // If remaining best ask had a partial-fill, or is not a match, put it back as the best ask
    //         if let Some(remaining_best_bid) = remaining_best_ask {
    //             self.asks.push(remaining_best_bid);
    //         }
    //
    //         trades
    //     }
    //
    //     /// Calculates the total number of open bids and asks.
    //     pub fn num_orders(&self) -> usize {
    //         self.bids.len() + self.asks.len()
    //     }
}
