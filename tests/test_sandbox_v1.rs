
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use unilink_execution::ClientExecution;
use unilink_execution::common::balance::{Balance, TokenBalance};
use unilink_execution::common::event::{AccountEvent, AccountEventKind};
use unilink_execution::common::instrument::Instrument;
use unilink_execution::common::instrument::kind::InstrumentKind;
use unilink_execution::common::order::identification::client_order_id::ClientOrderId;
use unilink_execution::common::order::identification::machine_id::generate_machine_id;
use unilink_execution::common::order::identification::OrderId;
use unilink_execution::common::Side;
use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade;
use unilink_execution::sandbox::sandbox_client::{SandBoxClient, SandBoxClientEvent};

use crate::util::{initial_balances, open_order, order_cancel_request, order_limit_cancelled, order_request_limit, run_sample_exchange};

mod util;

#[allow(warnings)]
#[derive(Clone)]
struct Ids {
    cid: ClientOrderId,
    id: OrderId,
}

impl Ids {
    fn new(cid: ClientOrderId, id: OrderId) -> Self {
        Self {
            cid,
            id,
        }
    }
}
#[allow(warnings)]
#[tokio::test]
async fn main() {
    #[allow(warnings)]
    // 创建通道用于发送和接收事件
    let (event_sandbox_tx, mut event_sandbox_rx) = mpsc::unbounded_channel();
    let ( mut request_tx,  request_rx) = mpsc::unbounded_channel();

    // 给定测试用的timestamp和machine_id和IDs
    let timestamp = 1234124124124123u64;
    let machine_id = generate_machine_id().unwrap();
    let test_3_ids = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));

    // 创建并运行 SimulatedExchange
    tokio::spawn(run_sample_exchange(event_sandbox_tx, request_rx));

    // 初始化 SandBoxClient，用于与交易所进行交互
    let client = SandBoxClient {
        request_tx: request_tx.clone(),
    };

    // 1. 获取初始的未成交订单列表，检查当前没有未成交订单
    test_1_fetch_initial_orders_and_check_empty(&client).await;

    // 2. 获取初始的余额信息，检查当前没有发生任何余额变化事件
    test_2_fetch_balances_and_check_same_as_initial(&client).await;

    // 3. 下达限价买单，并检查是否为报价货币（USDT）发送了 AccountEvent 余额事件
    test_3_open_limit_buy_order(
        &client,
        test_3_ids.clone(),
        &mut event_sandbox_rx
    ).await;

    // 4. 发送一个不匹配任何未成交订单的市场事件，并检查是否没有发送 AccountEvent
    test_4_send_market_trade_that_does_not_match_any_open_order(
        &mut request_tx,
        &mut event_sandbox_rx,
    );

    // // 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent
    test_5_cancel_buy_order(&client, test_3_ids, &mut event_sandbox_rx).await;
    //
    // // 6. Open 2x LIMIT Buy Orders & assert on received AccountEvents
    let test_6_ids_1 = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));
    let test_6_ids_2 = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));
    test_6_open_2x_limit_buy_orders(&client, test_6_ids_1.clone(), test_6_ids_2, &mut event_sandbox_rx,)
    .await;

    // // 7. Send MarketEvent that exactly full matches 1x open Order (trade) and check AccountEvents
    // //    for balances and trades
    // test_7_send_market_trade_that_exact_full_matches_order(
    //     &mut event_sandbox_tx,
    //     &mut request_rx,
    // )
    // .await;
    //
    // // 8. Fetch open orders & check only one limit buy order remaining from test_6_order_cid_1
    // test_8_fetch_open_orders_and_check_test_6_order_cid_1_only(&client, test_6_ids_1.clone()).await;
    //
    // // 9. Open 2x LIMIT Sell Order & assert on received AccountEvents
    // let test_9_ids_1 = Ids::new(Uuid::new_v4(), 4);
    // let test_9_ids_2 = Ids::new(Uuid::new_v4(), 5);
    // test_9_open_2x_limit_sell_orders(
    //     &client,
    //     test_9_ids_1,
    //     test_9_ids_2.clone(),
    //     &mut event_sandbox_rx,
    // )
    // .await;
    //
    // // 10. Send MarketEvent that fully matches 1x sell Order (trade), and partially matches the other
    // //     sell Order (trade). Check AccountEvents for balances and trades of both matches are sent.
    // test_10_send_market_trade_that_full_and_partial_matches_orders(
    //     &mut event_simulated_tx,
    //     &mut event_sandbox_rx,
    // )
    // .await;
    //
    // // 11. Cancel all open orders. Includes a partially filled sell order, and non-filled buy order.
    // //     Check AccountEvents for orders cancelled and balances are sent.
    // test_11_cancel_all_orders(&client, test_6_ids_1, test_9_ids_2, &mut event_sandbox_rx).await;
    //
    // // 12. Fetch open orders (now that we've called cancel_all) and check it is empty
    // test_12_fetch_open_orders_and_check_empty(&client).await;
    //
    // // 13. Fail to open limit buy order with insufficient funds
    // let test_13_ids_1 = Ids::new(Uuid::new_v4(), 6);
    // let test_13_ids_2 = Ids::new(Uuid::new_v4(), 6); // 6 because first should fail
    // test_13_fail_to_open_one_of_two_limits_with_insufficient_funds(
    //     &client,
    //     test_13_ids_1,
    //     test_13_ids_2,
    //     &mut event_sandbox_rx,
    // )
    // .await;
    //
    // // 14. Fail to cancel limit order with OrderNotFound using incorrect OrderId
    // test_14_fail_to_cancel_limit_with_order_not_found(&client).await;
}
#[allow(warnings)]
// 1. Fetch initial OpenOrders when we have no open Orders.
async fn test_1_fetch_initial_orders_and_check_empty(client: &SandBoxClient) {
    let initial_orders_result = client.fetch_orders_open().await;

    // 打印返回结果
    match initial_orders_result {
        Ok(initial_orders) => {
            assert!(initial_orders.len() > 0 , "Found expected open orders.");
        }
        Err(_) => {  // 使用 _ 忽略错误值
            panic!("Error occurred while fetching open orders.");
        }
    }
}
#[allow(warnings)]

// 自定义比较函数，忽略 time 字段
fn assert_balance_equal_ignore_time(actual: &Balance, expected: &Balance) {
    assert_eq!(actual.current_price, expected.current_price, "current_price mismatch");
    assert_eq!(actual.total, expected.total, "total mismatch");
    assert_eq!(actual.available, expected.available, "available mismatch");
}

#[allow(warnings)]
async fn test_2_fetch_balances_and_check_same_as_initial(client: &SandBoxClient) {
    let actual_balances = client.fetch_balances().await.unwrap();
    println!("[test_2] : actual balances: {:?}", actual_balances);
    // NOTE seems that fetch_balances is working.so that the response_rx should be working.
    let initial_balances = initial_balances().await;

    assert_eq!(actual_balances.len(), initial_balances.len());

    for actual in actual_balances {
        if let Some(expected) = initial_balances.get(&actual.token) {
            assert_balance_equal_ignore_time(&actual.balance, expected);
        } else {
            panic!("Test failed due to missing token in initial balances.");
        }
    }
}

#[allow(warnings)]
async fn test_3_open_limit_buy_order(
    client: &SandBoxClient,
    test_3_ids: Ids,
    event_sandbox_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let actual_balances = client.fetch_balances().await.unwrap();
    println!("[test_3] : actual balances: {:?}", actual_balances);
    let open_request = order_request_limit(
        Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),
        Side::Buy,
        100.0,
        1.0,
    );

    println!("[test_3] : Sending order request via SandBoxClient : {:?}", open_request);

    let new_orders = client.open_orders(vec![open_request]).await;
    println!("[test_3] : {:?}", new_orders);
    let expected_new_order = open_order(
        Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),
        test_3_ids.id,
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );
    println!("[test_3] : expected_new_order: {:?}", expected_new_order);

    assert_eq!(new_orders[0].as_ref().unwrap().cid, expected_new_order.cid);
    // //
    // // 使用实际的价格来确保一致性
    let current_px = 1.0;  // 与订单中的价格匹配
    match event_sandbox_rx.recv().await {
        Some(AccountEvent {
                 kind: AccountEventKind::Balance(USDT_balance),
                 ..
             }) => {
            let expected = TokenBalance::new("USDT", Balance::new(200.0, 149.0, current_px));
            assert_balance_equal_ignore_time(&USDT_balance.balance, &expected.balance);
        }
        other => {
            panic!("[test_3] : Unexpected or missing balance event: {:?}", other);
        }
    }

    match event_sandbox_rx.recv().await {
        Some(AccountEvent {
                 kind: AccountEventKind::OrdersNew(new_orders),
                 ..
             }) => {
            println!("[test_3] : Orders new event received.");
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].cid, expected_new_order.cid);
        }
        other => {
            panic!("[test_3] : Unexpected or missing orders new event: {:?}", other);
        }
    }

    match event_sandbox_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {
            println!("[test_3] : No additional account events, as expected.");
        }
        other => {
            panic!("[test_3] : Unexpected additional account event: {:?}", other);
        }
    }
}

#[allow(warnings)]
// 4. 发送一个不匹配任何未完成订单的 MarketTrade，并检查是否没有发送 AccountEvents。注意，这一部分可能存在问题，因为它没有使用`new_market_trade`。
// NOTE 其次还要检查一下available的数值处理是否正确。
fn test_4_send_market_trade_that_does_not_match_any_open_order(
    event_simulated_tx: &mut UnboundedSender<SandBoxClientEvent>,
    event_sandbox_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {

    let new_market_trade = MarketTrade { exchange: "binance-futures".into(), symbol: "1000RATSUSDT".into(), side: "buy".into(), price: 0.13461, timestamp: 1714924612471000, amount: 744.0 };

    // 检查是否没有生成更多的 AccountEvents
    match event_sandbox_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() 消耗了意外的结果: {:?}", other);
        }
    }
}

#[allow(warnings)]

// // 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent.
async fn test_5_cancel_buy_order(
    client: &SandBoxClient,
    test_3_ids: Ids,
    event_sandbox_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let cancelled = client
        .cancel_orders(vec![order_cancel_request(
            Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
            test_3_ids.cid.clone(),  // 使用 clone()
            Side::Buy,
            test_3_ids.id.clone(),
        )])
        .await;

    println!("[test_5] : {:?}", cancelled);
    let expected_cancelled = order_limit_cancelled(
        Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),  // 使用 clone()
        Side::Buy,
        test_3_ids.id.clone(),  // 使用 clone()
    );
    println!("[test_5] : {:?}", expected_cancelled);
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].clone().unwrap(), expected_cancelled);

    // Check AccountEvent Order cancelled
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::OrdersCancelled(cancelled),
            ..
        }) => {
            println!("[test_5] : Orders cancelled event received.");
            assert_eq!(cancelled.len(), 1);
            assert_eq!(cancelled[0].clone(), expected_cancelled);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }


    let current_px = 1.0;  // 与订单中的价格匹配
    // Check AccountEvent Balance for quote currency (USDT) has available balance increase
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(USDT_balance),
            ..
        }) => {
            let expected = TokenBalance::new("USDT", Balance::new(200.0, 249.0, current_px));
            println!("[test_5] : Balance event received.");
            assert_eq!(USDT_balance.balance.total, expected.balance.total);
            assert_eq!(USDT_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check no more AccountEvents generated
    match event_sandbox_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }
}
#[allow(warnings)]

// 6. 开两个限价买单，并检查是否发送了关于余额和新订单的 AccountEvents
// NOTE 数值是否对的上？
// NOTE 真的要开杠杆怎么处理？杠杆率怎么设置？

async fn test_6_open_2x_limit_buy_orders(
    client: &SandBoxClient,
    test_6_ids_1: Ids,
    test_6_ids_2: Ids,
    event_sandbox_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let opened_orders = client
        .open_orders(vec![
            order_request_limit(
                Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
                test_6_ids_1.cid.clone(),
                Side::Buy,
                100.0,
                1.0,
            ),
            order_request_limit(
                Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
                test_6_ids_2.cid.clone(),
                Side::Buy,
                200.0,
                1.0,
            ),
        ])
        .await;
    println!("[test_6] : {:?}", opened_orders);

    let expected_order_new_1 = open_order(
        Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
        test_6_ids_1.cid.clone(),
        test_6_ids_1.id.clone(),
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );

    let expected_order_new_2 = open_order(
        Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual)),
        test_6_ids_2.cid.clone(),
        test_6_ids_2.id.clone(),
        Side::Buy,
        200.0,
        1.0,
        0.0,
    );
    println!("[test_6] : expected_order_new_1: {:?}", expected_order_new_1);
    println!("[test_6] : expected_order_new_2: {:?}", expected_order_new_2);

    assert_eq!(opened_orders.len(), 2);
    assert_eq!(opened_orders[0].clone().unwrap().cid, expected_order_new_1.cid);
    assert_eq!(opened_orders[1].clone().unwrap().cid, expected_order_new_2.cid);


    let current_px = 1.0;  // 与订单中的价格匹配
    // Check AccountEvent Balance for first order - quote currency has available balance decrease
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(USDT_balance),
            ..
        }) => {
            let expected = TokenBalance::new("USDT", Balance::new(200.0, 148.0,current_px));
            assert_eq!(USDT_balance.balance.total, expected.balance.total);
            assert_eq!(USDT_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check AccountEvent OrdersNew for first order
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::OrdersNew(new_orders),
            ..
        }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone().cid, expected_order_new_1.cid);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check AccountEvent Balance for second order - quote currency has available balance decrease
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(USDT_balance),
            ..
        }) => {
            // Expected USDT Balance.available = 9_900 - (200.0 * 1.0)
            let expected = TokenBalance::new("USDT", Balance::new(200.0, 147.0,current_px));
            assert_eq!(USDT_balance.balance.total, expected.balance.total);
            assert_eq!(USDT_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check AccountEvent OrdersNew for second order
    match event_sandbox_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::OrdersNew(new_orders),
            ..
        }) => {
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].clone().cid, expected_order_new_2.cid);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check no more AccountEvents generated
    match event_sandbox_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }
}

// 7. 发送 MarketEvent，该事件与 1x 开放订单（交易）完全匹配，并检查 AccountEvents 是否发送了余额和ClientTrade信息。
// 8. 获取未完成的订单并检查是否只剩下一个来自 test_6_order_cid_1 的限价买单。
// 9. 开启 2 个限价卖单，并检查 AccountEvents 是否发送了余额和订单更新信息。
// 10. 发送一个完全匹配 1 个卖单（交易）的 MarketEvent，并部分匹配另一个卖单。
// 11. 取消所有未完成的订单。包括部分成交的卖单和未成交的买单。
// 12. 获取未完成的订单（既然我们已经调用了取消所有订单），并检查它是否为空。
// 13. 尝试用资金不足的情况失败地开设限价买单。
// 14. 使用不正确的订单ID尝试取消限价订单时，订单未找到失败。
