// src/test_utils

use crate::{
    common::{
        account_positions::{
            future::{FuturePosition, FuturePositionConfig},
            perpetual::{PerpetualPosition, PerpetualPositionConfig},
            position_id::PositionId,
            position_meta::PositionMeta,
            AccountPositions, PositionDirectionMode, PositionMarginMode,
        },
        balance::{Balance, TokenBalance}
        ,
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, machine_id::generate_machine_id, OrderId},
            order_instructions::OrderInstruction,
            states::{open::Open, request_open::RequestOpen},
            Order, OrderRole,
        },
        token::Token,
        Side,
    },
    sandbox::account::{
        account_config::{AccountConfig, CommissionLevel, CommissionRates, MarginMode, SandboxMode},
        account_latency::{AccountLatency, FluctuationMode},
        account_orders::AccountOrders,
        Account,
    },
    Exchange,
};
use dashmap::DashMap;
use rand::Rng;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 创建一个测试用的 `Instrument` 实例。
pub fn create_test_instrument(kind: InstrumentKind) -> Instrument
{
    Instrument { base: Token::from("BTC"),
                 quote: Token::from("USDT"),
                 kind }
}

/// 创建一个测试用的 `AccountConfig` 实例。
pub fn create_test_account_config() -> AccountConfig
{
    let leverage_rate = 1.0;

    AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
                    position_direction_mode: PositionDirectionMode::NetMode,
                    position_margin_mode: PositionMarginMode::Isolated,
                    commission_level: CommissionLevel::Lv1,
                    funding_rate: 0.0,
                    account_leverage_rate: leverage_rate,
                    fees_book: HashMap::new(),
                    execution_mode: SandboxMode::Backtest }
}
// 帮助函数，用于创建测试用的 AccountOrders 实例
pub async fn create_test_account_orders() -> AccountOrders
{
    let instruments = vec![Instrument::new("BTC", "USD", InstrumentKind::Spot)];
    let account_latency = AccountLatency::new(FluctuationMode::Sine, 100, 10);
    AccountOrders::new(123124, instruments, account_latency).await
}

/// 创建一个测试用的 `Order<Open>` 实例。
pub fn create_test_order_open(side: Side, price: f64, size: f64) -> Order<Open>
{
    Order { instruction: OrderInstruction::Limit, // 假设测试订单使用限价订单类型
            exchange: Exchange::SandBox,          // 假设测试环境使用 SandBox 交易所
            instrument: Instrument { base: Token::from("ETH"),        // 测试用基础货币
                                     quote: Token::from("USDT"),      // 测试用报价货币
                                     kind: InstrumentKind::Perpetual  /* 测试用永续合约 */ },
            timestamp: 1625247600000,                       // 假设的客户端时间戳
            cid: Some(ClientOrderId("validCID123".into())), // 假设的客户端订单ID
            side,
            state: Open { id: OrderId(123), // 假设的订单ID
                          price,
                          size,
                          filled_quantity: 0.0,         // 初始填充数量为0
                          order_role: OrderRole::Taker  /* 假设订单角色为 Taker */ } }
}

// 帮助函数，用于创建测试用的订单
pub fn create_test_request_open(base: &str, quote: &str) -> Order<RequestOpen>
{
    let machine_id = generate_machine_id().unwrap();
    let mut rng = rand::thread_rng();
    let counter = rng.gen_range(0..10);
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).expect("时间出现倒退").as_millis() as u64;

    let order_id = OrderId::new(now_ts, machine_id, counter);
    Order { instruction: OrderInstruction::Market,
            exchange: Exchange::SandBox,
            instrument: Instrument { base: Token::from(base),
                                     quote: Token::from(quote),
                                     kind: InstrumentKind::Spot },
            timestamp: 1625247600000,
            cid: Some(ClientOrderId(format!("CID{}", order_id.0 % 1_000_000))),
            side: Side::Buy,
            state: RequestOpen { price: 50000.0,
                                 size: 1.0,
                                 reduce_only: false } }
}

pub async fn create_test_account() -> Account
{
    let leverage_rate = 1.0;
    let balances = DashMap::new();
    balances.insert(Token::from("ETH"), Balance::new(10.0, 10.0, 1.0));
    balances.insert(Token::from("USDT"), Balance::new(10_000.0, 10_000.0, 1.0));

    let commission_rates = CommissionRates { maker_fees: 0.001,
                                             taker_fees: 0.002 };

    let mut account_config = AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
                                             position_direction_mode: PositionDirectionMode::NetMode,
                                             position_margin_mode: PositionMarginMode::Isolated,
                                             commission_level: CommissionLevel::Lv1,
                                             funding_rate: 0.0,
                                             account_leverage_rate: leverage_rate,
                                             fees_book: HashMap::new(),
                                             execution_mode: SandboxMode::Backtest };

    account_config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

    let positions = AccountPositions { margin_pos_long: Arc::new(RwLock::new(HashMap::new())),
                                       margin_pos_short: Arc::new(RwLock::new(HashMap::new())),
                                       perpetual_pos_long:Arc::new(RwLock::new(HashMap::new())),
                                       perpetual_pos_short:Arc::new(RwLock::new(HashMap::new())),
                                       futures_pos_long:Arc::new(RwLock::new(HashMap::new())),
                                       futures_pos_short:Arc::new(RwLock::new(HashMap::new())),
                                       option_pos_long_call:Arc::new(RwLock::new(HashMap::new())),
                                       option_pos_long_put:Arc::new(RwLock::new(HashMap::new())),
                                       option_pos_short_call:Arc::new(RwLock::new(HashMap::new())),
                                       option_pos_short_put: Arc::new(RwLock::new(HashMap::new()))};

    let machine_id = generate_machine_id().unwrap();

    // 创建 Account 实例，并将其包裹在 Arc<Account> 中
    Account { current_session: Uuid::new_v4(),
              machine_id,
              exchange_timestamp: AtomicI64::new(1234567),
              account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
              config: account_config,
              balances,
              positions,
              orders: Arc::new(RwLock::new(AccountOrders::new(machine_id,
                                                              vec![Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual))],
                                                              AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                               maximum: 300,
                                                                               minimum: 0,
                                                                               current_value: 0 }).await)) }
}

/// 创建一个测试用的 `PerpetualPosition` 实例。
pub fn create_test_perpetual_position(instrument: Instrument) -> PerpetualPosition
{
    PerpetualPosition { meta: PositionMeta { position_id: PositionId(12341241241),
                                             enter_ts: 0,
                                             update_ts: 0,
                                             exit_balance: TokenBalance { token: instrument.base.clone(),
                                                                          balance: Balance::new(0.0, 0.0, 1.0) },
                                             exchange: Exchange::SandBox,
                                             instrument,
                                             side: Side::Buy,
                                             current_size: 1.0,
                                             current_fees_total: 0.2,
                                             current_avg_price_gross: 0.0,
                                             current_symbol_price: 0.0,
                                             current_avg_price: 0.0,
                                             unrealised_pnl: 0.0,
                                             realised_pnl: 0.0 },
                        pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                              leverage: 1.0,
                                                              position_mode: PositionDirectionMode::LongShortMode },
                        liquidation_price: 0.0,
                        margin: 0.0 }
}

/// 创建一个测试用的 `FuturePosition` 实例，指定 `Side`。
pub fn create_test_future_position_with_side(instrument: Instrument, side: Side) -> FuturePosition
{
    FuturePosition { meta: PositionMeta { position_id: PositionId(1234124512412),
                                          enter_ts: 0,
                                          update_ts: 0,
                                          exit_balance: TokenBalance { token: instrument.base.clone(),
                                                                       balance: Balance::new(0.0, 0.0, 1.0) },
                                          exchange: Exchange::SandBox,
                                          instrument,
                                          side,
                                          current_size: 0.0,
                                          current_fees_total:0.2,
                                          current_avg_price_gross: 0.0,
                                          current_symbol_price: 0.0,
                                          current_avg_price: 0.0,
                                          unrealised_pnl: 0.0,
                                          realised_pnl: 0.0 },
                     pos_config: FuturePositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                        leverage: 1.0,
                                                        position_mode: PositionDirectionMode::LongShortMode },
                     liquidation_price: 0.0,
                     margin: 0.0,
                     funding_fee: 0.0 }
}
