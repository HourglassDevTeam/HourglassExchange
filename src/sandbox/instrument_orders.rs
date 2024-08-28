use crate::common::datafeed::market_event::MarketEvent;
use rayon::prelude::ParallelSliceMut;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::{
    common::{
        friction::{Fees, InstrumentFees, OptionFees, PerpetualFees, SpotFees},
        instrument::kind::InstrumentKind,
        order::{states::open::Open, Order},
        trade::{ClientTrade, ClientTradeId},
        Side,
    },
    error::ExecutionError,
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade,
};

/// 客户端针对一个 [`Instrument`] 的 [`InstrumentOrders`]。模拟客户端订单簿。
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InstrumentOrders
{
    /// 在当前的代码设计中，batch_id 的递增仅在成功匹配订单并生成交易事件时发生
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
            let spot_fees = SpotFees { maker_fee: fees_percent * trade_quantity, // 制造流动性的费率计算
                                       taker_fee: fees_percent * trade_quantity  /* 消耗流动性的费率计算 */ };
            InstrumentFees::new(order.instrument.kind, Fees::Spot(spot_fees))
        }

        // 针对永续合约的费用计算
        | InstrumentKind::Perpetual => {
            let perpetual_fees = PerpetualFees { maker_fee: fees_percent * trade_quantity,   // 开仓费率计算
                                                 taker_fee: fees_percent * trade_quantity,   // 平仓费率计算
                                                 funding_fee: fees_percent * trade_quantity  /* 资金费率计算 */ };
            InstrumentFees::new(order.instrument.kind, Fees::Perpetual(perpetual_fees))
        }

        // 针对期权交易的费用计算
        | InstrumentKind::CryptoOption => {
            let option_fees = OptionFees { trade_fee: fees_percent * trade_quantity /* 交易费率计算 */ };
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
                self.bids.par_sort();
            }
            | Side::Sell => {
                // 添加 Order<Open> 到卖单
                self.asks.push(new_open_order);
                self.asks.par_sort();
            }
        }
    }

    // 检查传入的 [`MarketTrade`] 与当前客户 [`Order<Open>`] 匹配的是买单还是卖单
    pub fn determine_matching_side(&self, market_event: &MarketEvent<MarketTrade>) -> Option<Side>
    {
        match market_event.kind.side.as_str() {
            | "buy" => {
                // 如果市场方向是买单，检查卖单的最佳报价
                if let Some(best_ask) = self.asks.last() {
                    if market_event.kind.price >= best_ask.state.price {
                        return Some(Side::Sell);
                    }
                }
            }
            | "sell" => {
                // 如果市场方向是卖单，检查买单的最佳报价
                if let Some(best_bid) = self.bids.last() {
                    if market_event.kind.price <= best_bid.state.price {
                        return Some(Side::Buy);
                    }
                }
            }
            | _ => {
                println!("Input MarketTrade is likely to have mistaken 'side' info.")
            }
        }
        None
    }

    pub fn match_bids(&mut self, market_event: &MarketEvent<MarketTrade>, fees_percent: f64) -> Vec<ClientTrade>
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
                trades.push(self.generate_client_trade_event(&best_bid, remaining_quantity, fees_percent).unwrap());

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            }
            else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_bid.state.filled_quantity += trade_quantity;
                trades.push(self.generate_client_trade_event(&best_bid, trade_quantity, fees_percent).unwrap());
                self.bids.push(best_bid); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }

    /// NOTE 目前暂时的做法是使用 `batch_id` 值为此 [`Instrument`] 市场生成唯一的 [`ClientTradeId`]。
    pub fn trade_id(&self) -> ClientTradeId
    {
        ClientTradeId(self.batch_id)
    }

    pub fn match_asks(&mut self, market_event: &MarketEvent<MarketTrade>, fees_percent: f64) -> Vec<ClientTrade>
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
                trades.push(self.generate_client_trade_event(&best_ask, remaining_quantity, fees_percent).unwrap());

                // 如果流动性刚好耗尽，退出循环
                if remaining_liquidity == 0.0 {
                    break;
                }
            }
            else {
                // 部分成交
                let trade_quantity = remaining_liquidity;
                best_ask.state.filled_quantity += trade_quantity;
                trades.push(self.generate_client_trade_event(&best_ask.clone(), trade_quantity, fees_percent).unwrap());
                self.asks.push(best_ask); // 将部分成交后的订单重新放回队列
                break;
            }
        }

        trades
    }

    // 辅助函数：生成 ClientTrade
    // NOTE 直接生成 ClientTrade 事件而不生成 OrderFill（例如 FullyFill 或 PartialFill）在某些场景下是合理的，但也有一些需要注意的地方。
    pub fn generate_client_trade_event(&self, order: &Order<Open>, trade_quantity: f64, fees_percent: f64) -> Result<ClientTrade, ExecutionError>
    {
        let fee = trade_quantity * order.state.price * fees_percent;

        Ok(ClientTrade { trade_id: self.batch_id.into(), // NOTE trade_id 现在本质上是InstrumentOrders的一个counter生成的
                         client_order_id: order.state.id.clone(),
                         instrument: order.instrument.clone(),
                         side: order.side,
                         price: order.state.price,
                         quantity: trade_quantity,
                         fees: fee })
    }

    /// 计算所有未成交买单和卖单的总数。
    pub fn num_orders(&self) -> usize
    {
        self.bids.len() + self.asks.len()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{
            instrument::Instrument,
            order::{
                identification::{client_order_id::ClientOrderId, OrderId},
                order_instructions::OrderInstruction,
                OrderRole,
            },
            token::Token,
            Side,
        },
        sandbox::instrument_orders::Side::Buy,
        Exchange,
    };
    use Side::Sell;

    fn create_order(side: Side, price: f64, size: f64) -> Order<Open>
    {
        Order { kind: OrderInstruction::Limit,
                exchange: Exchange::Binance,
                instrument: Instrument { base: Token::from("SOL"),
                                         quote: Token::from("USDT"),
                                         kind: Default::default() },
                client_ts: 0,
                cid: ClientOrderId(Option::from("OJBK".to_string())),
                side,
                state: Open { id: OrderId("12345".into()), // 使用一个有效的 OrderId
                              price,
                              size,
                              filled_quantity: 0.0,
                              order_role: OrderRole::Maker,
                              received_ts: 0 } }
    }

    #[test]
    fn test_add_order_open()
    {
        let mut instrument_orders = InstrumentOrders::default();

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        let order_sell = create_order(Sell, 110.0, 1.0);

        instrument_orders.add_order_open(order_buy.clone());
        instrument_orders.add_order_open(order_sell.clone());

        assert_eq!(instrument_orders.bids.len(), 1);
        assert_eq!(instrument_orders.asks.len(), 1);
        assert_eq!(instrument_orders.bids[0], order_buy);
        assert_eq!(instrument_orders.asks[0], order_sell);
    }

    #[test]
    fn test_determine_matching_side_with_equal_prices()
    {
        let mut instrument_orders = InstrumentOrders { batch_id: 0,
                                                       bids: Vec::new(),
                                                       asks: Vec::new() };

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        let order_sell = create_order(Sell, 100.0, 1.0); // 与买单价格相同

        instrument_orders.add_order_open(order_buy);
        instrument_orders.add_order_open(order_sell);

        // 创建 MarketEvent，价格与买单和卖单相同
        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "buy".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let matching_side = instrument_orders.determine_matching_side(&market_event);
        // 假设你希望在价格相等时优先匹配买单
        assert_eq!(matching_side, Some(Sell));
    }

    #[test]
    fn test_match_bids()
    {
        let mut instrument_orders = InstrumentOrders { batch_id: 0,
                                                       bids: Vec::new(),
                                                       asks: Vec::new() };

        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        instrument_orders.add_order_open(order_buy);

        // 创建 MarketEvent，价格便宜
        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "sell".to_string(),
                                                             price: 95.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = InstrumentOrders::match_bids(&mut instrument_orders, &market_event, 0.01);
        assert_eq!(trades.len(), 1); // 价格匹配

        // 创建 MarketEvent，价格刚好达到买单
        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "sell".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = instrument_orders.match_bids(&market_event, 0.01);
        assert_eq!(trades.len(), 0); // 价格匹配，但是之前的订单已经完成了，所以现在bids长度是0.
        assert_eq!(instrument_orders.num_orders(), 0); // 所有买单已匹配完成

        // 创建 MarketEvent，部分匹配买单
        let order_buy_partial = create_order(Side::Buy, 100.0, 2.0);
        instrument_orders.add_order_open(order_buy_partial);

        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "sell".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = instrument_orders.match_bids(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 部分匹配
        assert_eq!(instrument_orders.bids[0].state.remaining_quantity(), 1.0); // 剩余数量
        let remaining_order = &instrument_orders.bids[0];
        assert_eq!(remaining_order.state.remaining_quantity(), 1.0); // 剩余数量为1.0
        assert_eq!(remaining_order.state.filled_quantity, 1.0); // 已成交数量为1.0
    }
    #[test]
    fn test_match_asks()
    {
        let mut instrument_orders = InstrumentOrders { batch_id: 0,
                                                       bids: Vec::new(),
                                                       asks: Vec::new() };

        let order_sell = create_order(Sell, 100.0, 1.0);
        instrument_orders.add_order_open(order_sell);

        // 创建 MarketEvent，价格刚好达到卖单
        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "buy".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 价格匹配成功

        // 创建 MarketEvent，价格更高
        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "buy".to_string(),
                                                             price: 105.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 0); // ，价格匹配，但是之前的订单已经完成了，所以现在asks长度是0
        assert_eq!(instrument_orders.num_orders(), 0); // 所有卖单已匹配完成

        // 创建 MarketEvent，部分匹配卖单
        let order_sell_partial = create_order(Sell, 100.0, 2.0);
        instrument_orders.add_order_open(order_sell_partial);

        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "buy".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let trades = instrument_orders.match_asks(&market_event, 0.01);
        assert_eq!(trades.len(), 1); // 部分匹配
        assert_eq!(instrument_orders.asks[0].state.remaining_quantity(), 1.0); // 剩余数量
        let remaining_order = &instrument_orders.asks[0];
        assert_eq!(remaining_order.state.remaining_quantity(), 1.0); // 剩余数量为1.0
        assert_eq!(remaining_order.state.filled_quantity, 1.0); // 已成交数量为1.0
    }

    #[test]
    fn test_generate_trade_event_success()
    {
        let instrument_orders = InstrumentOrders { batch_id: 1234,
                                                   bids: Vec::new(),
                                                   asks: Vec::new() };

        // 创建一个有效的 OrderId
        let order = create_order(Side::Buy, 100.0, 1.0);
        let trade_event = instrument_orders.generate_client_trade_event(&order, 1.0, 0.01);

        match trade_event {
            | Ok(trade) => {
                assert_eq!(trade.trade_id, ClientTradeId(1234));
                assert_eq!(trade.price, 100.0);
                assert_eq!(trade.quantity, 1.0);
                assert_eq!(trade.fees, 1.0); // 100 * 1 * 0.01 = 1.0
                                             // assert_eq!(trade.count, 1); // 确保 count 为 1
            }
            | Err(e) => panic!("Test failed with error: {:?}", e),
        }
    }

    #[test]
    fn test_num_orders()
    {
        let mut instrument_orders = InstrumentOrders { batch_id: 0,
                                                       bids: Vec::new(),
                                                       asks: Vec::new() };

        // 测试无订单的情况
        assert_eq!(instrument_orders.num_orders(), 0);

        // 添加一个买单
        let order_buy = create_order(Side::Buy, 100.0, 1.0);
        instrument_orders.add_order_open(order_buy);
        assert_eq!(instrument_orders.num_orders(), 1);

        // 添加一个卖单
        let order_sell = create_order(Sell, 110.0, 1.0);
        let order_sell_2 = create_order(Sell, 115.0, 1.0);
        instrument_orders.add_order_open(order_sell);
        assert_eq!(instrument_orders.num_orders(), 2);

        // 再添加两个买单和一个卖单
        let order_buy_2 = create_order(Side::Buy, 105.0, 1.0);
        let order_buy_3 = create_order(Side::Buy, 107.0, 1.0);

        instrument_orders.add_order_open(order_buy_2);
        instrument_orders.add_order_open(order_buy_3);
        instrument_orders.add_order_open(order_sell_2);
        //
        assert_eq!(instrument_orders.num_orders(), 5); // 3 个买单和 2 个卖单
    }

    #[test]
    fn test_no_orders()
    {
        let instrument_orders = InstrumentOrders::default();

        let market_event = MarketEvent { exchange_time: 1625097600000,
                                         received_time: 1625097610000,
                                         exchange: Exchange::Binance,
                                         instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                         kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                             symbol: "BTCUSDT".to_string(),
                                                             side: "buy".to_string(),
                                                             price: 100.0,
                                                             timestamp: 1625097600000,
                                                             amount: 1.0 } };

        let matching_side = instrument_orders.determine_matching_side(&market_event);
        assert_eq!(matching_side, None); // 没有订单时应该返回None
    }

    #[test]
    fn test_out_of_range_price()
    {
        let mut instrument_orders = InstrumentOrders::default();

        let order_buy = create_order(Buy, 90.0, 1.0); // 低价买单
        let order_sell = create_order(Sell, 110.0, 1.0); // 高价卖单

        instrument_orders.add_order_open(order_buy);
        instrument_orders.add_order_open(order_sell);

        // 市场价格远高于卖单，应该没有匹配
        let market_event_low = MarketEvent { exchange_time: 1625097600000,
                                             received_time: 1625097610000,
                                             exchange: Exchange::Binance,
                                             instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                             kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                                 symbol: "BTCUSDT".to_string(),
                                                                 side: "buy".to_string(),
                                                                 price: 100.0,
                                                                 timestamp: 1625097600000,
                                                                 amount: 1.0 } };

        let matching_side_low = instrument_orders.determine_matching_side(&market_event_low);
        assert_eq!(matching_side_low, None); // 应该没有匹配

        // 市场价格高于买单，应该没有匹配
        let market_event_high = MarketEvent { exchange_time: 1625097600000,
                                              received_time: 1625097610000,
                                              exchange: Exchange::Binance,
                                              instrument: Instrument::new("BTC".to_string(), "USDT".to_string(), InstrumentKind::Spot),
                                              kind: MarketTrade { exchange: "binance_futures".to_string(),
                                                                  symbol: "BTCUSDT".to_string(),
                                                                  side: "sell".to_string(),
                                                                  price: 115.0,
                                                                  timestamp: 1625097600000,
                                                                  amount: 1.0 } };

        let matching_side_high = instrument_orders.determine_matching_side(&market_event_high);
        assert_eq!(matching_side_high, None); // 应该没有匹配
    }
}
