use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::common_skeleton::{
    order::{Open, Order},
    Side,
};

/// 客户端针对一个 [`Instrument`] 的 [`InstrumentOrders`]。模拟客户端订单簿。
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InstrumentOrders
{
    pub batch_id: u64,
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

/// 添加一个 [`Order<Open>`] 到买单或卖单中，取决于它的 [`Side`]。
impl InstrumentOrders
{
    pub fn add_order_open(&mut self, new_open_order: Order<Open>)
    {
        match new_open_order.side {
            | Side::Buy => {
                // 添加 Order<Opened> 到买单
                self.bids.push(new_open_order);
                self.bids.sort();
            }
            | Side::Sell => {
                // 添加 Order<Opened> 到卖单
                self.asks.push(new_open_order);
                self.asks.sort();
            }
        }
    }
}

//     /// Check if an input [`PublicTrade`] matches an bid or ask client [`Open<Order>`].
//     ///
//     /// Note:
//     ///  - In the event that the client has opened both a bid and ask [`Order<Open>`] at the same
//     ///    price, preferentially select the Order<Open> with the larger remaining quantity to
//     ///    match on.
//     pub fn has_matching_order(&self, trade: &PublicTrade) -> Option<Side> {
//         match (self.bids.last(), self.asks.last()) {
//             // Check the best bid & ask Order<Open> for a match
//             | (Some(best_bid), Some(best_ask)) => {
//                 // Note:
//                 // In the unlikely case that: best_bid.price == best_ask.price == trade.price
//                 // Preferentially select the larger remaining quantity Order<Open> to match on
//                 if best_bid.state.price == trade.price && best_ask.state.price == trade.price {
//                     let best_bid_quantity = best_bid.state.remaining_quantity();
//                     let best_ask_quantity = best_ask.state.remaining_quantity();
//                     match best_bid_quantity.partial_cmp(&best_ask_quantity) {
//                         | Some(Ordering::Greater) => Some(Side::Buy),
//                         | _ => Some(Side::Sell),
//                     }
//                 }
//                 // Best bid matches
//                 else if best_bid.state.price >= trade.price {
//                     Some(Side::Buy)
//                 }
//                 // Best ask matches
//                 else if best_ask.state.price <= trade.price {
//                     Some(Side::Sell)
//                 }
//                 // No matches
//                 else {
//                     None
//                 }
//             }
//
//             // Best bid Order<Open> matches the input PublicTrade
//             | (Some(best_bid), None) if best_bid.state.price >= trade.price => Some(Side::Buy),
//
//             // Best ask Order<Open> matches the input PublicTrade
//             | (None, Some(best_ask)) if best_ask.state.price <= trade.price => Some(Side::Sell),
//
//             // Either no bid or ask Order<Open>, or no matches
//             | _ => None,
//         }
//     }
//
//     /// Simulates [`Side::Buy`] trades by using the [`PublicTrade`] liquidity to match on open
//     /// client bid [`Order<Open>`]s.
//     pub fn match_bids(&mut self, trade: &PublicTrade, fees_percent: f64) -> Vec<Trade> {
//         // Keep track of how much trade liquidity is remaining to match with
//         let mut remaining_liquidity = trade.amount;
//
//         // Collection of execution Trades generated from Order<Open> matches
//         let mut trades = vec![];
//
//         let remaining_best_bid = loop {
//             // Pop the best bid Order<Open>
//             let mut best_bid = match self.bids.pop() {
//                 | Some(best_bid) => best_bid,
//                 | None => break None,
//             };
//
//             // Break with remaining best bid if it's not a match, or trade liquidity is exhausted
//             if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
//                 break Some(best_bid);
//             }
//
//             // Remaining liquidity is either a full-fill or a partial-fill
//             self.batch_id += 1;
//             match OrderFill::kind(&best_bid, remaining_liquidity) {
//                 // Full Order<Open> fill
//                 | OrderFill::Full => {
//                     // Remove trade quantity from remaining liquidity
//                     let trade_quantity = best_bid.state.remaining_quantity();
//                     remaining_liquidity -= trade_quantity;
//
//                     // Generate execution Trade from full Order<Open> fill
//                     trades.push(self.generate_trade(best_bid, trade_quantity, fees_percent));
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
//                     best_bid.state.filled_quantity += trade_quantity;
//                     trades.push(self.generate_trade(best_bid.clone(), trade_quantity, fees_percent));
//
//                     break Some(best_bid);
//                 }
//             }
//         };
//
//         // If remaining best bid had a partial-fill, or is not a match, put it back as the best bid
//         if let Some(remaining_best_bid) = remaining_best_bid {
//             self.bids.push(remaining_best_bid);
//         }
//
//         trades
//     }
//
//     /// Generate a client [`Trade`] with a unique [`TradeId`] for this [`Instrument`] market.
//     pub fn generate_trade(&self, order: Order<Open>, trade_quantity: f64, fees_percent: f64) -> Trade {
//         // Calculate the trade fees (denominated in base or quote depending on Order Side)
//         let fees = calculate_fees(&order, trade_quantity, fees_percent);
//
//         // Generate execution Trade from the Order<Open> match
//         Trade {
//             id: self.trade_id(),
//             order_id: order.state.id,
//             instrument: order.instrument,
//             side: order.side,
//             price: order.state.price,
//             quantity: trade_quantity,
//             fees,
//         }
//     }
//
//     /// Use the `batch_id` value to generate a unique [`TradeId`] for this [`Instrument`]
//     /// market.
//     pub fn trade_id(&self) -> TradeId {
//         TradeId(self.batch_id.to_string())
//     }
//
//     /// Simulates [`Side::Sell`] trades by using the [`PublicTrade`] liquidity to match on open
//     /// client bid [`Order<Open>`]s.
//     pub fn match_asks(&mut self, trade: &PublicTrade, fees_percent: f64) -> Vec<Trade> {
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
// }
