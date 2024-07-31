use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use tokio::sync::RwLock;

use crate::{
    common_skeleton::{
        balance::{Balance, BalanceDelta, TokenBalance},
        datafeed::event::MarketEvent,
        event::{AccountEvent, AccountEventKind},
        instrument::{kind::InstrumentKind, Instrument},
        order::{Open, Order},
        token::Token,
        Side,
    },
    error::ExecutionError,
    simulated_exchange::{
        account::{
            account_config::{MarginMode, PositionMode},
            Account,
        },
        load_from_clickhouse::queries_operations::ClickhouseTrade,
    },
    ExchangeVariant,
};

#[derive(Clone, Debug)]
pub struct AccountBalances<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord + Ord
{
    pub balance_map: HashMap<Token, Balance>,
    pub account_ref: Weak<RwLock<Account<Event>>>, // NOTE :如果不使用弱引用，可能会导致循环引用和内存泄漏。
}

impl<Event> PartialEq for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn eq(&self, other: &Self) -> bool
    {
        self.balance_map == other.balance_map
        // account_ref 是Weak<RwLock<>>，一般不会比较其内容
    }
}

impl<Event> AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    /// 返回指定[`Token`]的[`Balance`]的引用。
    pub fn balance(&self, token: &Token) -> Result<&Balance, ExecutionError>
    {
        self.balance_map
            .get(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// 返回指定[`Token`]的[`Balance`]的可变引用。
    pub fn balance_mut(&mut self, token: &Token) -> Result<&mut Balance, ExecutionError>
    {
        self.balance_map
            .get_mut(token)
            .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for Token: {token}")))
    }

    /// Sets the account reference.
    pub fn set_account(&mut self, account: Arc<RwLock<Account<Event>>>)
    {
        self.account_ref = Arc::downgrade(&account);
    }

    /// 获取指定 [`InstrumentKind`] 的手续费。
    pub async fn get_fee(&self, instrument_kind: &InstrumentKind) -> Result<f64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let config_read = account_read.config.read().await;
            config_read.fees_book
                       .get(instrument_kind)
                       .cloned()
                       .ok_or_else(|| ExecutionError::Simulated(format!("SimulatedExchange is not configured for InstrumentKind: {:?}", instrument_kind)))
        }
        else {
            Err(ExecutionError::Simulated("Account reference is not set".to_string()))
        }
    }

    // 异步方法来获取 Account 的某个字段
    pub async fn get_exchange_ts(&self) -> Result<i64, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            Ok(account_read.exchange_timestamp)
        }
        else {
            Err(ExecutionError::Simulated("Account reference is not set".to_string()))
        }
    }

    /// 获取所有[`Token`]的[`Balance`]。
    pub fn fetch_all(&self) -> Vec<TokenBalance>
    {
        self.balance_map.clone().into_iter().map(|(token, balance)| TokenBalance::new(token, balance)).collect()
    }

    /// 判断client是否有足够的可用[`Balance`]来执行[`Order<RequestOpen>`]。
    pub fn has_sufficient_available_balance(&self, token: &Token, required_balance: f64) -> Result<(), ExecutionError>
    {
        let available = self.balance(token)?.available;
        if available >= required_balance {
            Ok(())
        }
        else {
            Err(ExecutionError::InsufficientBalance(token.clone()))
        }
    }


    /// 判断Account的当前持仓模式。
    #[allow(dead_code)]
    async fn determine_position_mode(&self) -> Result<PositionMode, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let config_read = account_read.config.read().await;
            Ok(config_read.position_mode.clone())
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }


    /// 判断Account的当前保证金模式。
    #[allow(dead_code)]
    async fn determine_margin_mode(&self) -> Result<MarginMode, ExecutionError>
    {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let config_read = account_read.config.read().await;
            Ok(config_read.margin_mode.clone())
        }
        else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }

    /// Check if there is already some position of this instrument in the AccountPositions
    /// need to determine InstrumentKind from the open order first as position types vary
    pub async fn is_position_open(&self, open: &Order<Open>) -> Result<bool, ExecutionError> {
        if let Some(account) = self.account_ref.upgrade() {
            let account_read = account.read().await;
            let positions_read = account_read.positions.read().await;
            for positions in positions_read.iter() {
                if positions.has_position(&open.instrument) {
                    return Ok(true);
                }
            }
            Ok(false)
        } else {
            Err(ExecutionError::Simulated("[UniLink_Execution] : Account reference is not set".to_string()))
        }
    }


    /// 当client创建[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub async fn update_from_open(&mut self, open: &Order<Open>, required_balance: f64) -> AccountEvent
    {
        let updated_balance = match open.side {
            | Side::Buy => {
                let balance = self.balance_mut(&open.instrument.quote).expect("[UniLink_Execution] : Balance existence is questionable");
                balance.available -= required_balance;
                TokenBalance::new(open.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&open.instrument.base).expect("[UniLink_Execution] : Balance existence is questionable");
                balance.available -= required_balance;
                TokenBalance::new(open.instrument.base.clone(), *balance)
            }
        };

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                       exchange: ExchangeVariant::Simulated,
                       kind: AccountEventKind::Balance(updated_balance) }
    }

    /// 当client取消[`Order<Open>`]时，更新相关的[`Token`] [`Balance`]。
    /// [`Balance`]的变化取决于[`Order<Open>`]是[`Side::Buy`]还是[`Side::Sell`]。
    pub fn update_from_cancel(&mut self, cancelled: &Order<Open>) -> TokenBalance
    {
        match cancelled.side {
            | Side::Buy => {
                let balance = self.balance_mut(&cancelled.instrument.quote)
                                  .expect("[UniLink_Execution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.price * cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.quote.clone(), *balance)
            }
            | Side::Sell => {
                let balance = self.balance_mut(&cancelled.instrument.base)
                                  .expect("[UniLink_Execution] : Balance existence checked when opening Order");
                balance.available += cancelled.state.remaining_quantity();
                TokenBalance::new(cancelled.instrument.base.clone(), *balance)
            }
        }
    }

    /// 从交易中更新余额并返回 [`AccountEvent`]
    /// NOTE 注意[ClickhouseTrade]行情数据和此处所需Trade是否兼容。
    pub async fn update_from_trade(&mut self, market_event: &MarketEvent<ClickhouseTrade>) -> AccountEvent
    {
        let Instrument { base, quote, kind, .. } = &market_event.instrument;
        let fee = self.get_fee(kind).await.unwrap_or(0.0);
        let side = market_event.kind.parse_side();
        let (base_delta, quote_delta) = match side {
            | Side::Buy => {
                let base_increase = market_event.kind.amount - fee;
                // Note: available was already decreased by the opening of the Side::Buy order
                let base_delta = BalanceDelta { total: base_increase,
                                                available: base_increase };
                let quote_delta = BalanceDelta { total: -market_event.kind.amount * market_event.kind.price,
                                                 available: 0.0 };
                (base_delta, quote_delta)
            }
            | Side::Sell => {
                // Note: available was already decreased by the opening of the Side::Sell order
                let base_delta = BalanceDelta { total: -market_event.kind.amount,
                                                available: 0.0 };
                let quote_increase = (market_event.kind.amount * market_event.kind.price) - fee;
                let quote_delta = BalanceDelta { total: quote_increase,
                                                 available: quote_increase };
                (base_delta, quote_delta)
            }
        };

        let base_balance = self.update(base, base_delta);
        let quote_balance = self.update(quote, quote_delta);

        AccountEvent { exchange_timestamp: self.get_exchange_ts().await.expect("[UniLink_Execution] : Failed to get exchange timestamp"),
                       exchange: ExchangeVariant::Simulated,
                       kind: AccountEventKind::Balances(vec![TokenBalance::new(base.clone(), base_balance), TokenBalance::new(quote.clone(), quote_balance),]) }
    }

    /// 将 [`BalanceDelta`] 应用于指定 [`Token`] 的 [`Balance`]，并返回更新后的 [`Balance`] 。
    pub fn update(&mut self, token: &Token, delta: BalanceDelta) -> Balance
    {
        let base_balance = self.balance_mut(token).unwrap();

        base_balance.apply(delta);

        *base_balance
    }
}

impl<Event> Deref for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    type Target = HashMap<Token, Balance>;

    fn deref(&self) -> &Self::Target
    {
        &self.balance_map
    }
}

impl<Event> DerefMut for AccountBalances<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.balance_map
    }
}

#[cfg(test)]
mod tests
{
    use std::sync::Arc;

    use tokio::sync::mpsc::unbounded_channel;
    use uuid::Uuid;

    use crate::{
        common_skeleton::{
            datafeed::event::MarketEvent,
            event::ClientOrderId,
            order::{OrderKind, OrderRole},
        },
        simulated_exchange::{
            account::{
                account_config::{AccountConfig, CommissionLevel, CommissionRates, MarginMode, PositionMode},
                account_latency::{AccountLatency, FluctuationMode},
                account_market_feed::AccountDataStreams,
                account_orders::AccountOrders,
                Account,
            },
            load_from_clickhouse::queries_operations::ClickhouseTrade,
        },
    };

    use super::*;
    #[allow(dead_code)]
    async fn create_test_account() -> Arc<RwLock<Account<MarketEvent<ClickhouseTrade>>>>
    {
        let (account_event_tx, _account_event_rx) = unbounded_channel();
        let (market_event_tx, _market_event_rx) = unbounded_channel();

        let instruments = vec![]; // Populate with test data if needed
        let account_latency = AccountLatency { fluctuation_mode: FluctuationMode::None,
                                               maximum: 100,
                                               minimum: 0,
                                               current_value: 50 };

        Arc::new(RwLock::new(Account { exchange_timestamp: 0,
                                       data: Arc::new(RwLock::new(AccountDataStreams::new())),
                                       account_event_tx,
                                       market_event_tx,
                                       config: Arc::new(RwLock::new(AccountConfig { margin_mode: MarginMode::SimpleMode,
                                                                                    position_mode: PositionMode::LongShortMode,
                                                                                    commission_level: CommissionLevel::Lv3,
                                                                                    current_commission_rate: CommissionRates { spot_maker: 0.001,
                                                                                                                               spot_taker: 0.002,
                                                                                                                               perpetual_open: 0.001,
                                                                                                                               perpetual_close: 0.002 },
                                                                                    leverage_book: HashMap::new(),
                                                                                    fees_book: HashMap::new() })),
                                       balances: Arc::new(RwLock::new(AccountBalances { balance_map: HashMap::new(),
                                                                                        account_ref: Weak::new() })),
                                       positions: Arc::new(RwLock::new(Vec::new())),
                                       orders: Arc::new(RwLock::new(AccountOrders::new(instruments, account_latency).await)) }))
    }

    #[tokio::test]
    async fn test_balance()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 100.0);
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let balances = AccountBalances { balance_map, account_ref };

        assert_eq!(balances.balance(&token).unwrap().available, 100.0);
    }

    #[tokio::test]
    async fn test_balance_mut()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 100.0);
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let mut balances = AccountBalances { balance_map, account_ref };

        {
            let balance_mut = balances.balance_mut(&token).unwrap();
            balance_mut.available = 50.0;
        }

        assert_eq!(balances.balance(&token).unwrap().available, 50.0);
    }

    #[tokio::test]
    async fn test_get_fee()
    {
        let instrument_kind = InstrumentKind::Spot;
        let mut fees_book = HashMap::new();
        fees_book.insert(instrument_kind.clone(), 0.1);

        let account = create_test_account().await;
        account.write().await.config.write().await.fees_book = fees_book;

        let account_ref = Arc::downgrade(&account);

        let balances = AccountBalances { balance_map: HashMap::new(),
                                         account_ref };

        let fee = balances.get_fee(&instrument_kind).await.unwrap();
        assert_eq!(fee, 0.1);
    }

    #[tokio::test]
    async fn test_get_exchange_ts()
    {
        let account = create_test_account().await;
        account.write().await.exchange_timestamp = 1627843987;

        let account_ref = Arc::downgrade(&account);

        let balances = AccountBalances { balance_map: HashMap::new(),
                                         account_ref };

        let exchange_ts = balances.get_exchange_ts().await.unwrap();
        assert_eq!(exchange_ts, 1627843987);
    }

    #[tokio::test]
    async fn test_fetch_all()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 100.0);
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let balances = AccountBalances { balance_map, account_ref };

        let all_balances = balances.fetch_all();
        assert_eq!(all_balances.len(), 1);
        assert_eq!(all_balances[0].balance.available, 100.0);
    }

    #[tokio::test]
    async fn test_has_sufficient_available_balance()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 100.0);
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let balances = AccountBalances { balance_map, account_ref };

        assert!(balances.has_sufficient_available_balance(&token, 50.0).is_ok());
        assert!(balances.has_sufficient_available_balance(&token, 150.0).is_err());
    }

    /// TODO code from this line on are tests for key methods of [AccountBalances] needs to be tested
    #[tokio::test]
    async fn test_update_from_open()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 100.0);
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let mut balances = AccountBalances { balance_map, account_ref };

        let instrument = Instrument::new(token.clone(), token.clone(), InstrumentKind::Spot);
        let client_order_id = Uuid::new_v4();
        let open_state = Open { id: client_order_id.into(),
                                price: 50000.0,
                                size: 1.0,
                                filled_quantity: 0.0,
                                order_role: OrderRole::Maker,
                                received_ts: 0 };
        let order = Order { kind: OrderKind::Limit,
                            exchange: ExchangeVariant::Simulated,
                            instrument: instrument.clone(),
                            client_ts: 0,
                            cid: ClientOrderId(client_order_id.clone()),
                            side: Side::Buy,
                            state: open_state };
        let account_event = balances.update_from_open(&order, 50.0).await;

        assert_eq!(balances.balance(&token).unwrap().available, 50.0);
        if let AccountEventKind::Balance(token_balance) = account_event.kind {
            assert_eq!(token_balance.balance.available, 50.0);
        }
        else {
            panic!("Unexpected account event kind");
        }
    }
    #[tokio::test]
    async fn test_update_from_cancel()
    {
        let token = Token::new("BTC");
        let balance = Balance::new(100.0, 50.0); // Initial total balance: 100, available balance: 50
        let mut balance_map = HashMap::new();
        balance_map.insert(token.clone(), balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let mut balances = AccountBalances { balance_map, account_ref };

        let instrument = Instrument { base: token.clone(),
                                      quote: token.clone(),
                                      kind: InstrumentKind::Spot };
        let client_order_id = Uuid::new_v4();

        let open_state = Open { id: client_order_id.into(),
                                price: 50000.0,
                                size: 1.0,
                                filled_quantity: 0.0,
                                order_role: OrderRole::Maker,
                                received_ts: 0 };
        let order = Order { kind: OrderKind::Limit,
                            exchange: ExchangeVariant::Simulated,
                            instrument: instrument.clone(),
                            client_ts: 0,
                            cid: ClientOrderId(client_order_id.clone()),
                            side: Side::Buy,
                            state: open_state };

        let token_balance = balances.update_from_cancel(&order);

        assert_eq!(balances.balance(&token).unwrap().available, 50.0 + (50000.0 * 1.0)); // 50050.0
        assert_eq!(token_balance.balance.available, 50.0 + (50000.0 * 1.0)); // 50050.0
    }

    #[tokio::test]
    async fn test_update_from_trade()
    {
        let base_token = Token::new("BTC");
        let quote_token = Token::new("USDT");

        let base_balance = Balance::new(1.0, 1.0); // 初始余额: 1 BTC
        let quote_balance = Balance::new(50000.0, 50000.0); // 初始余额: 50,000 USDT

        let mut balance_map = HashMap::new();
        balance_map.insert(base_token.clone(), base_balance);
        balance_map.insert(quote_token.clone(), quote_balance);

        let account = create_test_account().await;
        let account_ref = Arc::downgrade(&account);

        let mut balances = AccountBalances { balance_map, account_ref };

        let instrument = Instrument { base: base_token.clone(),
                                      quote: quote_token.clone(),
                                      kind: InstrumentKind::Spot };

        let market_event = MarketEvent { exchange_time: 0,
                                         instrument: instrument.clone(),
                                         kind: ClickhouseTrade { basequote: "BTC/USDT".to_string(),
                                                                 side: "Buy".to_string(),
                                                                 price: 50000.0,
                                                                 timestamp: 0,
                                                                 amount: 0.1 },
                                         exchange: ExchangeVariant::Simulated,
                                         received_time: 0 };

        let account_event = balances.update_from_trade(&market_event).await;

        println!("Base Token Balance: {:?}", balances.balance(&base_token).unwrap());
        println!("Quote Token Balance: {:?}", balances.balance(&quote_token).unwrap());
        println!("Account Event: {:?}", account_event);

        let expected_base_balance = Balance::new(1.1, 1.1); // 1 BTC + 0.1 BTC
        let expected_quote_balance = Balance::new(45000.0, 50000.0); // 50,000 USDT - (0.1 * 50,000)

        assert_eq!(balances.balance(&base_token).unwrap().total, expected_base_balance.total);
        assert_eq!(balances.balance(&base_token).unwrap().available, expected_base_balance.available);
        assert_eq!(balances.balance(&quote_token).unwrap().total, expected_quote_balance.total);
        assert_eq!(balances.balance(&quote_token).unwrap().available, expected_quote_balance.available);

        if let AccountEventKind::Balances(balances) = account_event.kind {
            let base_balance_event = balances.iter().find(|tb| tb.token == base_token).unwrap();
            let quote_balance_event = balances.iter().find(|tb| tb.token == quote_token).unwrap();

            assert_eq!(base_balance_event.balance, expected_base_balance);
            assert_eq!(quote_balance_event.balance.total, expected_quote_balance.total);
            assert_eq!(quote_balance_event.balance.available, expected_quote_balance.available);
        }
        else {
            panic!("Unexpected account event kind");
        }
    }
}
