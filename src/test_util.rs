// src/test_util.rs

use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc, Weak},
};

use tokio::sync::{Mutex, RwLock};
use crate::{
    common::{
        order::identification::machine_id::generate_machine_id,
        position::{AccountPositions, PositionDirectionMode, PositionMarginMode},
    },
    sandbox::account::{
        account_config::{AccountConfig, CommissionLevel, MarginMode},
        account_latency::{AccountLatency, FluctuationMode},
        account_orders::AccountOrders,
        account_states::AccountState,
    },
};
use crate::sandbox::account::Account;

pub async fn create_test_account() -> Account {
    let leverage_rate = 1.0;

    // 创建账户配置
    let account_config = AccountConfig {
        margin_mode: MarginMode::SingleCurrencyMargin,
        position_mode: PositionDirectionMode::NetMode,
        position_margin_mode: PositionMarginMode::Isolated,
        commission_level: CommissionLevel::Lv1,
        funding_rate: 0.0,
        account_leverage_rate: leverage_rate,
        fees_book: HashMap::new(),
    };

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
        exchange_timestamp: AtomicI64::new(0),
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
