// src/test_util.rs

use crate::sandbox::account::account_config::CommissionRates;
use crate::common::friction::FutureFees;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc, Weak},
};

use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use crate::{common::{
    order::identification::machine_id::generate_machine_id,
    position::{AccountPositions, PositionDirectionMode, PositionMarginMode},
}, sandbox::account::{
    account_config::{AccountConfig, CommissionLevel, MarginMode},
    account_latency::{AccountLatency, FluctuationMode},
    account_orders::AccountOrders,
    account_states::AccountState,
}, Exchange};
use crate::common::balance::{Balance, TokenBalance};
use crate::common::friction::{Fees, PerpetualFees};
use crate::common::instrument::Instrument;
use crate::common::instrument::kind::InstrumentKind;
use crate::common::position::future::{FuturePosition, FuturePositionConfig};
use crate::common::position::perpetual::{PerpetualPosition, PerpetualPositionConfig};
use crate::common::position::position_meta::PositionMeta;
use crate::common::Side;
use crate::common::token::Token;
use crate::sandbox::account::Account;

/// 创建一个测试用的 `Instrument` 实例。
pub fn create_test_instrument(kind: InstrumentKind) -> Instrument {
    Instrument {
        base: Token::from("BTC"),
        quote: Token::from("USDT"),
        kind,
    }
}

/// 创建一个测试用的 `AccountConfig` 实例。
pub fn create_test_account_config() -> AccountConfig {
    let leverage_rate = 1.0;

    AccountConfig {
        margin_mode: MarginMode::SingleCurrencyMargin,
        position_mode: PositionDirectionMode::NetMode,
        position_margin_mode: PositionMarginMode::Isolated,
        commission_level: CommissionLevel::Lv1,
        funding_rate: 0.0,
        account_leverage_rate: leverage_rate,
        fees_book: HashMap::new(),
    }
}

/// 创建一个测试用的 `AccountState` 实例，并将其封装在 `Arc<Mutex<...>>` 中。
pub async fn create_test_account_state() -> Arc<Mutex<AccountState>> {
    let balances = HashMap::new();
    let positions = AccountPositions { margin_pos: Vec::new(),
        perpetual_pos: Vec::new(),
        futures_pos: Vec::new(),
        option_pos: Vec::new() };

    let account_state = AccountState {
        balances: balances.clone(),
        positions: positions.clone(),
        account_ref: Weak::new(),
    };

    let account_state_arc = Arc::new(Mutex::new(account_state));

    let account = Arc::new(Account {
        machine_id: 0,
        exchange_timestamp: AtomicI64::new(1234567),
        account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
        config: Arc::new(create_test_account_config()),
        states: account_state_arc.clone(),
        orders: Arc::new(tokio::sync::RwLock::new(
            AccountOrders::new(
                0,
                vec![],
                AccountLatency {
                    fluctuation_mode: FluctuationMode::Sine,
                    maximum: 0,
                    minimum: 0,
                    current_value: 0,
                },
            ).await,
        )),
    });

    // 更新 `account_ref` 以指向 `Account`
    {
        let mut account_state_locked = account_state_arc.lock().await;
        account_state_locked.account_ref = Arc::downgrade(&account);
    }

    account_state_arc
}
pub async fn create_test_account() -> Account {
    let leverage_rate = 1.0;

    // 创建账户配置
    let mut account_config = AccountConfig {
        margin_mode: MarginMode::SingleCurrencyMargin,
        position_mode: PositionDirectionMode::NetMode,
        position_margin_mode: PositionMarginMode::Isolated,
        commission_level: CommissionLevel::Lv1,
        funding_rate: 0.0,
        account_leverage_rate: leverage_rate,
        fees_book: HashMap::new(),
    };

    // 设置 CommissionRates 并插入到 fees_book 中
    let commission_rates = CommissionRates {
        maker_fees: 0.001,
        taker_fees: 0.002,
    };
    account_config.fees_book.insert(InstrumentKind::Perpetual, commission_rates);

    // 创建账户状态
    let positions = AccountPositions {
        margin_pos: Vec::new(),
        perpetual_pos: Vec::new(),
        futures_pos: Vec::new(),
        option_pos: Vec::new(),
    };

    let account_state = AccountState {
        balances: HashMap::new(),
        positions,
        account_ref: Weak::new(),
    };

    // 包装为 Arc<Mutex<...>>
    let account_state_arc = Arc::new(Mutex::new(account_state.clone()));

    let machine_id = generate_machine_id().unwrap();
    // 创建 Account 实例
    let account = Account {
        machine_id,
        exchange_timestamp: AtomicI64::new(1234567),
        account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
        config: Arc::new(account_config),
        states: account_state_arc.clone(),
        orders: Arc::new(RwLock::new(AccountOrders::new(
            machine_id,
            vec![],
            AccountLatency {
                fluctuation_mode: FluctuationMode::Sine,
                maximum: 300,
                minimum: 0,
                current_value: 0,
            },
        )
            .await)),
    };

    // 更新 account_ref，使其指向 Account
    {
        let mut account_state_locked = account_state_arc.lock().await;
        account_state_locked.account_ref = Arc::downgrade(&Arc::new(account.clone()));
    }

    account
}


/// 创建一个测试用的 `PerpetualPosition` 实例。
pub fn create_test_perpetual_position(instrument: Instrument) -> PerpetualPosition {
    PerpetualPosition {
        meta: PositionMeta {
            position_id: Uuid::new_v4().to_string(),
            enter_ts: 0,
            update_ts: 0,
            exit_balance: TokenBalance {
                token: instrument.base.clone(),
                balance: Balance::new(0.0, 0.0, 1.0),
            },
            exchange: Exchange::SandBox,
            instrument,
            side: Side::Buy,
            current_size: 1.0,
            current_fees_total: Fees::Perpetual(PerpetualFees {
                maker_fee: 0.0,
                taker_fee: 0.0,
                funding_fee: 0.0,
            }),
            current_avg_price_gross: 0.0,
            current_symbol_price: 0.0,
            current_avg_price: 0.0,
            unrealised_pnl: 0.0,
            realised_pnl: 0.0,
        },
        pos_config: PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0,
            position_mode: PositionDirectionMode::LongShortMode,
        },
        liquidation_price: 0.0,
        margin: 0.0,
    }
}

/// 创建一个测试用的 `FuturePosition` 实例，指定 `Side`。
pub fn create_test_future_position_with_side(instrument: Instrument, side: Side) -> FuturePosition {
    FuturePosition {
        meta: PositionMeta {
            position_id: Uuid::new_v4().to_string(),
            enter_ts: 0,
            update_ts: 0,
            exit_balance: TokenBalance {
                token: instrument.base.clone(),
                balance: Balance::new(0.0, 0.0, 1.0),
            },
            exchange: Exchange::SandBox,
            instrument,
            side,
            current_size: 0.0,
            current_fees_total: Fees::Future(FutureFees {
                maker_fee: 0.0,
                taker_fee: 0.0,
                funding_fee: 0.0,
            }),
            current_avg_price_gross: 0.0,
            current_symbol_price: 0.0,
            current_avg_price: 0.0,
            unrealised_pnl: 0.0,
            realised_pnl: 0.0,
        },
        pos_config: FuturePositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0,
            position_mode: PositionDirectionMode::LongShortMode,
        },
        liquidation_price: 0.0,
        margin: 0.0,
        funding_fee: 0.0,
    }
}