use crate::{
    common::{
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::OrderRole,
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExchangeError,
    sandbox::{
        account::{account_config::SandboxMode, handlers::balance_handler::BalanceHandler, SandboxAccount},
        clickhouse_api::datatype::{clickhouse_trade_data::MarketTrade, single_level_order_book::SingleLevelOrderBook},
    },
    Exchange,
};
use async_trait::async_trait;
use std::{
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::warn;
use crate::sandbox::account::handlers::position_handler::PositionHandler;

#[async_trait]
pub trait TradeHandler
{
    async fn create_or_single_level_orderbook_from_market_trade(&mut self, trade: &MarketTrade);
    async fn handle_trade_data(&mut self, trade: &MarketTrade) -> Result<(), ExchangeError>;

    async fn match_orders(&mut self, market_trade: &MarketTrade) -> Result<Vec<ClientTrade>, ExchangeError>;

    async fn fees_percent(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExchangeError>;
    /// 处理客户端交易列表并更新账户余额及交易事件。
    ///
    /// 该方法接收多个 `ClientTrade` 实例，并依次处理每笔交易：
    ///
    /// 1. 更新账户的相关余额信息。
    /// 2. 发送交易事件 `AccountEventKind::Trade`。
    /// 3. 发送余额更新事件 `AccountEventKind::Balance`。
    ///
    /// # 参数
    ///
    /// * `client_trades` - 一个包含多个 `ClientTrade` 实例的向量，表示客户端生成的交易记录。
    ///
    /// # 错误处理
    ///
    /// * 如果在应用交易变化时发生错误，会记录警告日志并继续处理下一笔交易。
    /// * 如果发送交易事件或余额事件失败，也会记录警告日志。
    ///
    /// # 注意事项
    ///
    /// * 当 `client_trades` 为空时，该方法不会执行任何操作。
    async fn process_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;

    async fn process_trades(&mut self, client_trades: Vec<ClientTrade>);
    fn update_exchange_ts(&self, timestamp: i64);
}

#[async_trait]
impl TradeHandler for SandboxAccount
{
    /// 创建或更新一个单级别的订单簿（SingleLevelOrderBook），基于传入的市场交易（MarketTrade）。
    ///
    /// # 参数
    /// - `trade`: 引用 `MarketTrade` 类型的交易信息，用于从中提取 `instrument` 并更新对应的订单簿。
    ///
    /// # 实现步骤
    /// 1. 从 `trade` 中解析出 `instrument`，如果解析失败，程序会 panic（`unwrap`）。
    /// 2. 获取 `single_level_order_book` 的互斥锁以安全地访问共享资源。
    /// 3. 使用 `instrument` 作为键，查找对应的单级别订单簿，如果没有则创建一个新的订单簿。
    /// 4. 使用 `trade` 更新该 `instrument` 对应的订单簿。
    ///
    /// # 备注
    /// - `SingleLevelOrderBook::from(trade)` 是一个基于 `trade` 初始化订单簿的工厂方法。
    /// - 该函数异步锁定了 `single_level_order_book`，并且通过 `.await` 实现对共享数据的安全访问。
    async fn create_or_single_level_orderbook_from_market_trade(&mut self, trade: &MarketTrade)
    {
        let instrument = trade.parse_instrument().unwrap();
        let mut orderbook = self.single_level_order_book.lock().await;

        orderbook.entry(instrument)
                 .or_insert_with(|| SingleLevelOrderBook::from(trade)) // 传递引用 &trade
                 .update_from_trade(&trade);
    }

    /// 处理交易数据的方法
    async fn handle_trade_data(&mut self, trade: &MarketTrade) -> Result<(), ExchangeError>
    {
        // 更新时间戳
        self.update_exchange_ts(trade.timestamp);
        // 更新单层OrderBook，注意 这个做法仅仅适用于回测。
        self.create_or_single_level_orderbook_from_market_trade(trade).await;
        // 用交易所记录的用户的挂单去匹配 market_rade 以实现模拟的目的
        self.check_and_handle_liquidation(trade).await?;
        self.match_orders(&trade).await?;
        Ok(())
    }

    /// 处理市场交易事件并尝试匹配订单。
    ///
    /// 该函数根据市场交易事件尝试匹配账户中的订单，并生成相应的交易。它会根据市场事件的方向（买或卖）
    /// 查找最佳报价，并使用预先计算的 `OrderRole` 来确定订单的费用比例。匹配成功的订单会生成相应的交易记录。
    ///
    /// # 参数
    ///
    /// - `market_trade`: 一个 [`MarketTrade`] 实例，表示来自市场的交易事件。
    ///
    /// # 返回值
    ///
    /// 返回一个包含所有匹配到的 [`ClientTrade`] 实例的向量。
    ///
    /// # 逻辑
    ///
    /// 1. 从市场交易事件中解析出基础货币和报价货币，并确定金融工具种类。
    /// 2. 查找与该金融工具相关的挂单（`InstrumentOrders`）。
    /// 3. 根据市场事件的方向（买或卖）尝试匹配相应的挂单（买单匹配卖单，卖单匹配买单）。
    /// 4. 使用订单的 `OrderRole` 来计算手续费，并生成交易记录。
    /// 5. 处理并返回生成的交易记录。
    ///
    /// # 注意
    /// 该函数假设市场交易事件的符号格式为 `base_quote`，并从中解析出基础货币和报价货币。
    /// 如果找不到与市场事件相关的挂单，函数会记录警告并返回一个空的交易向量。
    async fn match_orders(&mut self, market_trade: &MarketTrade) -> Result<Vec<ClientTrade>, ExchangeError>
    {
        // println!("[match_orders]: market_trade: {:?}", market_trade);
        let mut trades = Vec::new();

        // 从市场交易事件的符号中解析基础货币和报价货币，并确定金融工具种类
        let base = Token::from(market_trade.parse_base().ok_or_else(|| ExchangeError::SandBox("Unknown base.".to_string()))?);
        let quote = Token::from(market_trade.parse_quote().ok_or_else(|| ExchangeError::SandBox("Unknown quote.".to_string()))?);
        let kind = market_trade.parse_kind();
        // println!("[match_orders]: kind is {}", kind);
        let instrument = Instrument { base, quote, kind };
        // println!("[match_orders]: instrument is {}", instrument);

        // 查找与指定金融工具相关的挂单
        if let Ok(mut instrument_orders) = self.orders.read().await.get_ins_orders_mut(&instrument) {
            // 确定市场事件匹配的挂单方向（买或卖）
            if let Some(matching_side) = instrument_orders.determine_matching_side(market_trade) {
                // println!("[match_orders]: matching side is {}, will look up in corresponding open orders", matching_side);
                match matching_side {
                    | Side::Buy => {
                        // 从最佳买单中提取 `OrderRole` 以获取正确的手续费比例
                        if let Some(best_bid) = instrument_orders.bids.last() {
                            let order_role = best_bid.state.order_role;
                            // println!("[match_orders]: order_role: {:?}", order_role);
                            let fees_percent = self.fees_percent(&kind, order_role).await.map_err(|_| ExchangeError::SandBox("Missing fees.".to_string()))?;

                            // 使用计算出的手续费比例匹配买单
                            trades.append(&mut instrument_orders.match_bids(market_trade, fees_percent, &self.client_trade_counter));
                        }
                    }
                    | Side::Sell => {
                        // 从最佳卖单中提取 `OrderRole` 以获取正确的手续费比例
                        if let Some(best_ask) = instrument_orders.asks.last() {
                            let order_role = best_ask.state.order_role;
                            // println!("[match_orders]: order_role: {:?}", order_role);
                            let fees_percent = self.fees_percent(&kind, order_role).await.map_err(|_| ExchangeError::SandBox("Missing fees.".to_string()))?;

                            // 使用计算出的手续费比例匹配卖单
                            trades.append(&mut instrument_orders.match_asks(market_trade, fees_percent, &self.client_trade_counter));
                        }
                    }
                }
            }
        }
        else {
            // 记录日志并继续，不返回错误
            warn!("未找到与市场事件相关的挂单，跳过处理。");
        }

        // println!("[match_orders]: generated client trades are: {:?}", trades);
        self.process_trades(trades.clone()).await;

        Ok(trades)
    }

    /// 根据金融工具类型和订单角色返回相应的手续费百分比。 NOTE 需要扩展并支持现货和期货。
    ///
    /// # 参数
    ///
    /// * `kind` - 表示金融工具的种类，如 `Spot` 或 `Perpetual`。
    /// * `role` - 表示订单的角色，如 `Maker` 或 `Taker`。
    ///
    /// # 返回值
    ///
    /// * `Option<f64>` - 返回适用于指定金融工具类型和订单角色的手续费百分比。
    ///     - `Some(f64)` - 如果手续费配置存在，则返回对应的 `maker_fees` 或 `taker_fees`。
    ///     - `None` - 如果手续费配置不存在或金融工具类型不受支持，返回 `None`。
    ///
    /// # 注意事项
    ///
    /// * 目前只支持 `Spot` 和 `Perpetual` 类型的金融工具。
    /// * 如果传入的 `InstrumentKind` 不受支持，函数会记录一个警告并返回 `None`。
    async fn fees_percent(&self, instrument_kind: &InstrumentKind, role: OrderRole) -> Result<f64, ExchangeError>
    {
        // Access the account's config field
        match role {
            | OrderRole::Maker => {
                // Fetch the maker fee rate using AccountConfig's method
                self.config.get_maker_fee_rate(instrument_kind)
            }
            | OrderRole::Taker => {
                // Fetch the taker fee rate using AccountConfig's method
                self.config.get_taker_fee_rate(instrument_kind)
            }
        }
    }

    /// 处理客户端交易列表并更新账户余额及交易事件。
    ///
    /// 该方法接收多个 `ClientTrade` 实例，并依次处理每笔交易：
    ///
    /// 1. 更新账户的相关余额信息。
    /// 2. 发送交易事件 `AccountEventKind::Trade`。
    /// 3. 发送余额更新事件 `AccountEventKind::Balance`。
    ///
    /// # 参数
    ///
    /// * `client_trades` - 一个包含多个 `ClientTrade` 实例的向量，表示客户端生成的交易记录。
    ///
    /// # 错误处理
    ///
    /// * 如果在应用交易变化时发生错误，会记录警告日志并继续处理下一笔交易。
    /// * 如果发送交易事件或余额事件失败，也会记录警告日志。
    ///
    /// # 注意事项
    ///
    /// * 当 `client_trades` 为空时，该方法不会执行任何操作。
    async fn process_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError> {
        let exchange_timestamp = self.exchange_timestamp.load(Ordering::SeqCst);

        // 直接调用 `self.apply_trade_changes` 来处理余额更新
        let balance_event = match self.apply_trade_changes(&trade).await {
            Ok(event) => event,
            Err(err) => {
                warn!("Failed to update balance: {:?}", err);
                return Err(err);
            }
        };

        // 发送交易事件
        if let Err(err) = self.account_event_tx.send(AccountEvent {
            exchange_timestamp,
            exchange: Exchange::SandBox,
            kind: AccountEventKind::Trade(trade.clone()), // 发送交易事件
        }) {
            warn!("[UniLinkEx] : Client offline - Failed to send AccountEvent::Trade: {:?}", err);
        }

        // 发送余额更新事件
        if let Err(err) = self.account_event_tx.send(balance_event) {
            warn!("[UniLinkEx] : Client offline - Failed to send AccountEvent::Balance: {:?}", err);
        }

        Ok(())
    }

    async fn process_trades(&mut self, client_trades: Vec<ClientTrade>) {
        if !client_trades.is_empty() {
            for trade in client_trades {
                if let Err(err) = self.process_trade(trade).await {
                    warn!("Failed to process trade: {:?}", err);
                }
            }
        }
    }


    /// 更新交易所时间辍
    fn update_exchange_ts(&self, timestamp: i64)
    {
        let adjusted_timestamp = match self.config.execution_mode {
            | SandboxMode::Backtest => timestamp,                                                            // 在回测模式下使用传入的时间戳
            | SandboxMode::Online => SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64, // 在实时模式下使用当前时间
        };
        self.exchange_timestamp.store(adjusted_timestamp, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::order::{
            identification::{client_order_id::ClientOrderId, OrderId},
            order_instructions::OrderInstruction,
            states::{open::Open, request_cancel::RequestCancel, request_open::RequestOpen},
            Order,
        },
        sandbox::account::handlers::trade_handler::TradeHandler,
        test_utils::create_test_account,
    };

    #[tokio::test]
    async fn test_fail_to_cancel_limit_order_due_to_invalid_order_id()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        let invalid_cancel_request = Order { instruction: OrderInstruction::Cancel,
                                             exchange: Exchange::SandBox,
                                             instrument: instrument.clone(),
                                             timestamp: 1625247600000,
                                             cid: Some(ClientOrderId("validCID123".into())),
                                             side: Side::Buy,
                                             state: RequestCancel { id: Some(OrderId(99999)) } /* 无效的OrderId */ };

        let result = account.atomic_cancel(invalid_cancel_request.clone()).await;
        // println!("Result: {:?}", result);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExchangeError::OrderNotFound { client_order_id: invalid_cancel_request.cid.clone(),
                                                                       order_id: Some(OrderId(99999)) });
    }

    #[tokio::test]
    async fn test_match_market_event_with_open_order_sell_with_insufficient_balance()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 查询 USDT 和 ETH 余额并 clone，以避免借用冲突
        let initial_usdt_balance = account.get_balance(&Token::from("USDT")).unwrap().clone();
        let initial_eth_balance = account.get_balance(&Token::from("ETH")).unwrap().clone();

        println!("[test_match_market_event_with_open_order_sell] : Initial ETH balance: {:?}", initial_eth_balance);
        println!("[test_match_market_event_with_open_order_sell] : Initial USDT balance: {:?}", initial_usdt_balance);

        // 创建一个待开卖单订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID456".into())),
                                 side: Side::Sell,
                                 state: RequestOpen { reduce_only: false,
                                                      price: 16406.0,
                                                      size: 2.0 } };

        // 将订单添加到账户
        let result = account.atomic_open(open_order.clone()).await;
        assert_eq!(result.is_ok(), false);
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 16605.0,
                                         side: Side::Buy.to_string(),
                                         amount: 2.0 };

        // 匹配订单并生成交易事件
        let _ = account.match_orders(&market_event).await.unwrap();

        // 检查余额是否已更新
        let base_balance = account.get_balance(&instrument.base).unwrap();
        let quote_balance = account.get_balance(&instrument.quote).unwrap();

        assert_eq!(base_balance.total, 10.0);
        assert_eq!(base_balance.available, 10.0);
        assert_eq!(quote_balance.available, 10000.0); // 根本不能成交。若以不应该变。
        assert_eq!(quote_balance.total, 10000.0); // 根本不能成交。若以不应该变。
    }

    #[tokio::test]
    async fn test_match_market_event_with_open_order_sell_with_sufficient_balance()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
        account.deposit_usdt(50000.0).unwrap(); // 往测试里面充 50000 USDT.
                                                // 查询 USDT 和 ETH 余额并 clone，以避免借用冲突
        let initial_usdt_balance = account.get_balance(&Token::from("USDT")).unwrap().clone();
        let initial_eth_balance = account.get_balance(&Token::from("ETH")).unwrap().clone();

        println!("[test_match_market_event_with_open_order_sell] : Initial ETH balance: {:?}", initial_eth_balance);
        println!("[test_match_market_event_with_open_order_sell] : Initial USDT balance: {:?}", initial_usdt_balance);

        // 创建一个待开卖单订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID456".into())),
                                 side: Side::Sell,
                                 state: RequestOpen { reduce_only: false,
                                                      price: 16406.0,
                                                      size: 2.0 } };

        // 将订单添加到账户
        let result = account.atomic_open(open_order.clone()).await;
        assert_eq!(result.is_ok(), false);
        // let result = account.atomic_open(open_order).await;
        // assert_eq!(result.is_ok(), true);
        // // 创建一个市场事件，该事件与 open订单完全匹配
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 16605.0, // 前面已经确认是Maker单，成交计算的价格应该按照这里的 16605
                                         side: Side::Buy.to_string(),
                                         amount: 2.0 };

        // 匹配订单并生成交易事件
        let trades = account.match_orders(&market_event).await.unwrap();
        println!("trades:{:?}", trades);

        // 检查余额是否已更新 注意合约交易中base_balance不应该被改变
        let base_balance = account.get_balance(&instrument.base).unwrap();
        assert_eq!(base_balance.total, 10.0);
        assert_eq!(base_balance.available, 10.0);
        let quote_balance = account.get_balance(&instrument.quote).unwrap();
        assert_eq!(quote_balance.available, 27155.188); // Maker 价格
        assert_eq!(quote_balance.total, 59967.188); // NOTE this is correct remaining total
    }

    #[tokio::test]
    async fn test_get_open_orders_should_be_empty_after_matching()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 创建并添加订单
        let open_order = Order { instruction: OrderInstruction::Limit,
                                 exchange: Exchange::SandBox,
                                 instrument: instrument.clone(),
                                 timestamp: 1625247600000,
                                 cid: Some(ClientOrderId("validCID123".into())),
                                 side: Side::Buy,
                                 state: Open { id: OrderId::new(0, 0, 0),

                                               price: 100.0,
                                               size: 2.0,
                                               filled_quantity: 0.0,
                                               order_role: OrderRole::Maker } };
        account.orders.write().await.get_ins_orders_mut(&instrument).unwrap().add_order_open(open_order.clone());

        // 匹配一个完全匹配的市场事件
        let market_event = MarketTrade { exchange: "binance-futures".to_string(),
                                         symbol: "ETH_USDT".to_string(),
                                         timestamp: 1625247600000,
                                         price: 100.0,
                                         side: Side::Sell.to_string(),
                                         amount: 2.0 };
        let _ = account.match_orders(&market_event).await;

        // 获取未完成的订单
        let orders = account.orders.read().await.fetch_all();
        assert!(orders.is_empty(), "Expected no open orders after full match, but found some.");
    }

    #[tokio::test]
    async fn test_fail_to_open_limit_order_due_to_insufficient_funds()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));

        // 设置一个资金不足的场景，减少USDT的余额
        {
            let mut quote_balance = account.get_balance_mut(&instrument.quote).unwrap();
            quote_balance.available = 1.0; // 模拟 USDT 余额不足
        }

        // 创建一个待开买单订单
        let open_order_request = Order { instruction: OrderInstruction::Limit,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         timestamp: 1625247600000,
                                         cid: Some(ClientOrderId("validCID123".into())),
                                         side: Side::Buy,
                                         state: RequestOpen { price: 16499.0,
                                                              size: 5.0,
                                                              reduce_only: false } };

        let result = account.atomic_open(open_order_request).await;

        // 断言开单失败，且返回的错误是余额不足
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExchangeError::InsufficientBalance(instrument.quote));
    }

    #[tokio::test]
    async fn test_handle_trade_data()
    {
        let mut account = create_test_account().await;

        let trade = MarketTrade { exchange: "binance-futures".to_string(),
                                  symbol: "BTC_USDT".to_string(),
                                  timestamp: 1625247600000,
                                  price: 100.0,
                                  side: Side::Buy.to_string(),
                                  amount: 0.0 };

        // 处理交易数据
        let result = account.handle_trade_data(&trade).await;
        assert!(result.is_ok());

        // 验证时间戳是否已更新
        assert_eq!(account.get_exchange_ts().unwrap(), 1625247600000);
    }
}
