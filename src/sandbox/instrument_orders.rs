use std::cmp::Ordering;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::error::ExecutionError;
use crate::{
    common_infrastructure::{
        datafeed::event::MarketEvent,
        friction::{Fees, InstrumentFees, OptionFees, PerpetualFees, SpotFees},
        instrument::kind::InstrumentKind,
        order::{Open, Order},
        trade::{ClientTrade, TradeId},
        Side,
    },
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade,
};

/// 客户端针对一个 [`Instrument`] 的 [`InstrumentOrders`]。模拟客户端订单簿。
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InstrumentOrders
{
    pub batch_id: i64, // NOTE might be redundant
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

/// 计算 [`Order<Open>`] 对应的 [`Fees`]
pub fn calculate_fees(order: &Order<Open>, trade_quantity: f64, fees_percent: f64) -> InstrumentFees
{
    match order.instrument.kind {
        // 针对现货交易的费用计算
        | InstrumentKind::Spot => {
            let spot_fees = SpotFees {
                maker_rate: fees_percent * trade_quantity, // 制造流动性的费率计算
                taker_rate: fees_percent * trade_quantity, /* 消耗流动性的费率计算 */
            };
            InstrumentFees::new(order.instrument.kind, Fees::Spot(spot_fees))
        }

        // 针对永续合约的费用计算
        | InstrumentKind::Perpetual => {
            let perpetual_fees = PerpetualFees {
                maker_rate: fees_percent * trade_quantity,   // 开仓费率计算
                taker_rate: fees_percent * trade_quantity,   // 平仓费率计算
                funding_rate: fees_percent * trade_quantity, /* 资金费率计算 */
            };
            InstrumentFees::new(order.instrument.kind, Fees::Perpetual(perpetual_fees))
        }

        // 针对期权交易的费用计算
        | InstrumentKind::CryptoOption => {
            let option_fees = OptionFees { trade_fee_rate: fees_percent * trade_quantity /* 交易费率计算 */ };
            InstrumentFees::new(order.instrument.kind, Fees::Option(option_fees))
        }

        // 处理未知的交易类型
        | _ => panic!("Unsupported instrument kind!"),
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

    // 检查输入的 [`ClickhousePublicTrade`] 是否匹配买单或卖单的客户 [`Order<Open>`]
    //
    // NOTE:
    //  - 如果Client在同一价格同时开了买单和卖单 [`Order<Open>`]，优先选择剩余数量较大的
    //    Order<Open> 进行匹配。
    pub fn determine_matching_side(&self, market_event: &MarketEvent<ClickhousePublicTrade>) -> Option<Side>
    {
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

            // 最佳买单 Order<Open> 匹配输入的 ClickhousePublicTrade
            | (Some(best_bid), None) if best_bid.state.price >= market_event.kind.price => Some(Side::Buy),

            // 最佳卖单 Order<Open> 匹配输入的 ClickhousePublicTrade
            | (None, Some(best_ask)) if best_ask.state.price <= market_event.kind.price => Some(Side::Sell),

            // 要么没有买单或卖单 Order<Open>，要么没有匹配
            | _ => None,
        }
    }

    pub fn match_bids(&mut self, market_event: &MarketEvent<ClickhousePublicTrade>, fees_percent: f64) -> Vec<ClientTrade>
    {
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
                trades.push(self.generate_trade_event(&best_bid, remaining_quantity, fees_percent).unwrap());

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            } else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_bid.state.filled_quantity += trade_quantity;
                trades.push(self.generate_trade_event(&best_bid, trade_quantity, fees_percent).unwrap());
                self.bids.push(best_bid); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }

    /// NOTE 目前暂时的做法是使用 `batch_id` 值为此 [`Instrument`] 市场生成唯一的 [`TradeId`]。
    pub fn trade_id(&self) -> TradeId
    {
        TradeId(self.batch_id)
    }

    pub fn match_asks(&mut self, market_event: &MarketEvent<ClickhousePublicTrade>, fees_percent: f64) -> Vec<ClientTrade>
    {
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
                trades.push(self.generate_trade_event(&best_ask, remaining_quantity, fees_percent).unwrap());

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            } else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_ask.state.filled_quantity += trade_quantity;
                trades.push(self.generate_trade_event(&best_ask.clone(), trade_quantity, fees_percent).unwrap());
                self.asks.push(best_ask); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }

    // FIXME count和 tradeid 还有 orderid 的关系是错误的。
    // 辅助函数：生成 TradeEvent
    pub fn generate_trade_event(&self, order: &Order<Open>, trade_quantity: f64, fees_percent: f64) -> Result<ClientTrade, ExecutionError> {
        let fee = trade_quantity * order.state.price * fees_percent;

        // 尝试将 OrderId 转换为 TradeId
        let trade_id = order.state.id.0.parse::<i64>().map_err(|_| ExecutionError::InvalidID)?;

        Ok(ClientTrade {
            id: TradeId(trade_id),
            instrument: order.instrument.clone(),
            side: order.side,
            price: order.state.price,
            size: trade_quantity,
            count: 1, // NOTE 假设每笔交易计数为1，可以根据实际情况调整
            fees: fee,
        })
    }

    /// 计算所有未成交买单和卖单的总数。
    pub fn num_orders(&self) -> usize
    {
        self.bids.len() + self.asks.len()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_infrastructure::order::{OrderId, OrderKind, OrderRole};
    use crate::common_infrastructure::token::Token;
    use crate::common_infrastructure::{
        event::ClientOrderId,
        instrument::Instrument,
        Side,
    };
    use crate::ExchangeVariant;
    use uuid::Uuid;

    fn create_order(side: Side, price: f64, size: f64) -> Order<Open> {
        Order {
            kind: OrderKind::Limit,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument {
                base: Token::from("SOL"),
                quote: Token::from("USDT"),
                kind: Default::default(),
            },
            client_ts: 0,
            cid: ClientOrderId(Uuid::new_v4()),
            side,
            state: Open {
                id: OrderId("12345".into()), // 使用一个有效的 OrderId
                price,
                size,
                filled_quantity: 0.0,
                order_role: OrderRole::Maker,
                received_ts: 0,
            },
        }
    }


    #[test]
    fn test_add_order_open() {
        let mut instrument_orders = InstrumentOrders::default();

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        let order_sell = create_order(Side::Sell, 110.0, 1.0);

        instrument_orders.add_order_open(order_buy.clone());
        instrument_orders.add_order_open(order_sell.clone());

        assert_eq!(instrument_orders.bids.len(), 1);
        assert_eq!(instrument_orders.asks.len(), 1);
        assert_eq!(instrument_orders.bids[0], order_buy);
        assert_eq!(instrument_orders.asks[0], order_sell);
    }

    #[test]
    fn test_determine_matching_side() {
        let mut instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        let order_sell = create_order(Side::Sell, 110.0, 1.0);

        instrument_orders.add_order_open(order_buy);
        instrument_orders.add_order_open(order_sell);

        // 创建 MarketEvent，价格在买单和卖单之间
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                price: 105.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let matching_side = instrument_orders.determine_matching_side(&market_event);
        assert_eq!(matching_side, None);

        // 创建 MarketEvent，价格匹配买单
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                price: 100.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let matching_side = instrument_orders.determine_matching_side(&market_event);
        assert_eq!(matching_side, Some(Side::Buy));

        // 创建 MarketEvent，价格匹配卖单
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "sell".to_string(),
                price: 110.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let matching_side = instrument_orders.determine_matching_side(&market_event);
        assert_eq!(matching_side, Some(Side::Sell));
    }


    #[test]
    fn test_match_bids() {
        let mut instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        instrument_orders.add_order_open(order_buy);

        // 创建 MarketEvent，价格便宜
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "sell".to_string(),
                price: 95.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = InstrumentOrders::match_bids(&mut instrument_orders, &market_event, 0.01);
        assert_eq!(trades.len(), 1); // 价格匹配


        // 创建 MarketEvent，价格刚好达到买单
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "sell".to_string(),
                price: 100.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = instrument_orders.match_bids(&market_event, 0.01);
        assert_eq!(trades.len(), 0); // 价格匹配，但是之前的订单已经完成了，所以现在bids长度是0.
        assert_eq!(instrument_orders.num_orders(), 0); // 所有买单已匹配完成

        // 创建 MarketEvent，部分匹配买单
        let order_buy_partial = create_order(Side::Buy, 100.0, 2.0);
        instrument_orders.add_order_open(order_buy_partial);

        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "sell".to_string(),
                price: 100.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = instrument_orders.match_bids(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 部分匹配
        assert_eq!(instrument_orders.bids[0].state.remaining_quantity(), 1.0); // 剩余数量
        let remaining_order = &instrument_orders.bids[0];
        assert_eq!(remaining_order.state.remaining_quantity(), 1.0); // 剩余数量为1.0
        assert_eq!(remaining_order.state.filled_quantity, 1.0); // 已成交数量为1.0
    }
    #[test]
    fn test_match_asks() {
        let mut instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        let order_sell = create_order(Side::Sell, 100.0, 1.0);
        instrument_orders.add_order_open(order_sell);

        // 创建 MarketEvent，价格刚好达到卖单
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                price: 100.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 价格匹配成功

        // 创建 MarketEvent，价格更高
        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                price: 105.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 0); // ，价格匹配，但是之前的订单已经完成了，所以现在asks长度是0
        assert_eq!(instrument_orders.num_orders(), 0); // 所有卖单已匹配完成

        // 创建 MarketEvent，部分匹配卖单
        let order_sell_partial = create_order(Side::Sell, 100.0, 2.0);
        instrument_orders.add_order_open(order_sell_partial);

        let market_event = MarketEvent {
            exchange_time: 1625097600000,
            received_time: 1625097610000,
            exchange: ExchangeVariant::Binance,
            instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
            kind: ClickhousePublicTrade {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                price: 100.0,
                timestamp: 1625097600000,
                amount: 1.0,
            },
        };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 部分匹配
        assert_eq!(instrument_orders.asks[0].state.remaining_quantity(), 1.0); // 剩余数量
        let remaining_order = &instrument_orders.asks[0];
        assert_eq!(remaining_order.state.remaining_quantity(), 1.0); // 剩余数量为1.0
        assert_eq!(remaining_order.state.filled_quantity, 1.0); // 已成交数量为1.0
    }

    #[test]
    fn test_generate_trade_event_success() {
        let instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        // 创建一个有效的 OrderId
        let order = create_order(Side::Buy, 100.0, 1.0);
        let trade_event = instrument_orders.generate_trade_event(&order, 1.0, 0.01);

        match trade_event {
            Ok(trade) => {
                assert_eq!(trade.id, TradeId(12345));
                assert_eq!(trade.price, 100.0);
                assert_eq!(trade.size, 1.0);
                assert_eq!(trade.fees, 1.0); // 100 * 1 * 0.01 = 1.0
                assert_eq!(trade.count, 1);  // 确保 count 为 1
            }
            Err(e) => panic!("Test failed with error: {:?}", e),
        }
    }


    // 测试 generate_trade_event 方法处理无效 OrderId 时是否正确返回 ExecutionError::InvalidID。
    #[test]
    fn test_generate_trade_event_invalid_order_id() {
        let instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        // 创建一个无效的 OrderId
        let mut order = create_order(Side::Buy, 100.0, 1.0);
        order.state.id = OrderId("invalid_id".into()); // 设置一个无法解析为 i64 的 ID

        // 预期 generate_trade_event 返回 InvalidID 错误
        let result = instrument_orders.generate_trade_event(&order, 1.0, 0.01);
        assert!(matches!(result, Err(ExecutionError::InvalidID))); // 检查返回结果是否为 InvalidID 错误
    }


    #[test]
    fn test_num_orders() {
        let mut instrument_orders = InstrumentOrders {
            batch_id: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        };

        // 测试无订单的情况
        assert_eq!(instrument_orders.num_orders(), 0);

        // 添加一个买单
        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        instrument_orders.add_order_open(order_buy);
        assert_eq!(instrument_orders.num_orders(), 1);


        // 添加一个卖单
        let order_sell = create_order(Side::Sell, 110.0, 1.0);
        let order_sell_2 = create_order(Side::Sell, 115.0, 1.0);
        instrument_orders.add_order_open(order_sell);
        assert_eq!(instrument_orders.num_orders(), 2);

        //
        // 再添加两个买单和一个卖单
        let order_buy_2 = create_order(Side::Buy, 105.0, 1.0);
        let order_buy_3 = create_order(Side::Buy, 107.0, 1.0);

        instrument_orders.add_order_open(order_buy_2);
        instrument_orders.add_order_open(order_buy_3);
        instrument_orders.add_order_open(order_sell_2);
        //
        assert_eq!(instrument_orders.num_orders(), 5); // 3 个买单和 2 个卖单
    }
}