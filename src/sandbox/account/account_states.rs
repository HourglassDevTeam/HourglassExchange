//     #[tokio::test]
//         async fn test_check_position_direction_conflict()
//         {
//             let account_state = create_test_account_state().await;
//             let instrument = create_test_instrument(InstrumentKind::Perpetual);
//
//             // 情况1：没有冲突的情况下调用
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument, Side::Buy).await;
//             assert!(result.is_ok());
//
//             // 情况2：模拟存在冲突的Perpetual仓位，注意这里 `side` 是 `Sell`
//             account_state.lock().await.positions.perpetual_pos = vec![create_test_perpetual_position(instrument.clone()), ];
//
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument, Side::Sell).await;
//             assert!(result.is_err());
//             assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);
//
//             // 情况3：模拟不存在冲突的Future仓位
//             let instrument_future = create_test_instrument(InstrumentKind::Future);
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument_future, Side::Buy).await;
//             assert!(result.is_ok());
//
//             // 情况4：模拟存在冲突的Future仓位，注意这里 `side` 是 `Sell`
//             account_state.lock().await.positions.futures_pos = vec![create_test_future_position_with_side(instrument_future.clone(), Side::Sell), ];
//
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument_future, Side::Buy).await;
//             assert!(result.is_err());
//             assert_eq!(result.unwrap_err(), ExecutionError::InvalidDirection);
//
//             // 情况5：其他 InstrumentKind 还没有实现，因此我们只需要检查它们是否返回未实现的错误
//             let instrument_spot = create_test_instrument(InstrumentKind::Spot);
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument_spot, Side::Buy).await;
//             assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));
//
//             let instrument_commodity_future = create_test_instrument(InstrumentKind::CommodityFuture);
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument_commodity_future, Side::Buy).await;
//             assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));
//
//             let instrument_commodity_option = create_test_instrument(InstrumentKind::CommodityOption);
//             let result = account_state.lock().await.check_position_direction_conflict(&instrument_commodity_option, Side::Buy).await;
//             assert!(matches!(result, Err(ExecutionError::NotImplemented(_))));
//         }
//     }
