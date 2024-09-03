
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

#[tokio::test]
async fn main() {
    // 创建通道用于发送和接收事件
    let ( mut request_tx,  request_rx) = mpsc::unbounded_channel();
    let (event_account_tx, mut event_account_rx) = mpsc::unbounded_channel();

    // additional initialization
    let timestamp = 1234124124124123u64;
    let machine_id = generate_machine_id().unwrap();
    let test_3_ids = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));

    // 创建并运行 SimulatedExchange
    tokio::spawn(run_sample_exchange(event_account_tx, request_rx));

    // 初始化 SandBoxClient，用于与交易所进行交互
    let client = SandBoxClient {
        request_tx: request_tx.clone(),
    };

    // 1. 获取初始的未成交订单列表，检查当前没有未成交订单
    // test_1_fetch_initial_orders_and_check_empty(&client).await;

    // 2. 获取初始的余额信息，检查当前没有发生任何余额变化事件
    // test_2_fetch_balances_and_check_same_as_initial(&client).await;

    // 3. 下达限价买单，并检查是否为报价货币（TEST_QUOTE）发送了 AccountEvent 余额事件
    // test_3_open_limit_buy_order(
    //     &client,
    //     test_3_ids.clone(),
    //     &mut event_account_rx
    // ).await;

    // 4. 发送一个不匹配任何未成交订单的市场事件，并检查是否没有发送 AccountEvent
    // test_4_send_market_event_that_does_not_match_any_open_order(
    //     &mut request_tx,
    //     &mut event_account_rx,
    // );

    // // 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent
    // test_5_cancel_buy_order(&client, test_3_ids, &mut event_account_rx).await;
    //
    // // 6. Open 2x LIMIT Buy Orders & assert on received AccountEvents
    // let test_6_ids_1 = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));
    // let test_6_ids_2 = Ids::new(ClientOrderId(Some("test_cid".to_string())), OrderId(1234124124124123));
    // test_6_open_2x_limit_buy_orders(&client, test_6_ids_1.clone(), test_6_ids_2, &mut event_account_rx,)
    // .await;
    //
    // // 7. Send MarketEvent that exactly full matches 1x open Order (trade) and check AccountEvents
    // //    for balances and trades
    // test_7_send_market_event_that_exact_full_matches_order(
    //     &mut request_tx,
    //     &mut event_account_rx,
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
    //     &mut event_account_rx,
    // )
    // .await;
    //
    // // 10. Send MarketEvent that fully matches 1x sell Order (trade), and partially matches the other
    // //     sell Order (trade). Check AccountEvents for balances and trades of both matches are sent.
    // test_10_send_market_event_that_full_and_partial_matches_orders(
    //     &mut event_simulated_tx,
    //     &mut event_account_rx,
    // )
    // .await;
    //
    // // 11. Cancel all open orders. Includes a partially filled sell order, and non-filled buy order.
    // //     Check AccountEvents for orders cancelled and balances are sent.
    // test_11_cancel_all_orders(&client, test_6_ids_1, test_9_ids_2, &mut event_account_rx).await;
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
    //     &mut event_account_rx,
    // )
    // .await;
    //
    // // 14. Fail to cancel limit order with OrderNotFound using incorrect OrderId
    // test_14_fail_to_cancel_limit_with_order_not_found(&client).await;
}

// 1. Fetch initial OpenOrders when we have no open Orders.
async fn test_1_fetch_initial_orders_and_check_empty(client: &SandBoxClient) {
    let initial_orders_result = client.fetch_orders_open().await;

    // 打印返回结果
    match initial_orders_result {
        Ok(initial_orders) => {
            assert!(initial_orders.is_empty(), "Expected no open orders, but found some.");
        }
        Err(_) => {  // 使用 _ 忽略错误值
            panic!("Error occurred while fetching open orders.");
        }
    }
}
// 自定义比较函数，忽略 time 字段
fn assert_balance_equal_ignore_time(actual: &Balance, expected: &Balance) {
    assert_eq!(actual.current_price, expected.current_price, "current_price mismatch");
    assert_eq!(actual.total, expected.total, "total mismatch");
    assert_eq!(actual.available, expected.available, "available mismatch");
}


async fn test_2_fetch_balances_and_check_same_as_initial(client: &SandBoxClient) {
    let actual_balances = client.fetch_balances().await.unwrap();
    println!("actual balances: {:?}", actual_balances);
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

async fn test_3_open_limit_buy_order(
    client: &SandBoxClient,
    test_3_ids: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let actual_balances = client.fetch_balances().await.unwrap();
    println!("actual balances: {:?}", actual_balances);
    let open_request = order_request_limit(
        Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),
        Side::Buy,
        100.0,
        1.0,
    );

    println!("[test_3_open_limit_buy_order] : Sending order request via SandBoxClient : {:?}", open_request);

    let new_orders = client.open_orders(vec![open_request]).await;
    println!("[test_3_open_limit_buy_order]: {:?}", new_orders);
    let expected_new_order = open_order(
        Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),
        test_3_ids.id,
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );
    println!("[test_3_open_limit_buy_order] : expected_new_order: {:?}", expected_new_order);

    assert_eq!(new_orders[0].as_ref().unwrap().cid, expected_new_order.cid);
    // //
    // // 使用实际的价格来确保一致性
    let current_px = 1.0;  // 与订单中的价格匹配
    match event_account_rx.recv().await {
        Some(AccountEvent {
                 kind: AccountEventKind::Balance(TEST_QUOTE_balance),
                 ..
             }) => {
            let expected = TokenBalance::new("TEST_QUOTE", Balance::new(200.0, 149.0, current_px));
            assert_balance_equal_ignore_time(&TEST_QUOTE_balance.balance, &expected.balance);
        }
        other => {
            panic!("Unexpected or missing balance event: {:?}", other);
        }
    }

    match event_account_rx.recv().await {
        Some(AccountEvent {
                 kind: AccountEventKind::OrdersNew(new_orders),
                 ..
             }) => {
            println!("Orders new event received.");
            assert_eq!(new_orders.len(), 1);
            assert_eq!(new_orders[0].cid, expected_new_order.cid);
        }
        other => {
            panic!("Unexpected or missing orders new event: {:?}", other);
        }
    }

    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {
            println!("No additional account events, as expected.");
        }
        other => {
            panic!("Unexpected additional account event: {:?}", other);
        }
    }
}


// 4. 发送一个不匹配任何未完成订单的 MarketEvent，并检查是否没有发送 AccountEvents。注意，这一部分可能存在问题，因为它没有使用`new_market_event`。
// NOTE 其次还要检查一下available的数值处理是否正确。
fn test_4_send_market_trade_that_does_not_match_any_open_order(
    event_simulated_tx: &mut UnboundedSender<SandBoxClientEvent>,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {

    let new_market_trade = MarketTrade { exchange: "binance-futures".into(), symbol: "1000RATSUSDT".into(), side: "buy".into(), price: 0.13461, timestamp: 1714924612471000, amount: 744.0 };

    // 检查是否没有生成更多的 AccountEvents
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() 消耗了意外的结果: {:?}", other);
        }
    }
}

// // 5. Cancel the open buy order and check AccountEvents for cancelled order and balance are sent.
async fn test_5_cancel_buy_order(
    client: &SandBoxClient,
    test_3_ids: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let cancelled = client
        .cancel_orders(vec![order_cancel_request(
            Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
            test_3_ids.cid.clone(),  // 使用 clone()
            Side::Buy,
            test_3_ids.id.clone(),
        )])
        .await;

    let expected_cancelled = order_limit_cancelled(
        Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
        test_3_ids.cid.clone(),  // 使用 clone()
        Side::Buy,
        test_3_ids.id.clone(),  // 使用 clone()
    );

    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].clone().unwrap(), expected_cancelled);

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
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }


    let current_px = 1.0;  // 与订单中的价格匹配
    // Check AccountEvent Balance for quote currency (TEST_QUOTE) has available balance increase
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(TEST_QUOTE_balance),
            ..
        }) => {
            let expected = TokenBalance::new("TEST_QUOTE", Balance::new(200.0, 250.0, current_px));
            assert_eq!(TEST_QUOTE_balance.balance.total, expected.balance.total);
            assert_eq!(TEST_QUOTE_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check no more AccountEvents generated
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }
}

// 6. 开两个 2 倍杠杆的限价买单，并检查是否发送了关于余额和新订单的 AccountEvents
async fn test_6_open_2x_limit_buy_orders(
    client: &SandBoxClient,
    test_6_ids_1: Ids,
    test_6_ids_2: Ids,
    event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
) {
    let opened_orders = client
        .open_orders(vec![
            order_request_limit(
                Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
                test_6_ids_1.cid.clone(),
                Side::Buy,
                100.0,
                1.0,
            ),
            order_request_limit(
                Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
                test_6_ids_2.cid.clone(),
                Side::Buy,
                200.0,
                1.0,
            ),
        ])
        .await;

    let expected_order_new_1 = open_order(
        Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
        test_6_ids_1.cid.clone(),
        test_6_ids_1.id.clone(),
        Side::Buy,
        100.0,
        1.0,
        0.0,
    );

    let expected_order_new_2 = open_order(
        Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
        test_6_ids_2.cid.clone(),
        test_6_ids_2.id.clone(),
        Side::Buy,
        200.0,
        1.0,
        0.0,
    );

    assert_eq!(opened_orders.len(), 2);
    assert_eq!(opened_orders[0].clone().unwrap().cid, expected_order_new_1.cid);
    assert_eq!(opened_orders[1].clone().unwrap().cid, expected_order_new_2.cid);


    let current_px = 1.0;  // 与订单中的价格匹配
    // Check AccountEvent Balance for first order - quote currency has available balance decrease
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(TEST_QUOTE_balance),
            ..
        }) => {
            let expected = TokenBalance::new("TEST_QUOTE", Balance::new(200.0, 149.0,current_px));
            assert_eq!(TEST_QUOTE_balance.balance.total, expected.balance.total);
            assert_eq!(TEST_QUOTE_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check AccountEvent OrdersNew for first order
    match event_account_rx.try_recv() {
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
    match event_account_rx.try_recv() {
        Ok(AccountEvent {
            kind: AccountEventKind::Balance(TEST_QUOTE_balance),
            ..
        }) => {
            // Expected TEST_QUOTE Balance.available = 9_900 - (200.0 * 1.0)
            let expected = TokenBalance::new("TEST_QUOTE", Balance::new(200.0, 148.0,current_px));
            assert_eq!(TEST_QUOTE_balance.balance.total, expected.balance.total);
            assert_eq!(TEST_QUOTE_balance.balance.available, expected.balance.available);
        }
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }

    // Check AccountEvent OrdersNew for second order
    match event_account_rx.try_recv() {
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
    match event_account_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        other => {
            panic!("try_recv() consumed unexpected: {:?}", other);
        }
    }
}
//
// 7. 发送 MarketEvent，该事件与 1x 开放订单（交易）完全匹配，并检查 AccountEvents 是否发送了余额和ClientTrade信息。
// async fn test_7_send_market_event_that_exact_full_matches_order(
//     event_simulated_tx: &mut mpsc::UnboundedSender<SandBoxClientEvent>,
//     event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
// ) {
//     // Send matching MarketEvent
//     event_simulated_tx
//         .send(SandBoxClientEvent::MarketTrade((
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             PublicTrade {
//                 id: "test_7".to_string(),
//                 side: Side::Sell,
//                 price: 200.0,
//                 amount: 1.0,
//             },
//         )))
//         .unwrap();
//
//
//     // Check AccountEvent Balances for base & quote currencies related to the trade
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balances(balances),
//             ..
//         }) => {
//             // Base & Quote SymbolBalances should be updated
//             assert_eq!(balances.len(), 2);
//
//             // Base Balance first: expected TEST_BASE { total: 10.0 + 1.0 - fees, available: 10.0 + 1.0 - fees }
//             let TEST_BASE_fees = 1.0 * fees_50_percent();
//             let expected_TEST_BASE = TokenBalance::new(
//                 "TEST_BASE",
//                 Balance::new(10.0 + 1.0 - TEST_BASE_fees, 10.0 + 1.0 - TEST_BASE_fees),
//             );
//             assert_eq!(balances[0], expected_TEST_BASE);
//
//             // Quote Balance second: expected TEST_QUOTE Balance { total: 10_000 - 200, available: 9_700 }
//             let expected_TEST_QUOTE = TokenBalance::new("TEST_QUOTE", Balance::new(9_800.0, 9_700.0));
//             assert_eq!(balances[1], expected_TEST_QUOTE);
//         }
//         other => {
//             panic!("try_recv() consumed unexpected: {:?}", other);
//         }
//     }
//
//     // Check AccountEvent Trade for order matching MarketEvent
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Trade(trade),
//             ..
//         }) => {
//             let expected = Trade {
//                 id: TradeId(1.to_string()),
//                 order_id: OrderId(3.to_string()),
//                 instrument: Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 side: Side::Buy,
//                 price: 200.0,
//                 quantity: 1.0,
//                 fees: SymbolFees::new("TEST_BASE", 1.0 * fees_50_percent()),
//             };
//             assert_eq!(trade, expected);
//         }
//         other => {
//             panic!("try_recv() consumed unexpected: {:?}", other);
//         }
//     }
//
//     // Check no more AccountEvents generated
//     match event_account_rx.try_recv() {
//         Err(mpsc::error::TryRecvError::Empty) => {}
//         other => {
//             panic!("try_recv() consumed unexpected: {:?}", other);
//         }
//     }
// }

// 8. 获取未完成的订单并检查是否只剩下一个来自 test_6_order_cid_1 的限价买单。
// async fn test_8_fetch_open_orders_and_check_test_6_order_cid_1_only(
//     client: &SandBoxClient,
//     test_6_ids_1: Ids,
// ) {
//     let open_orders = client.fetch_orders_open().await.unwrap();
//     assert_eq!(open_orders.len(), 1);
//     assert_eq!(
//         open_orders[0].clone(),
//         open_order(
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             test_6_ids_1.cid,
//             test_6_ids_1.id,
//             Side::Buy,
//             100.0,
//             1.0,
//             0.0
//         )
//     );
// }
//
// 9. 开启 2 个限价卖单，并检查 AccountEvents 是否发送了余额和订单更新信息。
// async fn test_9_open_2x_limit_sell_orders(
//     client: &SandBoxClient,
//     test_9_ids_1: Ids,
//     test_9_ids_2: Ids,
//     event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
// ) {
//     let opened_orders = client
//         .open_orders(vec![
//             order_request_limit(
//                 Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 test_9_ids_1.cid,
//                 Side::Sell,
//                 500.0,
//                 1.0,
//             ),
//             order_request_limit(
//                 Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 test_9_ids_2.cid,
//                 Side::Sell,
//                 1000.0,
//                 1.0,
//             ),
//         ])
//         .await;
//
//     let expected_order_new_1 = open_order(
//         Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//         test_9_ids_1.cid,
//         test_9_ids_1.id,
//         Side::Sell,
//         500.0,
//         1.0,
//         0.0,
//     );
//
//     let expected_order_new_2 = open_order(
//         Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//         test_9_ids_2.cid,
//         test_9_ids_2.id,
//         Side::Sell,
//         1000.0,
//         1.0,
//         0.0,
//     );
//
//     assert_eq!(opened_orders.len(), 2);
//     assert_eq!(opened_orders[0].clone().unwrap(), expected_order_new_1);
//     assert_eq!(opened_orders[1].clone().unwrap(), expected_order_new_2);
//
//     // Check AccountEvent Balance for first order - quote currency has available balance decrease
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balance(TEST_BASE_balance),
//             ..
//         }) => {
//             // Expected TEST_BASE Balance.available = 10.5 - 1.0
//             let expected = TokenBalance::new("TEST_BASE", Balance::new(10.5, 10.5 - 1.0));
//             assert_eq!(TEST_BASE_balance, expected);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check AccountEvent OrdersNew for first order
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::OrdersNew(new_orders),
//             ..
//         }) => {
//             assert_eq!(new_orders.len(), 1);
//             assert_eq!(new_orders[0].clone(), expected_order_new_1);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check AccountEvent Balance for second order - quote currency has available balance decrease
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balance(TEST_BASE_balance),
//             ..
//         }) => {
//             // Expected TEST_BASE Balance.available = 9.5 - 1.0
//             let expected = TokenBalance::new("TEST_BASE", Balance::new(10.5, 9.5 - 1.0));
//             assert_eq!(TEST_BASE_balance, expected);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check AccountEvent OrdersNew for second order
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::OrdersNew(new_orders),
//             ..
//         }) => {
//             assert_eq!(new_orders.len(), 1);
//             assert_eq!(new_orders[0].clone(), expected_order_new_2);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check no more AccountEvents generated
//     match event_account_rx.try_recv() {
//         Err(mpsc::error::TryRecvError::Empty) => {}
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
// }
//
// 10. 发送一个完全匹配 1 个卖单（交易）的 MarketEvent，并部分匹配另一个卖单。
// //    (trade). Check AccountEvents for balances and trades of both matches are sent.
// async fn test_10_send_market_event_that_full_and_partial_matches_orders(
//     event_simulated_tx: &mut mpsc::UnboundedSender<SandBoxClientEvent>,
//     event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
// ) {
//     // Send MarketEvent that fully matches one order and partially matches another
//     event_simulated_tx
//         .send(SandBoxClientEvent::MarketTrade((
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             PublicTrade {
//                 id: "test_10".to_string(),
//                 side: Side::Buy,
//                 price: 1000.0,
//                 amount: 1.5,
//             },
//         )))
//         .unwrap();
//
//     tokio::time::sleep(latency_50ms()).await;
//
//     // a) First full match fill:
//     let first_full_fill_fees = (500.0 * 1.0) * fees_50_percent();
//     // Check AccountEvent Balances for base & quote currencies related to the trade
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balances(balances),
//             ..
//         }) => {
//             // Base & Quote SymbolBalances should be updated
//             assert_eq!(balances.len(), 2);
//
//             // Base Balance first: expected TEST_BASE Balance { total: 10.5 - 1.0, available: 8.5 }
//             let expected_TEST_BASE = TokenBalance::new("TEST_BASE", Balance::new(10.5 - 1.0, 8.5));
//             assert_eq!(balances[0], expected_TEST_BASE);
//
//             // Quote Balance second:
//             // Expected TEST_QUOTE increase = (500 * 1.0) - (500 * 1.0 * 0.5) = 500 - 250 = 250
//             // expected TEST_QUOTE Balance { total: 9_800 + 250, available: 9_700 + 250 }
//             let expected_TEST_QUOTE = TokenBalance::new("TEST_QUOTE", Balance::new(10_050.0, 9_950.0));
//             assert_eq!(balances[1], expected_TEST_QUOTE);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Trade(trade),
//             ..
//         }) => {
//             let expected = Trade {
//                 id: TradeId(2.to_string()),
//                 order_id: OrderId(4.to_string()),
//                 instrument: Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 side: Side::Sell,
//                 price: 500.0,
//                 quantity: 1.0,
//                 fees: SymbolFees::new("TEST_QUOTE", first_full_fill_fees),
//             };
//             assert_eq!(trade, expected);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // b) Second partial match fill
//     let second_partial_fill_fees = (1000.0 * 0.5) * fees_50_percent();
//
//     // Check AccountEvent Balances for base & quote currencies related to the trade
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balances(balances),
//             ..
//         }) => {
//             // Base & Quote SymbolBalances should be updated
//             assert_eq!(balances.len(), 2);
//
//             // TEST_BASE { total: 9.0, available: 8.5 } 0.5 left in partially filled trade
//
//             // Base Balance first: expected TEST_BASE Balance { total: 9.5 - 0.5, available: 8.5 }
//             let expected_TEST_BASE = TokenBalance::new("TEST_BASE", Balance::new(9.5 - 0.5, 8.5));
//             assert_eq!(balances[0], expected_TEST_BASE);
//
//             // Quote Balance second:
//             // Expected TEST_QUOTE increase = (1000 * 0.5) - (1000 * 0.5 * 0.5) = 500 - 250 = 250
//             // expected TEST_QUOTE Balance { total: 10_050 + 250, available: 9_950 + 250 }
//             let expected_TEST_QUOTE =
//                 TokenBalance::new("TEST_QUOTE", Balance::new(10_050.0 + 250.0, 9_950.0 + 250.0));
//             assert_eq!(balances[1], expected_TEST_QUOTE);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Trade(trade),
//             ..
//         }) => {
//             let expected = Trade {
//                 id: TradeId(3.to_string()),
//                 order_id: OrderId(5.to_string()),
//                 instrument: Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 side: Side::Sell,
//                 price: 1000.0,
//                 quantity: 0.5,
//                 fees: SymbolFees::new("TEST_QUOTE", second_partial_fill_fees),
//             };
//             assert_eq!(trade, expected);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check no more AccountEvents generated
//     match event_account_rx.try_recv() {
//         Err(mpsc::error::TryRecvError::Empty) => {}
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
// }
//
// // 11. Cancel all open orders. Includes a partially filled sell order, and non-filled buy order.
// //     Check AccountEvents for orders cancelled and balances are sent.
// async fn test_11_cancel_all_orders(
//     client: &SandBoxClient,
//     test_6_ids_1: Ids,
//     test_9_ids_2: Ids,
//     event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
// ) {
//     let cancelled = client.cancel_orders_all().await.unwrap();
//
//     let expected_cancelled = vec![
//         order_cancelled(
//             // Bids are cancelled first
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             test_6_ids_1.cid,
//             Side::Buy,
//             test_6_ids_1.id,
//         ),
//         order_cancelled(
//             // Asks are cancelled second
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             test_9_ids_2.cid,
//             Side::Sell,
//             test_9_ids_2.id,
//         ),
//     ];
//
//     assert_eq!(cancelled.len(), 2);
//     assert_eq!(cancelled[0].clone(), expected_cancelled[0]);
//     assert_eq!(cancelled[1].clone(), expected_cancelled[1]);
//
//     // Check AccountEvent Order cancelled for both the bid & ask
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::OrdersCancelled(cancelled),
//             ..
//         }) => {
//             assert_eq!(cancelled.len(), 2);
//             assert_eq!(cancelled[0].clone(), expected_cancelled[0]);
//             assert_eq!(cancelled[1].clone(), expected_cancelled[1]);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check AccountEvent Balances for cancelled bid and ask orders
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balances(balances),
//             ..
//         }) => {
//             // SymbolBalances for Bid order quote, & ask order base should be updated
//             assert_eq!(balances.len(), 2);
//
//             // Bids are cancelled first, so balance is updated first
//             // test_6_order_cid_1, Side::Buy, price=100.0, quantity=1.0
//             // Therefore, TEST_QUOTE Balance { total: 10_300, available: 10_200 + (100 * 1)
//             let expected_TEST_QUOTE =
//                 TokenBalance::new("TEST_QUOTE", Balance::new(10_300.0, 10_200.0 + 100.0));
//             assert_eq!(balances[0], expected_TEST_QUOTE);
//
//             // Asks are cancelled second, so balance is updated first
//             // test_9_order_cid_2, Side::Sell, price=1000.0, quantity=1.0, filled=0.5
//             // Therefore, TEST_BASE Balance { total: 9.0, available: 8.5 + 0.5 }
//             let expected_TEST_BASE = TokenBalance::new("TEST_BASE", Balance::new(9.0, 8.5 + 0.5));
//             assert_eq!(balances[1], expected_TEST_BASE);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check no more AccountEvents generated
//     match event_account_rx.try_recv() {
//         Err(mpsc::error::TryRecvError::Empty) => {}
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
// }
//
// // 12. Fetch open orders (now that we've called cancel_all) and check it is empty
// async fn test_12_fetch_open_orders_and_check_empty(client: &SandBoxClient) {
//     let open_orders = client.fetch_orders_open().await.unwrap();
//     assert!(open_orders.is_empty());
// }
//
// // 13. Fail to open limit buy order with insufficient funds
// async fn test_13_fail_to_open_one_of_two_limits_with_insufficient_funds(
//     client: &SandBoxClient,
//     test_13_ids_1: Ids,
//     test_13_ids_2: Ids,
//     event_account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>,
// ) {
//     let opened_orders = client
//         .open_orders(vec![
//             order_request_limit(
//                 Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 test_13_ids_1.cid,
//                 Side::Buy,
//                 1_000_000_000.0,
//                 1.0,
//             ),
//             order_request_limit(
//                 Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//                 test_13_ids_2.cid,
//                 Side::Sell,
//                 1000.0,
//                 1.0,
//             ),
//         ])
//         .await;
//
//     let expected_order_new_1 = Err(ExecutionError::InsufficientBalance(token::from("TEST_QUOTE")));
//     let expected_order_new_2 = open_order(
//         Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//         test_13_ids_2.cid,
//         test_13_ids_2.id,
//         Side::Sell,
//         1000.0,
//         1.0,
//         0.0,
//     );
//
//     assert_eq!(opened_orders.len(), 2);
//     assert_eq!(opened_orders[0].clone(), expected_order_new_1);
//     assert_eq!(opened_orders[1].clone().unwrap(), expected_order_new_2);
//
//     // Note: First order failed to due TEST_QUOTE InsufficientBalance, so don't expect any AccountEvents
//
//     // Check AccountEvent Balance for second order - quote currency has available balance decrease
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::Balance(TEST_BASE_balance),
//             ..
//         }) => {
//             // Expected TEST_BASE Balance.available = 9.0 - 1.0
//             let expected = TokenBalance::new("TEST_BASE", Balance::new(9.0, 9.0 - 1.0));
//             assert_eq!(TEST_BASE_balance, expected);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check AccountEvent OrdersNew for first order
//     match event_account_rx.try_recv() {
//         Ok(AccountEvent {
//             kind: AccountEventKind::OrdersNew(new_orders),
//             ..
//         }) => {
//             assert_eq!(new_orders.len(), 1);
//             assert_eq!(new_orders[0].clone(), expected_order_new_2);
//         }
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
//
//     // Check no more AccountEvents generated
//     match event_account_rx.try_recv() {
//         Err(mpsc::error::TryRecvError::Empty) => {}
//         other => {
//             panic!(
//                 "try_recv() consumed unexpected Result<AccountEvent>: {:?}",
//                 other
//             );
//         }
//     }
// }
//
// // 14. Fail to cancel limit order with OrderNotFound using incorrect OrderId
// async fn test_14_fail_to_cancel_limit_with_order_not_found(client: &SandBoxClient) {
//     let cid = ClientOrderId(Uuid::new_v4());
//     let cancelled = client
//         .cancel_orders(vec![order_cancel_request(
//             Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual)),
//             cid,
//             Side::Buy,
//             OrderId::from("order will not be found"),
//         )])
//         .await;
//
//     let expected = Err(ExecutionError::OrderNotFound(cid));
//
//     assert_eq!(cancelled.len(), 1);
//     assert_eq!(cancelled[0], expected);
// }
//

