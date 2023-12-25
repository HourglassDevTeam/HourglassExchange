use cerebro_data::subscription::trade::PublicTrade;
use cerebro_integration::model::{
    instrument::{Instrument, kind::InstrumentKind, symbol::Symbol},
    Side,
};
use tokio::sync::mpsc;
use uuid::Uuid;

use cerebro_broker::{
    error::ExecutionError,
    ExecutionClient,
    model::{
        AccountEvent,
        AccountEventKind,
        balance::{Balance, SymbolBalance},
        ClientOrderId, order::OrderId, trade::{SymbolFees, Trade, TradeId},
    },
    simulated::{execution::SimulatedExecution, SimulatedEvent},
};

use crate::util::{
    fees_50_percent, initial_balances, latency_50ms, open_order, order_cancel_request,
    order_cancelled, order_request_limit, run_default_exchange,
};

mod util;

#[derive(Clone)]
struct Ids {
    cid: ClientOrderId,
    id: OrderId,
}

impl Ids {
    fn new<Id: Into<OrderId>>(cid: Uuid, id: Id) -> Self {
        Self {
            cid: ClientOrderId(cid),
            id: id.into(),
        }
    }
}

#[tokio::test]
async fn main() {
    // 创建通道:
    //  - event_account_tx 发送AccountEvents到Cerebro Engine
    //  - event_simulated_tx 发送MarketEvents和执行请求到SimulatedExchange
    // Create channels:
    //  - event_account_tx sends AccountEvents to the Cerebro Engine
    //  - event_simulated_tx sends MarketEvents and execution requests to the SimulatedExchange
    let (event_account_tx, mut event_account_rx) = mpsc::unbounded_channel();
    let (mut event_simulated_tx, event_simulated_rx) = mpsc::unbounded_channel();

    // 构建SimulatedExchange并在它自己的Tokio任务上运行
    // Build SimulatedExchange & run on it's own Tokio task
    tokio::spawn(run_default_exchange(event_account_tx, event_simulated_rx));

    // 初始化SimulatedExecution执行，通过模拟通道与交易所交互
    // Initialise SimulatedExecution execution to interact with the exchange via the simulated channel
    let client = SimulatedExecution {
        request_tx: event_simulated_tx.clone(),
    };

    // 1. 获取初始OpenOrders时我们没有打开的订单
    // 1. Fetch initial OpenOrders when we have no open Orders
    test_1_fetch_initial_orders_and_check_empty(&client).await;

    // 2. 获取初始Balances时没有发生余额变化的事件
    // 2. Fetch initial Balances when there have been no balance changing events
    test_2_fetch_balances_and_check_same_as_initial(&client).await;

    // 3. 打开LIMIT Buy Order并检查报价货币(usdt)的AccountEvent Balance是否已发送
    // 3. Open LIMIT Buy Order and check AccountEvent Balance is sent for the quote currency (usdt)
    let test_3_ids = Ids::new(Uuid::new_v4(), 1);
    test_3_open_limit_buy_order(&client, test_3_ids.clone(), &mut event_account_rx).await;

    // 4. 发送不匹配任何打开订单的MarketEvent并检查是否没有发送AccountEvents
    // 4. Send MarketEvent that does not match any open Order and check no AccountEvents are sent
    test_4_send_market_event_that_does_not_match_any_open_order(
        &mut event_simulated_tx,
        &mut event_account_rx,
    );

    // 5. 取消打开的买单并检查是否已发送取消订单和余额的AccountEvents
    // 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent
    test_5_cancel_buy_order(&client, test_3_ids, &mut event_account_rx).await;

    // 6. 打开2x LIMIT Buy Orders并对接收到的AccountEvents进行断言
    // 6. Open 2x LIMIT Buy Orders & assert on received AccountEvents
    let test_6_ids_1 = Ids::new(Uuid::new_v4(), 2);
    let test_6_ids_2 = Ids::new(Uuid::new_v4(), 3);
    test_6_open_2x_limit_buy_orders(
        &client,
        test_6_ids_1.clone(),
        test_6_ids_2,
        &mut event_account_rx,
    )
    .await;

    // 7. 发送完全匹配1x打开订单(交易)的MarketEvent并检查余额和交易的AccountEvents
    // 7. Send MarketEvent that exactly full matches 1x open Order (trade) and check AccountEvents
    //    for balances and trades
    test_7_send_market_event_that_exact_full_matches_order(
        &mut event_simulated_tx,
        &mut event_account_rx,
    )
    .await;

    // 8. 获取打开的订单并检查test_6_order_cid_1是否只剩一个限价买单
    // 8. Fetch open orders & check only one limit buy order remaining from test_6_order_cid_1
    test_8_fetch_open_orders_and_check_test_6_order_cid_1_only(&client, test_6_ids_1.clone()).await;

    // 9. 打开2x LIMIT Sell Order并对接收到的AccountEvents进行断言
    // 9. Open 2x LIMIT Sell Order & assert on received AccountEvents
    let test_9_ids_1 = Ids::new(Uuid::new_v4(), 4);
    let test_9_ids_2 = Ids::new(Uuid::new_v4(), 5);
    test_9_open_2x_limit_sell_orders(
        &client,
        test_9_ids_1,
        test_9_ids_2.clone(),
        &mut event_account_rx,
    )
    .await;

    // 10. 发送完全匹配1x卖单(交易)的MarketEvent，并部分匹配另一卖单(交易)。检查两个匹配的余额和交易的AccountEvents是否已发送。
    // 10. Send MarketEvent that fully matches 1x sell Order (trade), and partially matches the other
    //     sell Order (trade). Check AccountEvents for balances and trades of both matches are sent.
    test_10_send_market_event_that_full_and_partial_matches_orders(
        &mut event_simulated_tx,
        &mut event_account_rx,
    )
    .await;

    // 11. 取消所有打开的订单。包括部分填充的卖单和未填充的买单。检查已发送订单取消和余额的AccountEvents。
    // 11. Cancel all open orders. Includes a partially filled sell order, and non-filled buy order.
    //     Check AccountEvents for orders cancelled and balances are sent.
    test_11_cancel_all_orders(&client, test_6_ids_1, test_9_ids_2, &mut event_account_rx).await;

    // 12. 获取打开的订单（现在我们已经调用了cancel_all）并检查它是否为空
    // 12. Fetch open orders (now that we've called cancel_all) and check it is empty
    test_12_fetch_open_orders_and_check_empty(&client).await;

    // 13. 由于资金不足，未能打开限价买单
    // 13. Fail to open limit buy order with insufficient funds
    let test_13_ids_1 = Ids::new(Uuid::new_v4(), 6);
    let test_13_ids_2 = Ids::new(Uuid::new_v4(), 6); // 6因为第一个应该失败
    test_13_fail_to_open_one_of_two_limits_with_insufficient_funds(
        &client,
        test_13_ids_1,
        test_13_ids_2,
        &mut event_account_rx,
    )
    .await;

    // 14. 使用错误的OrderId尝试取消不存在的限价订单而失败
    // 14. Fail to cancel limit order with OrderNotFound using incorrect OrderId
    test_14_fail_to_cancel_limit_with_order_not_found(&client).await;
}

// 1. 当我们没有打开的订单时，获取初始OpenOrders。
// 1. Fetch initial OpenOrders when we have no open Orders.
async fn test_1_fetch_initial_orders_and_check_empty(client: &SimulatedExecution) {
    // 获取当前打开的订单
    let initial_orders = client.fetch_orders_open().await.unwrap();
    // 断言初始订单列表为空
    assert!(initial_orders.is_empty());
}

// 2. 当没有发生余额变化的事件时，获取初始Balances。
// 2. Fetch initial Balances when there have been no balance changing events.
async fn test_2_fetch_balances_and_check_same_as_initial(client: &SimulatedExecution) {
    // 获取实际的余额
    let actual_balances = client.fetch_balances().await.unwrap();
    // 获取初始余额
    let initial_balances = initial_balances();

    // 断言实际余额的数量与初始余额的数量相等
    assert_eq!(actual_balances.len(), initial_balances.len());

    // 遍历实际余额
    for actual in actual_balances {
        // 获取预期的余额
        let expected = initial_balances.get(&actual.symbol).unwrap();
        // 断言实际余额等于预期余额
        assert_eq!(actual.balance, *expected);
    }
}


// 3. 打开LIMIT Buy Order并检查报价货币(usdt)的AccountEvent Balance是否已发送。
// 3. Open LIMIT Buy Order and check AccountEvent Balance is sent for the quote currency (usdt).
async fn test_3_open_limit_buy_order(
    client: &SimulatedExecution,
    test_3_ids: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 打开新的订单
    let new_orders = client
        .open_orders(vec![order_request_limit(
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            test_3_ids.cid,
            Side::Buy,
            100.0,
            1.0,
        )])
        .await;

    // 预期的新订单
    let expected_new_order = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_3_ids.cid,
        test_3_ids.id,
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );

    // 确认新订单被成功创建
    assert_eq!(new_orders.len(), 1);
    assert_eq!(new_orders[0].clone().unwrap(), expected_new_order);

    // 检查AccountEvent Balance，确认报价货币(usdt)的可用余额已减少
    // Check AccountEvent Balance for quote currency (usdt) has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(usdt_balance),
               ..
           }) => {
            // 预期usdt Balance.available = 10_000 - (100.0 * 1.0)
            // Expected usdt Balance.available = 10_000 - (100.0 * 1.0)
            let expected = SymbolBalance::new("usdt", Balance::new(10_000.0, 9_900.0));
            assert_eq!(usdt_balance, expected);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查是否生成了AccountEvent OrderNew
    // Check AccountEvent OrderNew generated
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_new_order);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }
}

// 4. 发送一个不匹配任何打开订单的MarketEvent，并检查是否没有发送AccountEvents。
// 4. Send MarketEvent that does not match any open Order and check no AccountEvents are sent.
fn test_4_send_market_event_that_does_not_match_any_open_order(
    event_simulated_tx: &mut mpsc::UnboundedSender<SimulatedEvent>,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 发送不匹配任何订单的市场交易事件
    event_simulated_tx
        .send(SimulatedEvent::MarketTrade((
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            PublicTrade {
                id: "test_4".to_string(),
                side: Side::Sell,
                price: 1000.0,
                amount: 1.0,
            },
        )))
        .unwrap();

    // 检查是否没有生成更多的AccountEvents
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }
}

// 5. 取消打开的买单并检查已发送取消订单和余额的AccountEvents。
// 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent.
async fn test_5_cancel_buy_order(
    client: &SimulatedExecution,
    test_3_ids: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 取消订单
    let cancelled = client
        .cancel_orders(vec![order_cancel_request(
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            test_3_ids.cid,
            Side::Buy,
            test_3_ids.id.clone(),
        )])
        .await;

    // 预期被取消的订单
    let expected_cancelled = order_cancelled(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_3_ids.cid,
        Side::Buy,
        test_3_ids.id,
    );

    // 确认订单已被取消
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].clone().unwrap(), expected_cancelled);

    // 检查AccountEvent Order是否被取消
    // Check AccountEvent Order cancelled
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersCancelled(cancelled),
               ..
           }) => {
            assert_eq!(cancelled.len(), 1);
            assert_eq!(cancelled[0].clone(), expected_cancelled);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查报价货币(usdt)的AccountEvent Balance是否显示可用余额增加
    // Check AccountEvent Balance for quote currency (usdt) has available balance increase
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(usdt_balance),
               ..
           }) => {
            // 预期usdt Balance.available = 9_900 + (100.0 * 1.0)
            // Expected usdt Balance.available = 9_900 + (100.0 * 1.0)
            let expected = SymbolBalance::new("usdt", Balance::new(10_000.0, 10_000.0));
            assert_eq!(usdt_balance, expected);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }
}

// 6. 打开2x限价买单并检查是否发送了余额和订单新建的AccountEvents。
// 6. Open 2x limit buy orders and check AccountEvents for balance & order new are sent.
async fn test_6_open_2x_limit_buy_orders(
    client: &SimulatedExecution,
    test_6_ids_1: Ids,
    test_6_ids_2: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 打开两个限价买单
    let opened_orders = client
        .open_orders(vec![
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_6_ids_1.cid,
                Side::Buy,
                100.0,
                1.0,
            ),
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_6_ids_2.cid,
                Side::Buy,
                200.0,
                1.0,
            ),
        ])
        .await;

    // 预期的第一个新订单
    let expected_order_new_1 = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_6_ids_1.cid,
        test_6_ids_1.id.clone(),
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );

    // 预期的第二个新订单
    let expected_order_new_2 = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_6_ids_2.cid,
        test_6_ids_2.id,
        Side::Buy,
        200.0,
        1.0,
        0.0,
    );

    // 确认订单已被成功创建
    assert_eq!(opened_orders.len(), 2);
    assert_eq!(opened_orders[0].clone().unwrap(), expected_order_new_1);
    assert_eq!(opened_orders[1].clone().unwrap(), expected_order_new_2);

    // 检查第一个订单的AccountEvent Balance - 报价货币的可用余额应该减少
    // Check AccountEvent Balance for first order - quote currency has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(usdt_balance),
               ..
           }) => {
            // 预期usdt Balance.available = 10_000 - (100.0 * 1.0)
            // Expected usdt Balance.available = 10_000 - (100.0 * 1.0)
            let expected = SymbolBalance::new("usdt", Balance::new(10_000.0, 9_900.0));
            assert_eq!(usdt_balance, expected);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查第一个订单的AccountEvent OrdersNew
    // Check AccountEvent OrdersNew for first order
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_order_new_1);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查第二个订单的AccountEvent Balance - 报价货币的可用余额应该减少
    // Check AccountEvent Balance for second order - quote currency has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(usdt_balance),
               ..
           }) => {
            // 预期usdt Balance.available = 9_900 - (200.0 * 1.0)
            // Expected usdt Balance.available = 9_900 - (200.0 * 1.0)
            let expected = SymbolBalance::new("usdt", Balance::new(10_000.0, 9_700.0));
            assert_eq!(usdt_balance, expected);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查第二个订单的AccountEvent OrdersNew
    // Check AccountEvent OrdersNew for second order
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_order_new_2);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }
}

// 7. 发送完全匹配1x打开订单（交易）的MarketEvent，并检查是否发送了余额和交易的AccountEvents。
// 7. Send MarketEvent that exactly full matches 1x open Order (trade) and check AccountEvents for
// balances and trades are sent.
async fn test_7_send_market_event_that_exact_full_matches_order(
    event_simulated_tx: &mut mpsc::UnboundedSender<SimulatedEvent>,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 发送匹配的MarketEvent
    // Send matching MarketEvent
    event_simulated_tx
        .send(SimulatedEvent::MarketTrade((
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            PublicTrade {
                id: "test_7".to_string(),
                side: Side::Sell,
                price: 200.0,
                amount: 1.0,
            },
        )))
        .unwrap();

    tokio::time::sleep(latency_50ms()).await;

    // 检查与交易相关的基础和报价货币的AccountEvent Balances
    // Check AccountEvent Balances for base & quote currencies related to the trade
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balances(balances),
               ..
           }) => {
            // 应更新基础和报价SymbolBalances
            // Base & Quote SymbolBalances should be updated
            assert_eq!(balances.len(), 2);

            // 先检查基础余额：预期btc { total: 10.0 + 1.0 - 手续费, available: 10.0 + 1.0 - 手续费 }
            // Base Balance first: expected btc { total: 10.0 + 1.0 - fees, available: 10.0 + 1.0 - fees }
            let btc_fees = 1.0 * fees_50_percent();
            let expected_btc = SymbolBalance::new(
                "btc",
                Balance::new(10.0 + 1.0 - btc_fees, 10.0 + 1.0 - btc_fees),
            );
            assert_eq!(balances[0], expected_btc);

            // 然后检查报价余额：预期usdt Balance { total: 10_000 - 200, available: 9_700 }
            // Quote Balance second: expected usdt Balance { total: 10_000 - 200, available: 9_700 }
            let expected_usdt = SymbolBalance::new("usdt", Balance::new(9_800.0, 9_700.0));
            assert_eq!(balances[1], expected_usdt);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查匹配MarketEvent的AccountEvent Trade
    // Check AccountEvent Trade for order matching MarketEvent
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Trade(trade),
               ..
           }) => {
            // 预期的交易信息
            let expected = Trade {
                id: TradeId(1.to_string()),
                order_id: OrderId(3.to_string()),
                instrument: Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                side: Side::Buy,
                price: 200.0,
                quantity: 1.0,
                fees: SymbolFees::new("btc", 1.0 * fees_50_percent()),
            };
            assert_eq!(trade, expected);
        }
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "[CerebroBroker] : try_recv() consumed unexpected: {:?}",
                other
            );
        }
    }
}
// 8. 获取打开的订单并检查test_6_order_cid_1是否只剩一个限价买单。
// 8. Fetch open orders & check there is only one limit buy order remaining from test_6_order_cid_1.
async fn test_8_fetch_open_orders_and_check_test_6_order_cid_1_only(
    client: &SimulatedExecution,
    test_6_ids_1: Ids,
) {
    let open_orders = client.fetch_orders_open().await.unwrap();
    assert_eq!(open_orders.len(), 1);
    assert_eq!(
        open_orders[0].clone(),
        open_order(
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            test_6_ids_1.cid,
            test_6_ids_1.id,
            Side::Buy,
            100.0,
            1.0,
            0.0
        )
    );
}

// 9. 打开2x LIMIT Sell Order并检查是否发送了余额和订单新建的AccountEvents。
// 9. Open 2x LIMIT Sell Order & check AccountEvents for balances and order news are sent.
async fn test_9_open_2x_limit_sell_orders(
    client: &SimulatedExecution,
    test_9_ids_1: Ids,
    test_9_ids_2: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let opened_orders = client
        .open_orders(vec![
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_9_ids_1.cid,
                Side::Sell,
                500.0,
                1.0,
            ),
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_9_ids_2.cid,
                Side::Sell,
                1000.0,
                1.0,
            ),
        ])
        .await;

    let expected_order_new_1 = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_9_ids_1.cid,
        test_9_ids_1.id,
        Side::Sell,
        500.0,
        1.0,
        0.0,
    );

    let expected_order_new_2 = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_9_ids_2.cid,
        test_9_ids_2.id,
        Side::Sell,
        1000.0,
        1.0,
        0.0,
    );

    assert_eq!(opened_orders.len(), 2);
    assert_eq!(opened_orders[0].clone().unwrap(), expected_order_new_1);
    assert_eq!(opened_orders[1].clone().unwrap(), expected_order_new_2);

    // 检查第一个订单的AccountEvent Balance - 基础货币的可用余额减少
    // Check AccountEvent Balance for first order - base currency has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(btc_balance),
               ..
           }) => {
            // 预期btc Balance.available = 10.5 - 1.0
            // Expected btc Balance.available = 10.5 - 1.0
            let expected = SymbolBalance::new("btc", Balance::new(10.5, 10.5 - 1.0));
            assert_eq!(btc_balance, expected);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查第一个订单的AccountEvent OrdersNew
    // Check AccountEvent OrdersNew for first order
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_order_new_1);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查第二个订单的AccountEvent Balance - 基础货币的可用余额减少
    // Check AccountEvent Balance for second order - base currency has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(btc_balance),
               ..
           }) => {
            // 预期btc Balance.available = 9.5 - 1.0
            // Expected btc Balance.available = 9.5 - 1.0
            let expected = SymbolBalance::new("btc", Balance::new(10.5, 9.5 - 1.0));
            assert_eq!(btc_balance, expected);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查第二个订单的AccountEvent OrdersNew
    // Check AccountEvent OrdersNew for second order
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_order_new_2);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }
}

// 10. 发送一个完全匹配1x卖单的MarketEvent（交易），并部分匹配另一个（交易）。检查是否发送了两次匹配的余额和交易的AccountEvents。
// 10. Send MarketEvent that fully matches 1x sell Order (trade), and partially matches the another
//    (trade). Check AccountEvents for balances and trades of both matches are sent.
async fn test_10_send_market_event_that_full_and_partial_matches_orders(
    event_simulated_tx: &mut mpsc::UnboundedSender<SimulatedEvent>,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 发送一个完全匹配一个订单且部分匹配另一个订单的MarketEvent
    // Send MarketEvent that fully matches one order and partially matches another
    event_simulated_tx
        .send(SimulatedEvent::MarketTrade((
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            PublicTrade {
                id: "test_10".to_string(),
                side: Side::Buy,
                price: 1000.0,
                amount: 1.5,
            },
        )))
        .unwrap();

    tokio::time::sleep(latency_50ms()).await;

    // a) 第一个完全匹配的成交：
    // a) First full match fill:
    let first_full_fill_fees = (500.0 * 1.0) * fees_50_percent();
    // 检查与交易相关的基础和报价货币的AccountEvent Balances
    // Check AccountEvent Balances for base & quote currencies related to the trade
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balances(balances),
               ..
           }) => {
            // 基础和报价SymbolBalances应该更新
            // Base & Quote SymbolBalances should be updated
            assert_eq!(balances.len(), 2);

            // 首先检查基础余额：预期btc Balance { total: 10.5 - 1.0, available: 8.5 }
            // Base Balance first: expected btc Balance { total: 10.5 - 1.0, available: 8.5 }
            let expected_btc = SymbolBalance::new("btc", Balance::new(10.5 - 1.0, 8.5));
            assert_eq!(balances[0], expected_btc);

            // 然后检查报价余额：预期的usdt增加 = (500 * 1.0) - (500 * 1.0 * 0.5) = 500 - 250 = 250
            // Quote Balance second:
            // Expected usdt increase = (500 * 1.0) - (500 * 1.0 * 0.5) = 500 - 250 = 250
            // expected usdt Balance { total: 9_800 + 250, available: 9_700 + 250 }
            let expected_usdt = SymbolBalance::new("usdt", Balance::new(10_050.0, 9_950.0));
            assert_eq!(balances[1], expected_usdt);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查匹配MarketEvent的AccountEvent Trade
    // Check AccountEvent Trade for order matching MarketEvent
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Trade(trade),
               ..
           }) => {
            let expected = Trade {
                id: TradeId(2.to_string()),
                order_id: OrderId(4.to_string()),
                instrument: Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                side: Side::Sell,
                price: 500.0,
                quantity: 1.0,
                fees: SymbolFees::new("usdt", first_full_fill_fees),
            };
            assert_eq!(trade, expected);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // b) 第二个部分匹配的成交
    // b) Second partial match fill
    let second_partial_fill_fees = (1000.0 * 0.5) * fees_50_percent();

    // 检查与交易相关的基础和报价货币的AccountEvent Balances
    // Check AccountEvent Balances for base & quote currencies related to the trade
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balances(balances),
               ..
           }) => {
            // 基础和报价SymbolBalances应该更新
            // Base & Quote SymbolBalances should be updated
            assert_eq!(balances.len(), 2);

            // 首先检查基础余额：预期btc Balance { total: 9.5 - 0.5, available: 8.5 }
            // Base Balance first: expected btc Balance { total: 9.5 - 0.5, available: 8.5 }
            let expected_btc = SymbolBalance::new("btc", Balance::new(9.5 - 0.5, 8.5));
            assert_eq!(balances[0], expected_btc);

            // 然后检查报价余额：预期的usdt增加 = (1000 * 0.5) - (1000 * 0.5 * 0.5) = 500 - 250 = 250
            // Quote Balance second:
            // Expected usdt increase = (1000 * 0.5) - (1000 * 0.5 * 0.5) = 500 - 250 = 250
            // expected usdt Balance { total: 10_050 + 250, available: 9_950 + 250 }
            let expected_usdt =
                SymbolBalance::new("usdt", Balance::new(10_050.0 + 250.0, 9_950.0 + 250.0));
            assert_eq!(balances[1], expected_usdt);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查匹配MarketEvent的AccountEvent Trade
    // Check AccountEvent Trade for order matching MarketEvent
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Trade(trade),
               ..
           }) => {
            let expected = Trade {
                id: TradeId(3.to_string()),
                order_id: OrderId(5.to_string()),
                instrument: Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                side: Side::Sell,
                price: 1000.0,
                quantity: 0.5,
                fees: SymbolFees::new("usdt", second_partial_fill_fees),
            };
            assert_eq!(trade, expected);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }
}
// 11. 取消所有打开的订单。包括部分填充的卖单和未填充的买单。检查是否发送了订单取消和余额的AccountEvents。
// 11. Cancel all open orders. Includes a partially filled sell order, and non-filled buy order.
//     Check AccountEvents for orders cancelled and balances are sent.
async fn test_11_cancel_all_orders(
    client: &SimulatedExecution,
    test_6_ids_1: Ids,
    test_9_ids_2: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 取消所有订单
    let cancelled = client.cancel_orders_all().await.unwrap();

    // 预期的取消订单
    let expected_cancelled = vec![
        order_cancelled(
            // 先取消买单
            // Bids are cancelled first
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            test_6_ids_1.cid,
            Side::Buy,
            test_6_ids_1.id,
        ),
        order_cancelled(
            // 然后取消卖单
            // Asks are cancelled second
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            test_9_ids_2.cid,
            Side::Sell,
            test_9_ids_2.id,
        ),
    ];

    // 确认订单已被取消
    assert_eq!(cancelled.len(), 2);
    assert_eq!(cancelled[0].clone(), expected_cancelled[0]);
    assert_eq!(cancelled[1].clone(), expected_cancelled[1]);

    // 检查买卖单的AccountEvent Order是否已被取消
    // Check AccountEvent Order cancelled for both the bid & ask
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersCancelled(cancelled),
               ..
           }) => {
            assert_eq!(cancelled.len(), 2);
            assert_eq!(cancelled[0].clone(), expected_cancelled[0]);
            assert_eq!(cancelled[1].clone(), expected_cancelled[1]);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查买卖单取消后的AccountEvent Balances
    // Check AccountEvent Balances for cancelled bid and ask orders
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balances(balances),
               ..
           }) => {
            // 买单报价和卖单基础的SymbolBalances应更新
            // SymbolBalances for Bid order quote, & ask order base should be updated
            assert_eq!(balances.len(), 2);

            // 先取消买单，因此首先更新余额
            // test_6_order_cid_1, Side::Buy, price=100.0, quantity=1.0
            // 因此，usdt Balance { total: 10_300, available: 10_200 + (100 * 1)
            // Bids are cancelled first, so balance is updated first
            let expected_usdt =
                SymbolBalance::new("usdt", Balance::new(10_300.0, 10_200.0 + 100.0));
            assert_eq!(balances[0], expected_usdt);

            // 然后取消卖单，因此随后更新余额
            // test_9_order_cid_2, Side::Sell, price=1000.0, quantity=1.0, filled=0.5
            // 因此，btc Balance { total: 9.0, available: 8.5 + 0.5 }
            // Asks are cancelled second, so balance is updated second
            let expected_btc = SymbolBalance::new("btc", Balance::new(9.0, 8.5 + 0.5));
            assert_eq!(balances[1], expected_btc);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }
}

// 12. 获取打开的订单（现在我们已经调用了cancel_all）并检查是否为空
// 12. Fetch open orders (now that we've called cancel_all) and check it is empty.
async fn test_12_fetch_open_orders_and_check_empty(client: &SimulatedExecution) {
    let open_orders = client.fetch_orders_open().await.unwrap();
    // 确认打开的订单列表为空
    assert!(open_orders.is_empty());
}

// 13. 由于资金不足，未能打开限价买单
// 13. Fail to open limit buy order with insufficient funds.
async fn test_13_fail_to_open_one_of_two_limits_with_insufficient_funds(
    client: &SimulatedExecution,
    test_13_ids_1: Ids,
    test_13_ids_2: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    // 尝试打开两个限价订单，其中一个由于资金不足而失败
    let opened_orders = client
        .open_orders(vec![
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_13_ids_1.cid,
                Side::Buy,
                1_000_000_000.0,
                1.0,
            ),
            order_request_limit(
                Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
                test_13_ids_2.cid,
                Side::Sell,
                1000.0,
                1.0,
            ),
        ])
        .await;

    // 预期第一个订单由于资金不足而失败
    let expected_order_new_1 = Err(ExecutionError::InsufficientBalance(Symbol::from("usdt")));
    // 预期第二个订单成功创建
    let expected_order_new_2 = open_order(
        Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
        test_13_ids_2.cid,
        test_13_ids_2.id,
        Side::Sell,
        1000.0,
        1.0,
        0.0,
    );

    // 确认订单创建结果
    assert_eq!(opened_orders.len(), 2);
    assert_eq!(opened_orders[0].clone(), expected_order_new_1);
    assert_eq!(opened_orders[1].clone().unwrap(), expected_order_new_2);

    // 第一个订单由于usdt资金不足失败，因此不期待任何AccountEvents

    // 检查第二个订单的AccountEvent Balance - 基础货币的可用余额减少
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::Balance(btc_balance),
               ..
           }) => {
            // 预期btc Balance.available = 9.0 - 1.0
            let expected = SymbolBalance::new("btc", Balance::new(9.0, 9.0 - 1.0));
            assert_eq!(btc_balance, expected);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查第二个订单的AccountEvent OrdersNew
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
               kind: AccountEventKind::OrdersNew(new_orders),
               ..
           }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone(), expected_order_new_2);
        }
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }

    // 检查是否没有更多的AccountEvents生成
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!(
                "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
                other
            );
        }
    }
}

// 14. 使用错误的OrderId尝试取消限价订单并失败，因为找不到订单
// 14. Fail to cancel limit order with OrderNotFound using incorrect OrderId.
async fn test_14_fail_to_cancel_limit_with_order_not_found(client: &SimulatedExecution) {
    // 生成一个新的客户端订单ID
    let cid = ClientOrderId(Uuid::new_v4());
    // 尝试取消一个不存在的订单
    let cancelled = client
        .cancel_orders(vec![order_cancel_request(
            Instrument::from(("btc", "usdt", InstrumentKind::Perpetual)),
            cid,
            Side::Buy,
            OrderId::from("order will not be found"),
        )])
        .await;

    // 预期结果为找不到订单的错误
    let expected = Err(ExecutionError::OrderNotFound(cid));

    // 验证取消操作的结果
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0], expected);
}

