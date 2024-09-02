use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicI64, Weak},
    time::Duration,
};

use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use unilink_execution::{
    common::{
        balance::Balance
        ,
        instrument::{Instrument, kind::InstrumentKind},
        order::{
            identification::{client_order_id::ClientOrderId, OrderId},
            Order,
            order_instructions::OrderInstruction,
            OrderRole, states::{open::Open, request_open::RequestOpen},
        },
        position::AccountPositions,
        Side,
        token::Token,
    },
    Exchange,
    sandbox::account::{
        Account,
        account_latency::{AccountLatency, FluctuationMode},
        account_orders::AccountOrders,
        account_states::AccountState,
    },
};
use unilink_execution::common::order::states::cancelled::Cancelled;
use unilink_execution::common::order::states::request_cancel::RequestCancel;
use unilink_execution::sandbox::sandbox_client::SandBoxClientEvent;
use unilink_execution::sandbox::SandBoxExchange;
use unilink_execution::test_utils::{create_test_account, create_test_account_config, create_test_account_state};
pub async fn run_default_exchange(
    event_simulated_rx: mpsc::UnboundedReceiver<SandBoxClientEvent>,
) {
    // 创建 AccountState 实例
    let account_state = create_test_account_state().await;

    // 创建 Account 实例，并将其包裹在 Arc<Mutex<Account>> 中
    let account_arc = Arc::new(Mutex::new(create_test_account().await));

    // 打印引用计数
    println!(
        "Before updating account_ref: Strong count: {}, Weak count: {}",
        Arc::strong_count(&account_arc),
        Arc::weak_count(&account_arc),
    );

    // 手动更新 account_state 的 account_ref
    {
        let mut account_state_locked = account_state.lock().await;
        // 使用 Arc<Mutex<Account>> 直接调用 Arc::downgrade
        account_state_locked.account_ref = Arc::downgrade(&account_arc);
        println!("Account reference successfully set.");
    }

    // 再次打印引用计数
    println!(
        "After updating account_ref: Strong count: {}, Weak count: {}",
        Arc::strong_count(&account_arc),
        Arc::weak_count(&account_arc),
    );

    // 测试 account_ref 是否能够成功升级
    {
        let state = account_state.lock().await;
        match state.account_ref.upgrade() {
            Some(upgraded_account) => {
                println!("Successfully upgraded account_ref!");
                // 验证升级后的 account_ref 是否指向正确的 Account 实例
                assert_eq!(upgraded_account.lock().await.machine_id, account_arc.lock().await.machine_id);
                println!("machine_id is correct and matches the original account.");
            }
            None => {
                println!("Failed to upgrade account_ref, it is None.");
                panic!("account_ref upgrade failed!");
            }
        }
    }

    // 创建并初始化 SandBoxExchange
    let sandbox_exchange = SandBoxExchange::initiator()
        .event_sandbox_rx(event_simulated_rx)
        .account(account_arc.clone()) // 传递 Arc<Mutex<Account>> 实例
        .initiate() // 使用 initiate 初始化 SandBoxExchange
        .expect("failed to build SandBoxExchange");

    println!("Sandbox exchange built successfully");

    // 再次打印引用计数，确保在 SandBoxExchange 中 `account_ref` 没有丢失
    println!(
        "After SandBoxExchange creation: Strong count: {}, Weak count: {}",
        Arc::strong_count(&account_arc),
        Arc::weak_count(&account_arc),
    );

    // 运行交易所（本地或在线）
    sandbox_exchange.run_local().await;
    println!("Sandbox exchange is running");

    // 这里添加更多调试信息，特别是跟踪引用计数和account_ref状态
    {
        let state = account_state.lock().await;
        match state.account_ref.upgrade() {
            Some(upgraded_account) => {
                println!("Successfully upgraded account_ref after running the exchange!");
            }
            None => {
                println!("Failed to upgrade account_ref after running the exchange, it is None.");
            }
        }
    }
}

/// 设置延迟为50ms
#[allow(dead_code)]
pub fn latency_50ms() -> Duration {
    Duration::from_millis(50)
}

/// 设置交易手续费为0.5%
#[allow(dead_code)]
pub fn fees_50_percent() -> f64 {
    0.5
}


/// 定义沙箱交易所支持的Instrument
#[allow(dead_code)]
pub async fn initial_balances() -> Arc<Mutex<AccountState>> {
    // 初始化账户余额
    let mut balances = HashMap::new();
    balances.insert(Token::from("TEST_BASE"), Balance::new(10.0, 10.0, 1.0));
    balances.insert(Token::from("TEST_QUOTE"), Balance::new(10_000.0, 10_000.0, 1.0));

    let positions = AccountPositions {
        margin_pos: Vec::new(),
        perpetual_pos: Vec::new(),
        futures_pos: Vec::new(),
        option_pos: Vec::new(),
    };

    let account_state = AccountState {
        balances,
        positions,
        account_ref: Weak::new(),
    };

    // 包装 AccountState 实例在 Arc<Mutex<...>> 中
    let account_state_arc = Arc::new(Mutex::new(account_state));

    // 创建 Account 实例，并将其包装在 Arc<Mutex<...>> 中
    let account = Arc::new(Mutex::new(Account {
        current_session: Uuid::new_v4(),
        machine_id: 0,
        exchange_timestamp: AtomicI64::new(1234567),
        account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
        config: Arc::new(create_test_account_config()),
        states: account_state_arc.clone(),
        orders: Arc::new(tokio::sync::RwLock::new(AccountOrders::new(0, vec![], AccountLatency {
            fluctuation_mode: FluctuationMode::Sine,
            maximum: 0,
            minimum: 0,
            current_value: 0,
        }).await)),
    }));

    // 将 `Arc<Mutex<Account>>` 的弱引用赋给 account_ref
    {
        let mut account_state_locked = account_state_arc.lock().await;
        account_state_locked.account_ref = Arc::downgrade(&account);
    }

    account_state_arc
}

/// 创建限价订单请求
pub fn order_request_limit<I>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    price: f64,
    quantity: f64,
) -> Order<RequestOpen>
where
    I: Into<Instrument>,
{
    Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: RequestOpen {
            reduce_only: false, // 假设创建的订单不是 reduce_only
            price,
            size: quantity,
        },
    }
}

/// 创建开放订单
pub fn open_order<I>(
    instrument: I,
    cid: ClientOrderId,
    id: OrderId,
    side: Side,
    price: f64,
    quantity: f64,
    filled: f64,
) -> Order<Open>
where
    I: Into<Instrument>,
{
    Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: Open {
            id,
            price,
            size: quantity,
            filled_quantity: filled,
            order_role: OrderRole::Taker,
        },
    }
}

/// 创建订单取消请求
#[allow(dead_code)]
pub fn order_cancel_request<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<RequestCancel>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        kind: OrderInstruction::Cancel,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: RequestCancel::from(id),
    }
}

/// 创建取消的订单
#[allow(dead_code)]
pub fn order_cancelled<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<Cancelled>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        kind: OrderInstruction::Cancel,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: Cancelled::from(id),
    }
}