use crate::{
    common::{
        balance::{Balance, BalanceDelta, TokenBalance},
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            states::{open::Open, request_open::RequestOpen},
            Order,
        },
        token::Token,
        trade::ClientTrade,
        Side,
    },
    error::ExchangeError,
    sandbox::account::{respond, DashMapRefMut, SandboxAccount},
    Exchange,
};
use async_trait::async_trait;
use dashmap::mapref::one::Ref;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot::Sender;

#[async_trait]
pub trait BalanceHandler
{
    async fn get_balances(&self) -> Vec<TokenBalance>;
    /// 返回指定[`Token`]的[`Balance`]的引用。
    fn get_balance(&self, token: &Token) -> Result<Ref<Token, Balance>, ExchangeError>;
    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    fn get_balance_mut(&mut self, token: &Token) -> Result<DashMapRefMut<'_, Token, Balance>, ExchangeError>;
    async fn fetch_token_balances_and_respond(&self, response_tx: Sender<Result<Vec<TokenBalance>, ExchangeError>>);
    async fn fetch_token_balance_and_respond(&self, token: &Token, response_tx: Sender<Result<TokenBalance, ExchangeError>>);
    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    async fn apply_open_order_changes(&mut self, open: &Order<Open>, required_balance: f64) -> Result<AccountEvent, ExchangeError>;
    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    fn apply_cancel_order_changes(&mut self, cancelled: &Order<Open>) -> Result<AccountEvent, ExchangeError>;
    /// 从交易中更新余额并返回 [`AccountEvent`]
    async fn apply_trade_changes(&mut self, trade: &ClientTrade) -> Result<AccountEvent, ExchangeError>;
    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    fn apply_balance_delta(&mut self, token: &Token, delta: BalanceDelta) -> Balance;
    async fn required_available_balance<'a>(&'a self, order: &'a Order<RequestOpen>) -> Result<(&'a Token, f64), ExchangeError>;
    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExchangeError>;
}

#[async_trait]
impl BalanceHandler for SandboxAccount
{
    async fn get_balances(&self) -> Vec<TokenBalance>
    {
        self.balances.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
    }

    /// 返回指定[`Token`]的[`Balance`]的引用。
    fn get_balance(&self, token: &Token) -> Result<Ref<Token, Balance>, ExchangeError>
    {
        self.balances
            .get(token)
            .ok_or_else(|| ExchangeError::SandBox(format!("SandBoxExchange is not configured for Token: {:?}", token)))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    fn get_balance_mut(&mut self, token: &Token) -> Result<DashMapRefMut<'_, Token, Balance>, ExchangeError>
    {
        self.balances
            .get_mut(token)
            .ok_or_else(|| ExchangeError::SandBox(format!("SandBoxExchange is not configured for Token: {:?}", token)))
    }

    async fn fetch_token_balances_and_respond(&self, response_tx: Sender<Result<Vec<TokenBalance>, ExchangeError>>)
    {
        let balances = self.get_balances().await;
        respond(response_tx, Ok(balances));
    }

    async fn fetch_token_balance_and_respond(&self, token: &Token, response_tx: Sender<Result<TokenBalance, ExchangeError>>)
    {
        let balance_ref = self.get_balance(token).unwrap();
        let token_balance = TokenBalance::new(token.clone(), *balance_ref);
        respond(response_tx, Ok(token_balance));
    }

    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    async fn apply_open_order_changes(&mut self, open: &Order<Open>, required_balance: f64) -> Result<AccountEvent, ExchangeError>
    {
        println!("[apply_open_order_changes] : applying open order: {:?}, subtracting required_balance: {:?}", open, required_balance);

        // 根据 PositionMarginMode 处理余额更新 注意 : 暂时不支持spot的仓位逻辑
        match open.instrument.kind {
            | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken => {
                let delta = BalanceDelta { total: 0.0,
                                           available: -required_balance };
                self.apply_balance_delta(&open.instrument.quote, delta)
            }
            | _ => {
                return Err(ExchangeError::SandBox(format!("[UniLinkEx] : Unsupported InstrumentKind or PositionMarginMode for open order: {:?}", open.instrument.kind)));
            }
        };

        // 更新后的余额
        let updated_balance = match open.side {
            | Side::Buy => *self.get_balance(&open.instrument.quote)?,
            | Side::Sell => *self.get_balance(&open.instrument.base)?,
        };

        Ok(AccountEvent { exchange_timestamp: self.exchange_timestamp.load(Ordering::SeqCst),
                          exchange: Exchange::SandBox,
                          kind: AccountEventKind::Balance(TokenBalance::new(open.instrument.quote.clone(), updated_balance)) })
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    fn apply_cancel_order_changes(&mut self, cancelled: &Order<Open>) -> Result<AccountEvent, ExchangeError>
    {
        let updated_balance = match cancelled.side {
            | Side::Buy => {
                let mut balance = self.get_balance_mut(&cancelled.instrument.quote).expect("[UniLinkEx] : Balance existence checked when opening Order");
                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                *balance
            }
            | Side::Sell => {
                let mut balance = self.get_balance_mut(&cancelled.instrument.base).expect("[UniLinkEx] : Balance existence checked when opening Order");
                balance.available += cancelled.state.remaining_quantity();
                *balance
            }
        };

        // 根据 `Side` 确定使用 `base` 或 `quote` 作为 `Token`
        let token = match cancelled.side {
            | Side::Buy => cancelled.instrument.quote.clone(),
            | Side::Sell => cancelled.instrument.base.clone(),
        };

        Ok(AccountEvent { exchange_timestamp: self.exchange_timestamp.load(Ordering::SeqCst),
                          exchange: Exchange::SandBox,
                          kind: AccountEventKind::Balance(TokenBalance::new(token, updated_balance)) })
    }

    /// 从交易中更新余额并返回 [`AccountEvent`]
    async fn apply_trade_changes(&mut self, trade: &ClientTrade) -> Result<AccountEvent, ExchangeError>
    {
        println!("[apply_trade_changes] : applying trade: {:?}", trade);
        let Instrument { quote, kind, .. } = &trade.instrument;
        let fee = trade.fees; // 直接从 TradeEvent 中获取费用
        let side = trade.side; // 直接使用 TradeEvent 中的 side
                               // let trade_price = trade.price;
                               // let trade_quantity = trade.quantity;

        match kind {
            | InstrumentKind::Spot => {
                let base = &trade.instrument.base;
                let (base_delta, quote_delta) = match side {
                    | Side::Buy => {
                        let base_increase = trade.size;
                        // Note: available was already decreased by the opening of the Side::Buy order
                        let base_delta = BalanceDelta { total: base_increase,
                                                        available: base_increase };
                        let quote_delta = BalanceDelta { total: -trade.size * trade.price - fee,
                                                         available: -fee };
                        (base_delta, quote_delta)
                    }
                    | Side::Sell => {
                        // Note: available was already decreased by the opening of the Side::Sell order
                        let base_delta = BalanceDelta { total: -trade.size, available: 0.0 };
                        let quote_increase = (trade.size * trade.price) - fee;
                        let quote_delta = BalanceDelta { total: quote_increase,
                                                         available: quote_increase };
                        (base_delta, quote_delta)
                    }
                };

                let base_balance = self.apply_balance_delta(base, base_delta);
                let quote_balance = self.apply_balance_delta(quote, quote_delta);

                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().expect("[UniLinkEx] : Failed to get exchange timestamp"),
                                  exchange: Exchange::SandBox,
                                  kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance),]) })
            }
            | InstrumentKind::CryptoOption => {
                todo!("Option handling is not implemented yet");
            }
            | InstrumentKind::CommodityOption => {
                todo!("CommodityOption handling is not implemented yet")
            }
            | InstrumentKind::CommodityFuture => {
                todo!("CommodityFuture handling is not implemented yet")
            }
            | InstrumentKind::Perpetual | InstrumentKind::Future | InstrumentKind::CryptoLeveragedToken => {
                let leverage_rate = self.config.global_leverage_rate;
                let quote_delta = match side {
                    | Side::Buy => {
                        // 买入时减少的也是 quote 资金
                        BalanceDelta { total: -fee * leverage_rate,
                                       available: -fee * leverage_rate }
                    }
                    | Side::Sell => {
                        // 卖出时增加的也是 quote 资金
                        BalanceDelta { total: -fee * leverage_rate,
                                       available: -fee * leverage_rate }
                    }
                };

                println!("[apply_trade_changes] : quote_delta: {:?}", quote_delta);
                // 应用 quote 的余额变动
                let quote_balance = self.apply_balance_delta(quote, quote_delta);

                // 生成账户事件，只涉及 quote
                Ok(AccountEvent { exchange_timestamp: self.get_exchange_ts().expect("[UniLinkEx] : Failed to get exchange timestamp"),
                                  exchange: Exchange::SandBox,
                                  kind: AccountEventKind::Balances(vec![TokenBalance::new(quote.clone(), quote_balance),]) })
            }
        }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    fn apply_balance_delta(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let mut base_balance = self.get_balance_mut(token).unwrap();

        let _ = base_balance.apply(delta);

        *base_balance
    }

    async fn required_available_balance<'a>(&'a self, order: &'a Order<RequestOpen>) -> Result<(&'a Token, f64), ExchangeError>
    {
        // 从 AccountConfig 读取 max_price_deviation
        let max_price_deviation = self.config.max_price_deviation;
        println!("[required_available_balance] : max_price_deviation is {:?}", max_price_deviation);

        // 将锁定的 order_book 引用存储在一个变量中，确保其生命周期足够长
        let mut order_books_lock = self.single_level_order_book.lock().await;
        let order_book = order_books_lock.get_mut(&order.instrument).unwrap();

        match order.instrument.kind {
            // Spot 交易
            | InstrumentKind::Spot => {
                let latest_ask = order_book.latest_ask;
                let latest_bid = order_book.latest_bid;

                match order.side {
                    | Side::Buy => {
                        // 确保买单价格不比最新卖价低
                        if order.state.price < latest_ask * (1.0 - max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Buy order price is too low compared to the market".into()));
                        }
                        // 确保买单价格不比最新买价高
                        if order.state.price > latest_bid * (1.0 + max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Buy order price is too high compared to the market".into()));
                        }
                        let required_balance = latest_ask * order.state.size;
                        Ok((&order.instrument.quote, required_balance))
                    }
                    | Side::Sell => {
                        // 确保卖单价格不比最新买价高
                        if order.state.price > latest_bid * (1.0 + max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Sell order price is too high compared to the market".into()));
                        }
                        // 确保卖单价格不比最新卖价低
                        if order.state.price < latest_ask * (1.0 - max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Sell order price is too low compared to the market".into()));
                        }
                        let required_balance = latest_bid * order.state.size;
                        Ok((&order.instrument.base, required_balance))
                    }
                }
            }
            // Perpetual 和 Future 合约类型
            | InstrumentKind::Perpetual | InstrumentKind::Future => {
                let latest_ask = order_book.latest_ask;
                let latest_bid = order_book.latest_bid;

                match order.side {
                    | Side::Buy => {
                        if order.state.price < latest_ask * (1.0 - max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Buy order price is too low compared to the market".into()));
                        }
                        if order.state.price > latest_bid * (1.0 + max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Buy order price is too high compared to the market".into()));
                        }
                        let required_balance = order.state.price * order.state.size * self.config.global_leverage_rate;
                        Ok((&order.instrument.quote, required_balance))
                    }
                    | Side::Sell => {
                        if order.state.price > latest_bid * (1.0 + max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Sell order price is too high compared to the market".into()));
                        }
                        if order.state.price < latest_ask * (1.0 - max_price_deviation) {
                            return Err(ExchangeError::OrderRejected("Sell order price is too low compared to the market".into()));
                        }
                        let required_balance = order.state.price * order.state.size * self.config.global_leverage_rate;
                        Ok((&order.instrument.quote, required_balance))
                    }
                }
            }
            // 其他类型待实现
            | InstrumentKind::CryptoOption => {
                todo!("CryptoOption is not supported yet")
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!("CryptoLeveragedToken is not supported yet")
            }
            | InstrumentKind::CommodityOption => {
                todo!("CommodityOption is not supported yet")
            }
            | InstrumentKind::CommodityFuture => {
                todo!("CommodityFuture is not supported yet")
            }
        }
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExchangeError>
    {
        let available = self.get_balance(token)?.available;
        if available >= required_balance {
            println!("[has_sufficient_available_balance] : client does have sufficient balance");
            Ok(())
        }
        else {
            Err(ExchangeError::InsufficientBalance(token.clone()))
        }
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
            states::request_open::RequestOpen,
            OrderRole,
        },
        sandbox::account::account_handlers::{position_handler::PositionHandler, trade_handler::TradeHandler},
        test_utils::create_test_account,
    };

    #[tokio::test]
    async fn test_get_balance()
    {
        let account = create_test_account().await;

        let token = Token::from("ETH");
        let balance = account.get_balance(&token).unwrap();
        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 10.0);
    }

    #[tokio::test]
    async fn test_get_balance_mut()
    {
        let mut account = create_test_account().await;

        let token = Token::from("ETH");
        let balance = account.get_balance_mut(&token).unwrap();
        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 10.0);
    }

    #[tokio::test]
    async fn test_get_fee()
    {
        let account = create_test_account();
        let fee = account.await.fees_percent(&InstrumentKind::Perpetual, OrderRole::Maker).await.unwrap();
        assert_eq!(fee, 0.001);
    }

    #[tokio::test]
    async fn test_apply_cancel_order_changes()
    {
        let mut account = create_test_account().await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
                            timestamp: 1625247600000,
                            cid: Some(ClientOrderId("validCID123".into())),
                            side: Side::Buy,
                            state: Open { id: OrderId::new(0, 0, 0),
                                          price: 100.0,
                                          size: 2.0,
                                          filled_quantity: 0.0,
                                          order_role: OrderRole::Maker } };

        let balance_before = account.get_balance(&Token::from("USDT")).unwrap().available;
        let account_event = account.apply_cancel_order_changes(&order).unwrap();

        // 从 AccountEvent 提取 TokenBalance
        if let AccountEventKind::Balance(token_balance) = account_event.kind {
            // 验证余额是否已更新
            assert_eq!(token_balance.balance.available, balance_before + 200.0);
        }
        else {
            panic!("Expected AccountEventKind::Balance");
        }
    }

    #[tokio::test]
    async fn test_fetch_all_balances()
    {
        let account = create_test_account().await;

        let all_balances = account.get_balances().await;

        assert_eq!(all_balances.len(), 2, "Expected 2 balances but got {}", all_balances.len());

        assert!(all_balances.iter().any(|b| b.token == Token::from("ETH")), "Expected ETH balance not found");
        assert!(all_balances.iter().any(|b| b.token == Token::from("USDT")), "Expected USDT balance not found");
    }

    #[tokio::test]
    async fn test_get_position_none()
    {
        let account = create_test_account().await;
        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
        let position = account.get_position_long(&instrument).await.unwrap();
        // 这是因为create_test_account()没有内建任何仓位
        assert!(position.is_none());
    }
    #[tokio::test]
    async fn test_required_available_balance_with_insufficient_bid()
    {
        let account = create_test_account().await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
                            timestamp: 1625247600000,
                            cid: Some(ClientOrderId("validCID123".into())),
                            side: Side::Buy,
                            state: RequestOpen { price: 100.0, // 设置一个低于市场价格的买单
                                                 size: 2.0,
                                                 reduce_only: false } };

        match account.required_available_balance(&order).await {
            | Ok((_token, _required_balance)) => {
                // 这里不应该触发，因为订单价格太低应被拒绝
                panic!("Test should have failed due to insufficient bid price but has not");
            }
            | Err(e) => {
                // 订单应该因价格过低而被拒绝
                assert_eq!(e.to_string(), "[UniLinkEx] : Order rejected: Buy order price is too low compared to the market");
            }
        }
    }

    #[tokio::test]
    async fn test_required_available_balance_with_sufficient_bid()
    {
        let account = create_test_account().await;

        let order = Order { instruction: OrderInstruction::Limit,
                            exchange: Exchange::SandBox,
                            instrument: Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
                            timestamp: 1625247600000,
                            cid: Some(ClientOrderId("validCID123".into())),
                            side: Side::Buy,
                            state: RequestOpen { price: 16499.0,
                                                 size: 2.0,
                                                 reduce_only: false } };

        match account.required_available_balance(&order).await {
            | Ok((token, required_balance)) => {
                println!("{} {}", token, required_balance);
                assert_eq!(token, &order.instrument.quote);
                assert_eq!(required_balance, 32998.0);
            }
            | Err(e) => {
                panic!("Test failed with error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_has_sufficient_available_balance()
    {
        let account = create_test_account().await;

        let token = Token::from("ETH");
        let result = account.has_sufficient_available_balance(&token, 5.0);
        assert!(result.is_ok());

        let result = account.has_sufficient_available_balance(&token, 15.0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_balance_delta()
    {
        let mut account = create_test_account().await;

        let token = Token::from("ETH");
        let delta = BalanceDelta::new(0.0, -10.0);

        let balance = account.apply_balance_delta(&token, delta);

        assert_eq!(balance.total, 10.0);
        assert_eq!(balance.available, 0.0);
    }

    #[tokio::test]
    async fn test_apply_open_order_changes_buy()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
        let open_order_request = Order { instruction: OrderInstruction::Limit,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         timestamp: 1625247600000,
                                         cid: Some(ClientOrderId("validCID123".into())),
                                         side: Side::Buy,
                                         state: RequestOpen { price: 1.0,
                                                              size: 2.0,
                                                              reduce_only: false } };

        // 将订单状态从 RequestOpen 转换为 Open
        let open_order = Order { instruction: open_order_request.instruction,
                                 exchange: open_order_request.exchange,
                                 instrument: open_order_request.instrument.clone(),
                                 timestamp: open_order_request.timestamp,
                                 cid: open_order_request.cid.clone(),
                                 side: open_order_request.side,
                                 state: Open { id: OrderId::new(0, 0, 0), // 使用一个新的 OrderId

                                               price: open_order_request.state.price,
                                               size: open_order_request.state.size,
                                               filled_quantity: 0.0,
                                               order_role: OrderRole::Maker } };

        let required_balance = 2.0; // 模拟需要的余额

        let result = account.apply_open_order_changes(&open_order, required_balance).await;
        assert!(result.is_ok());

        let balance = account.get_balance(&Token::from("USDT")).unwrap();
        assert_eq!(balance.available, 9998.0); // 原始余额是 10,000.0，减去 2.0 后应该是 9998.0
    }

    #[tokio::test]
    async fn test_apply_open_order_changes_sell()
    {
        let mut account = create_test_account().await;

        let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
        let open_order_request = Order { instruction: OrderInstruction::Limit,
                                         exchange: Exchange::SandBox,
                                         instrument: instrument.clone(),
                                         timestamp: 1625247600000,
                                         cid: Some(ClientOrderId("validCID123".into())),
                                         side: Side::Sell,
                                         state: RequestOpen { price: 1.0,
                                                              size: 2.0,
                                                              reduce_only: false } };

        // 将订单状态从 RequestOpen 转换为 Open
        let open_order = Order { instruction: open_order_request.instruction,
                                 exchange: open_order_request.exchange,
                                 instrument: open_order_request.instrument.clone(),
                                 timestamp: open_order_request.timestamp,
                                 cid: open_order_request.cid.clone(),
                                 side: open_order_request.side,
                                 state: Open { id: OrderId::new(0, 0, 0), // 使用一个新的 OrderId

                                               price: open_order_request.state.price,
                                               size: open_order_request.state.size,
                                               filled_quantity: 0.0,
                                               order_role: OrderRole::Maker } };

        let required_balance = 2.0; // 模拟需要的余额

        let result = account.apply_open_order_changes(&open_order, required_balance).await;
        assert!(result.is_ok());

        let balance = account.get_balance(&Token::from("USDT")).unwrap();
        assert_eq!(balance.available, 9998.0); // 原始余额是 10000.0，减去 2.0 后应该是 9998.0
    }
}
