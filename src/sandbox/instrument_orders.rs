use std::cmp::Ordering;
// use std::cmp::Ordering;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::common_infrastructure::{
    order::{Open, Order},
    Side,
};
use crate::common_infrastructure::datafeed::event::MarketEvent;
use crate::common_infrastructure::friction::{Fees, InstrumentFees, OptionFees, PerpetualFees, SpotFees};
use crate::common_infrastructure::instrument::kind::InstrumentKind;
use crate::common_infrastructure::order::FullyFill;
use crate::common_infrastructure::trade::{Trade, TradeId};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhouseTrade;

/// 客户端针对一个 [`Instrument`] 的 [`InstrumentOrders`]。模拟客户端订单簿。
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InstrumentOrders
{
    pub batch_id: i64, // NOTE might be redundant
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

/// 计算 [`Order<Open>`] 对应的 [`Fees`]
pub fn calculate_fees(order: &Order<Open>, trade_quantity: f64, fees_percent: f64) -> InstrumentFees {
    match order.instrument.kind {
        // 针对现货交易的费用计算
        InstrumentKind::Spot => {
            let spot_fees = SpotFees {
                maker_fee_rate: fees_percent * trade_quantity, // 制造流动性的费率计算
                taker_fee_rate: fees_percent * trade_quantity, // 消耗流动性的费率计算
            };
            InstrumentFees::new(order.instrument.kind.clone(), Fees::Spot(spot_fees))
        }

        // 针对永续合约的费用计算
        InstrumentKind::Perpetual => {
            let perpetual_fees = PerpetualFees {
                open_fee_rate: fees_percent * trade_quantity,  // 开仓费率计算
                close_fee_rate: fees_percent * trade_quantity, // 平仓费率计算
                funding_rate: fees_percent * trade_quantity,   // 资金费率计算
            };
            InstrumentFees::new(order.instrument.kind.clone(), Fees::Perpetual(perpetual_fees))
        }

        // 针对期权交易的费用计算
        InstrumentKind::CryptoOption => {
            let option_fees = OptionFees {
                trade_fee_rate: fees_percent * trade_quantity, // 交易费率计算
            };
            InstrumentFees::new(order.instrument.kind.clone(), Fees::Option(option_fees))
        }

        // 处理未知的交易类型
        _ => panic!("Unsupported instrument kind!"),
    }
}

/// 添加一个 [`Order<Open>`] 到买单或卖单中，取决于它的 [`Side`]。
impl InstrumentOrders
{
    pub fn add_order_open(&mut self, new_open_order: Order<Open>)
    {
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
    // NOTE:
    //  - 如果Client在同一价格同时开了买单和卖单 [`Order<Open>`]，优先选择剩余数量较大的
    //    Order<Open> 进行匹配。
    pub fn has_matching_order(&self, market_event: &MarketEvent<ClickhouseTrade>) -> Option<Side> {
        match (self.bids.last(), self.asks.last()) {
            // 检查最佳买单和卖单的 Order<Open> 是否匹配
            | (Some(best_bid), Some(best_ask)) => {
                // 注意:
                //      在极少数情况下: best_bid.price == best_ask.price == trade.price
                //      优先选择剩余数量较大的 Order<Open> 进行匹配
                if best_bid.state.price == market_event.kind.price && best_ask.state.price == market_event.kind.price {
                    let best_bid_quantity = best_bid.state.remaining_quantity();
                    let best_ask_quantity = best_ask.state.remaining_quantity();
                    match best_bid_quantity.partial_cmp(&best_ask_quantity) {
                        | Some(Ordering::Greater) => Some(Side::Buy),
                        | _ => Some(Side::Sell),
                    }
                }
                // 最佳买单匹配
                else if best_bid.state.price >= market_event.kind.price {
                    Some(Side::Buy)
                }
                // 最佳卖单匹配
                else if best_ask.state.price <= market_event.kind.price {
                    Some(Side::Sell)
                }
                // 无匹配
                else {
                    None
                }
            }

            // 最佳买单 Order<Open> 匹配输入的 ClickhouseTrade
            | (Some(best_bid), None) if best_bid.state.price >= market_event.kind.price => Some(Side::Buy),

            // 最佳卖单 Order<Open> 匹配输入的 ClickhouseTrade
            | (None, Some(best_ask)) if best_ask.state.price <= market_event.kind.price => Some(Side::Sell),

            // 要么没有买单或卖单 Order<Open>，要么没有匹配
            | _ => None,
        }
    }

    pub fn match_bids(&mut self, market_event: &MarketEvent<ClickhouseTrade>, fees_percent: f64) -> Vec<ClickhouseTrade> {
        // 跟踪剩余的可用流动性，以便匹配
        let mut remaining_liquidity = market_event.kind.amount;

        // 收集由匹配未成交的客户端买单生成的交易
        let mut trades = Vec::new();

        while let Some(mut best_bid) = self.bids.pop() {
            // 如果最优买单价格低于市场事件价格或流动性耗尽，退出循环
            if best_bid.state.price < market_event.kind.price || remaining_liquidity <= 0.0 {
                self.bids.push(best_bid);
                break;
            }

            // 增加批次ID
            self.batch_id += 1;

            // 获取订单的剩余数量
            let remaining_quantity = best_bid.state.remaining_quantity();

            // 判断是全量成交还是部分成交
            if remaining_quantity <= remaining_liquidity {
                // 全量成交
                remaining_liquidity -= remaining_quantity;
                trades.push(self.generate_trade(best_bid, remaining_quantity, fees_percent));

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            } else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_bid.state.filled_quantity += trade_quantity;
                trades.push(self.generate_trade(best_bid.clone(), trade_quantity, fees_percent));
                self.bids.push(best_bid); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }



    /// 使用 `batch_id` 值为此 [`Instrument`] 市场生成唯一的 [`TradeId`]。
    pub fn trade_id(&self) -> TradeId {
        TradeId(self.batch_id.into())
    }

    pub fn match_asks(&mut self, market_event: &MarketEvent<ClickhouseTrade>, fees_percent: f64) -> Vec<ClickhouseTrade> {
        // 跟踪剩余的可用流动性，以便匹配
        let mut remaining_liquidity = market_event.kind.amount;

        // 收集由匹配未成交的客户端卖单生成的交易
        let mut trades = Vec::new();

        while let Some(mut best_ask) = self.asks.pop() {
            // 如果最优卖单价格高于市场事件价格或流动性耗尽，退出循环
            if best_ask.state.price > market_event.kind.price || remaining_liquidity <= 0.0 {
                self.asks.push(best_ask);
                break;
            }

            // 增加批次ID
            self.batch_id += 1;

            // 获取订单的剩余数量
            let remaining_quantity = best_ask.state.remaining_quantity();

            // 判断是全量成交还是部分成交
            if remaining_quantity <= remaining_liquidity {
                // 全量成交
                remaining_liquidity -= remaining_quantity;
                trades.push(self.generate_trade(best_ask, remaining_quantity, fees_percent));

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            } else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_ask.state.filled_quantity += trade_quantity;
                trades.push(self.generate_trade(best_ask.clone(), trade_quantity, fees_percent));
                self.asks.push(best_ask); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }


    /// 使用唯一的 [`TradeId`] 为此 [`Instrument`] 市场生成一个客户端 [`Trade`]。
    pub fn generate_trade(&self, order: Order<Open>, trade_quantity: f64, fees_percent: f64) -> ClickhouseTrade {
        // 计算交易费用（取决于订单的方向，费用用基货币或报价货币表示）
        let fees = calculate_fees(&order, trade_quantity, fees_percent);

        // 生成由匹配订单 [`Order<Open>`] 生成的执行交易
        ClickhouseTrade {
            basequote: "".to_string(),
            side: order.side.to_string(),
            price: order.state.price,
            timestamp: 0,
            amount: 0.0,
        }
    }


    /// 计算所有未成交买单和卖单的总数。
    pub fn num_orders(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

