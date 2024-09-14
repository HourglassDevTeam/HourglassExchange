use crate::{
    common::{
        account_positions::{Position, PositionConfig},
        instrument::Instrument,
    },
    error::ExchangeError,
    sandbox::{config_request::ConfigurationRequest, sandbox_client::ConfigureInstrumentsResults},
};
use async_trait::async_trait;

use crate::{
    common::{
        account_positions::{
            exited_position::PositionExit,
            future::{FuturePosition, FuturePositionConfig},
            leveraged_token::{LeveragedTokenPosition, LeveragedTokenPositionConfig},
            option::OptionPosition,
            perpetual::{PerpetualPosition, PerpetualPositionConfig},
            position_meta::PositionMeta,
            AccountPositions, PositionDirectionMode, PositionMarginMode,
        },
        instrument::kind::InstrumentKind,
        trade::ClientTrade,
        Side,
    },
    sandbox::account::{handlers::position_handler::PositionHandling::CloseCompleteAndReverse, respond, SandboxAccount},
};
use tokio::sync::oneshot::Sender;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PositionHandling
{
    OpenBrandNewPosition,
    ClosePartial,
    CloseComplete,
    CloseCompleteAndReverse
    {
        reverse_size: f64,
    }, // 使用命名字段来说明 Option<f64> 的含义
    UpdateExisting,
}

#[async_trait]
pub trait PositionHandler
{
    async fn preconfigure_position(&mut self, config_request: ConfigurationRequest) -> Result<PositionConfig, ExchangeError>;

    async fn preconfigure_positions(&mut self, config_requests: Vec<ConfigurationRequest>, response_tx: Sender<ConfigureInstrumentsResults>) -> Result<Vec<PositionConfig>, ExchangeError>;

    async fn get_position_long(&self, instrument: &Instrument) -> Result<Option<Position>, ExchangeError>;

    async fn get_position_short(&self, instrument: &Instrument) -> Result<Option<Position>, ExchangeError>;

    async fn get_position_both_ways(&self, instrument: &Instrument) -> Result<(Option<Position>, Option<Position>), ExchangeError>;

    async fn fetch_positions_and_respond(&self, response_tx: Sender<Result<AccountPositions, ExchangeError>>);

    async fn fetch_long_position_and_respond(&self, instrument: &Instrument, response_tx: Sender<Result<Option<Position>, ExchangeError>>);

    async fn fetch_short_position_and_respond(&self, instrument: &Instrument, response_tx: Sender<Result<Option<Position>, ExchangeError>>);

    // async fn check_and_handle_liquidation(&mut self, trade: ClientTrade) -> Result<Option<ClientTrade>, ExchangeError>;
    async fn check_position_direction_conflict(&self, instrument: &Instrument, new_order_side: Side, is_reduce_only: bool) -> Result<(), ExchangeError>;

    async fn create_perpetual_position(&mut self, trade: ClientTrade, handle_type: PositionHandling) -> Result<PerpetualPosition, ExchangeError>;

    async fn create_future_position(&mut self, trade: ClientTrade) -> Result<FuturePosition, ExchangeError>;

    async fn create_option_position(&mut self, trade: ClientTrade) -> Result<OptionPosition, ExchangeError>;

    async fn create_leveraged_token_position(&mut self, trade: ClientTrade) -> Result<LeveragedTokenPosition, ExchangeError>;
    async fn handle_config_inheritance(&self, trade: &ClientTrade) -> Result<PerpetualPositionConfig, ExchangeError>;

    async fn determine_handling_type(&self, trade: ClientTrade) -> Result<PositionHandling, ExchangeError>;
    async fn update_position_from_client_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    /// 在 create_position 过程中确保仓位的杠杆率不超过账户的最大杠杆率。  [TODO] : TO BE CHECKED & APPLIED
    fn enforce_leverage_limits(&self, new_position: &PerpetualPosition) -> Result<(), ExchangeError>;

    async fn remove_position(&self, instrument: Instrument, side: Side) -> Option<Position>;
    async fn remove_perpetual_position(&self, instrument: Instrument, side: Side) -> Option<PerpetualPosition>;
    async fn remove_future_position(&self, instrument: Instrument, side: Side) -> Option<FuturePosition>;
    async fn remove_leveraged_token_position(&self, instrument: Instrument, side: Side) -> Option<LeveragedTokenPosition>;
    async fn remove_option_position(&self, instrument: Instrument, side: Side) -> Option<OptionPosition>;

    /// NOTE this is currently omitted and should be applied appropriately.
    async fn exit_position_and_dump(&self, meta: &PositionMeta, side: Side) -> Result<(), ExchangeError>;

    async fn get_position_long_config(&self, instrument: &Instrument) -> Result<Option<PerpetualPositionConfig>, ExchangeError>;
    async fn get_position_short_config(&self, instrument: &Instrument) -> Result<Option<PerpetualPositionConfig>, ExchangeError>;
    // 更新已有仓位
    async fn update_existing_position(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    // 关闭仓位
    async fn close_position(&mut self, instrument: Instrument, side: Side) -> Result<(), ExchangeError>;
    // 关闭并反向开仓
    async fn close_and_reverse_position(&mut self, trade: ClientTrade, remaining: f64) -> Result<(), ExchangeError>;
    // 部分平仓
    async fn partial_close_position(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    // 更新隔离保证金 /// NOTE this is currently problematic and should be checked very carefully.
    async fn update_isolated_margin(&mut self, position: &mut PerpetualPosition, trade: &ClientTrade);
}

#[async_trait]
impl PositionHandler for SandboxAccount
{
    /// 预先设置控制仓位的字段。
    async fn preconfigure_position(&mut self, config_request: ConfigurationRequest) -> Result<PositionConfig, ExchangeError>
    {
        let side = config_request.side;
        match config_request.instrument.kind {
            | InstrumentKind::Spot => Err(ExchangeError::UnsupportedInstrumentKind),
            | InstrumentKind::Perpetual => {
                let perpetual_config = PerpetualPositionConfig::from(config_request.clone());
                match side {
                    | Side::Buy => {
                        self.positions.perpetual_pos_long_config.write().await.insert(config_request.instrument, perpetual_config.clone());
                    }
                    | Side::Sell => {
                        self.positions.perpetual_pos_short_config.write().await.insert(config_request.instrument, perpetual_config.clone());
                    }
                }
                Ok(PositionConfig::Perpetual(perpetual_config))
            }
            | InstrumentKind::Future => {
                let future_config = FuturePositionConfig::from(config_request.clone());
                match side {
                    | Side::Buy => {
                        self.positions.futures_pos_long_config.write().await.insert(config_request.instrument.clone(), future_config.clone());
                    }
                    | Side::Sell => {
                        self.positions.futures_pos_short_config.write().await.insert(config_request.instrument.clone(), future_config.clone());
                    }
                }
                Ok(PositionConfig::Future(future_config))
            }
            | InstrumentKind::CryptoLeveragedToken => {
                let leveraged_token_config = LeveragedTokenPositionConfig::from(config_request.clone());
                match side {
                    | Side::Buy => {
                        self.positions.margin_pos_long_config.write().await.insert(config_request.instrument.clone(), leveraged_token_config.clone());
                    }
                    | Side::Sell => {
                        self.positions.margin_pos_short_config.write().await.insert(config_request.instrument.clone(), leveraged_token_config.clone());
                    }
                }
                Ok(PositionConfig::LeveragedToken(leveraged_token_config))
            }
            | InstrumentKind::CryptoOption => Err(ExchangeError::UnsupportedInstrumentKind),
            | _ => Err(ExchangeError::UnsupportedInstrumentKind),
        }
    }

    async fn preconfigure_positions(&mut self, config_requests: Vec<ConfigurationRequest>, response_tx: Sender<ConfigureInstrumentsResults>) -> Result<Vec<PositionConfig>, ExchangeError>
    {
        let mut position_configs = Vec::new();
        let mut results = Vec::new();

        for config_request in config_requests {
            match self.preconfigure_position(config_request).await {
                | Ok(config) => {
                    results.push(Ok(config.clone()));
                    position_configs.push(config);
                }
                | Err(e) => {
                    results.push(Err(e));
                }
            }
        }

        response_tx.send(results).unwrap_or_else(|_| {
                                     eprintln!("[UniLinkEx] : Failed to send preconfigure_positions response");
                                 });

        Ok(position_configs)
    }

    /// 获取指定 `Instrument` 的多头仓位
    async fn get_position_long(&self, instrument: &Instrument) -> Result<Option<Position>, ExchangeError>
    {
        let positions = &self.positions;

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExchangeError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument)));
            }
            | InstrumentKind::Perpetual => {
                let perpetual_positions = &positions.perpetual_pos_long;

                // 获取读锁
                let read_lock = perpetual_positions.read().await;

                // 在读锁上调用 `iter()` 遍历 HashMap
                if let Some(position) = read_lock.iter().find(|(_, pos)| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::Perpetual(position.1.clone())));
                }
            }
            | InstrumentKind::Future => {
                todo!()
            }
            | InstrumentKind::CryptoOption => {
                todo!()
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!()
            }
            | InstrumentKind::CommodityOption | InstrumentKind::CommodityFuture => {
                todo!("Commodity positions are not yet implemented");
            }
        }

        Ok(None) // 没有找到对应的仓位
    }

    /// 获取指定 `Instrument` 的空头仓位
    async fn get_position_short(&self, instrument: &Instrument) -> Result<Option<Position>, ExchangeError>
    {
        let positions = &self.positions; // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExchangeError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument)));
            }
            | InstrumentKind::Perpetual => {
                let perpetual_positions = &positions.perpetual_pos_short;

                // 获取读锁
                let read_lock = perpetual_positions.read().await;

                // 通过读锁访问 HashMap
                if let Some((_, position)) = read_lock.iter().find(|(_, pos)| pos.meta.instrument == *instrument) {
                    return Ok(Some(Position::Perpetual(position.clone())));
                }
            }
            | InstrumentKind::Future => {
                todo!()
            }
            | InstrumentKind::CryptoOption => {
                todo!()
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!()
            }
            | InstrumentKind::CommodityOption | InstrumentKind::CommodityFuture => {
                todo!("Commodity positions are not yet implemented");
            }
        }

        Ok(None) // 没有找到对应的仓位
    }

    async fn get_position_both_ways(&self, instrument: &Instrument) -> Result<(Option<Position>, Option<Position>), ExchangeError>
    {
        let positions = &self.positions; // 获取锁

        match instrument.kind {
            | InstrumentKind::Spot => Err(ExchangeError::InvalidInstrument(format!("Spots do not support positions: {:?}", instrument))),
            | InstrumentKind::Perpetual => {
                // 获取读锁
                let long_pos_lock = positions.perpetual_pos_long.read().await;
                let short_pos_lock = positions.perpetual_pos_short.read().await;

                // 通过读锁访问 HashMap
                let long_pos = long_pos_lock.get(instrument).map(|pos| Position::Perpetual(pos.clone()));
                let short_pos = short_pos_lock.get(instrument).map(|pos| Position::Perpetual(pos.clone()));

                Ok((long_pos, short_pos))
            }
            | InstrumentKind::Future => {
                todo!()
            }
            | InstrumentKind::CryptoOption => {
                todo!()
            }
            | InstrumentKind::CryptoLeveragedToken => {
                todo!()
            }
            | InstrumentKind::CommodityOption | InstrumentKind::CommodityFuture => {
                todo!("Commodity positions are not yet implemented");
            }
        }
    }

    async fn fetch_positions_and_respond(&self, response_tx: Sender<Result<AccountPositions, ExchangeError>>)
    {
        let positions = self.positions.clone();
        respond(response_tx, Ok(positions));
    }

    async fn fetch_long_position_and_respond(&self, instrument: &Instrument, response_tx: Sender<Result<Option<Position>, ExchangeError>>)
    {
        let position = self.get_position_long(instrument).await.unwrap();
        respond(response_tx, Ok(position));
    }

    async fn fetch_short_position_and_respond(&self, instrument: &Instrument, response_tx: Sender<Result<Option<Position>, ExchangeError>>)
    {
        let position = self.get_position_short(instrument).await.unwrap();
        respond(response_tx, Ok(position));
    }

    //  async fn check_and_handle_liquidation(&mut self, trade: ClientTrade) -> Result<Option<ClientTrade>, ExchangeError> {
    //     let instrument = &trade.instrument;
    //
    //     // 获取多头和空头仓位
    //     let (long_position, short_position) = self.get_position_both_ways(instrument).await?;
    //
    //     // 用于存储平仓的 `ClientTrade`
    //     let mut liquidation_trade: Option<ClientTrade> = None;
    //
    //     // 检查多头仓位是否爆仓
    //     if let Some(Position::Perpetual(long_pos)) = long_position {
    //         if let Some(liquidation_price) = long_pos.liquidation_price {
    //             if trade.price <= liquidation_price {
    //                 // 多头仓位爆仓，生成平仓的 `ClientTrade`
    //                 liquidation_trade = Some(ClientTrade {
    //                     exchange: trade.exchange,
    //                     timestamp: trade.timestamp,  // 使用当前时间戳
    //                     trade_id: ClientTradeId::new(), // 生成新的 trade_id
    //                     order_id: long_pos.meta.position_id.into(),  // 使用仓位的ID作为order_id
    //                     cid: trade.cid.clone(),
    //                     instrument: instrument.clone(),
    //                     side: Side::Sell,  // 平仓方向为卖出
    //                     price: trade.price,  // 平仓时的价格
    //                     size: long_pos.meta.current_size,  // 全部平仓
    //                     fees: 0.0,  // 可以根据实际情况计算费用
    //                 });
    //
    //                 // 处理平仓
    //                 self.close_position(instrument.clone(), Side::Buy).await?;
    //             }
    //         }
    //     }
    //
    //     // 检查空头仓位是否爆仓
    //     if let Some(Position::Perpetual(short_pos)) = short_position {
    //         if let Some(liquidation_price) = short_pos.liquidation_price {
    //             if trade.price >= liquidation_price {
    //                 // 空头仓位爆仓，生成平仓的 `ClientTrade`
    //                 liquidation_trade = Some(ClientTrade {
    //                     exchange: trade.exchange,
    //                     timestamp: trade.timestamp,  // 使用当前时间戳
    //                     trade_id: ClientTradeId::new(), // 生成新的 trade_id
    //                     order_id: short_pos.meta.position_id.into(),  // 使用仓位的ID作为order_id
    //                     cid: trade.cid.clone(),
    //                     instrument: instrument.clone(),
    //                     side: Side::Buy,  // 平仓方向为买入
    //                     price: trade.price,  // 平仓时的价格
    //                     size: short_pos.meta.current_size,  // 全部平仓
    //                     fees: 0.0,  // 可以根据实际情况计算费用
    //                 });
    //
    //                 // 处理平仓
    //                 self.close_position(instrument.clone(), Side::Sell).await?;
    //             }
    //         }
    //     }
    //
    //     // 返回生成的平仓交易（如果有的话）
    //     Ok(liquidation_trade)
    // }

    /// 检查给定的 `new_order_side` 是否与现有仓位方向冲突，并根据 `is_reduce_only` 标志做出相应处理。
    ///
    /// ### 参数:
    /// - `instrument`: 订单涉及的金融工具。
    /// - `new_order_side`: 新订单的方向（买/卖）。
    /// - `is_reduce_only`: 如果为 `true`，则订单仅用于减少现有仓位。
    ///
    /// ### 返回:
    /// - 如果没有方向冲突，返回 `Ok(())`。
    /// - 如果存在与订单方向相反的仓位，并且 `is_reduce_only` 为 `false`，返回 `Err(ExchangeError::InvalidDirection)`。
    ///
    /// ### 特殊情况:
    /// - 对于 `Spot`、`CommodityOption`、`CommodityFuture`、`CryptoOption` 和 `CryptoLeveragedToken` 类型的 `InstrumentKind`，
    ///   当前不支持仓位冲突检查，返回 `Err(ExchangeError::NotImplemented)`。
    /// - 如果 `is_reduce_only` 为 `true`，允许方向冲突。
    ///
    /// ### 错误:
    /// - `ExchangeError::InvalidDirection`: 当存在方向冲突时。
    /// - `ExchangeError::NotImplemented`: 当 `InstrumentKind` 不支持检查时。
    async fn check_position_direction_conflict(&self, instrument: &Instrument, new_order_side: Side, is_reduce_only: bool /* 添加reduce_only标志 */) -> Result<(), ExchangeError>
    {
        let positions_lock = &self.positions;

        match instrument.kind {
            | InstrumentKind::Spot => {
                return Err(ExchangeError::NotImplemented("Spot account_positions conflict check not implemented".into()));
            }
            | InstrumentKind::CommodityOption | InstrumentKind::CommodityFuture => {
                return Err(ExchangeError::NotImplemented("Commodity account_positions conflict check not implemented".into()));
            }
            | InstrumentKind::Perpetual => {
                // 获取读锁
                let long_pos_read_lock = positions_lock.perpetual_pos_long.read().await;
                let short_pos_read_lock = positions_lock.perpetual_pos_short.read().await;

                // 在持有读锁的情况下调用 `iter()` 遍历 HashMap
                let long_position_exists = long_pos_read_lock.iter().any(|(_, pos)| pos.meta.instrument == *instrument);
                let short_position_exists = short_pos_read_lock.iter().any(|(_, pos)| pos.meta.instrument == *instrument);

                // 如果订单是 reduce only，允许方向冲突
                if is_reduce_only {
                    return Ok(());
                }

                // 如果存在与订单方向相反的仓位，返回错误
                if (new_order_side == Side::Buy && short_position_exists) || (new_order_side == Side::Sell && long_position_exists) {
                    return Err(ExchangeError::InvalidDirection);
                }
            }
            | InstrumentKind::Future => {
                // 获取读锁
                let long_pos_read_lock = positions_lock.futures_pos_long.read().await;
                let short_pos_read_lock = positions_lock.futures_pos_short.read().await;

                let long_position_exists = long_pos_read_lock.iter().any(|(_, pos)| pos.meta.instrument == *instrument);
                let short_position_exists = short_pos_read_lock.iter().any(|(_, pos)| pos.meta.instrument == *instrument);

                // 如果订单是 reduce only，允许方向冲突
                if is_reduce_only {
                    return Ok(());
                }

                // 如果存在与订单方向相反的仓位，返回错误
                if (new_order_side == Side::Buy && short_position_exists) || (new_order_side == Side::Sell && long_position_exists) {
                    return Err(ExchangeError::InvalidDirection);
                }
            }
            | InstrumentKind::CryptoOption | InstrumentKind::CryptoLeveragedToken => {
                return Err(ExchangeError::NotImplemented("Position conflict check for this instrument kind not implemented".into()));
            }
        }

        Ok(())
    }

    /// 更新 PerpetualPosition 的方法
    /// 这里传入了一个 `PositionMarginMode`， 意味着初始化的
    async fn create_perpetual_position(&mut self, trade: ClientTrade, handle_type: PositionHandling) -> Result<PerpetualPosition, ExchangeError>
    {
        // 获取该 instrument 的配置
        let perpetual_config = self.handle_config_inheritance(&trade).await?;

        // 创建 PositionMeta 和新的 PerpetualPosition
        let meta = match handle_type {
            | PositionHandling::OpenBrandNewPosition => PositionMeta::create_from_trade(&trade),
            | CloseCompleteAndReverse { reverse_size } => PositionMeta::create_from_trade_with_remaining(&trade, reverse_size),
            | _ => return Err(ExchangeError::SandBox("Not supposed to create any position here.".into())),
        };

        // 计算 isolated_margin（隔离保证金），仅在 Isolated 模式下
        let isolated_margin = if let PositionMarginMode::Isolated { .. } = perpetual_config.pos_margin_mode {
            // 根据 trade.price, leverage 和 size 计算 isolated_margin
            Some(trade.price * perpetual_config.leverage * trade.size)
        }
        else {
            None
        };

        // 创建新的 PerpetualPosition，包括 isolated_margin
        let new_position = PerpetualPosition { meta,
                                               pos_config: perpetual_config.clone(),
                                               isolated_margin,         // 直接赋值
                                               liquidation_price: None  /* 后续可计算 */ };

        // 根据买卖方向将仓位插入相应的仓位列表
        match trade.side {
            | Side::Buy => self.positions.perpetual_pos_long.write().await.insert(trade.instrument, new_position.clone()),
            | Side::Sell => self.positions.perpetual_pos_short.write().await.insert(trade.instrument, new_position.clone()),
        };

        Ok(new_position)
    }

    #[allow(dead_code)]
    /// 更新 FuturePosition 的方法（占位符）
    async fn create_future_position(&mut self, trade: ClientTrade) -> Result<FuturePosition, ExchangeError>
    {
        // 获取预存储的配置，首先获取写锁
        let mut pos_config_lock = match trade.side {
            | Side::Buy => self.positions.futures_pos_long_config.write().await,
            | Side::Sell => self.positions.futures_pos_short_config.write().await,
        };

        // 获取该 instrument 的配置，如果没有找到则返回错误
        let future_config = pos_config_lock.get_mut(&trade.instrument)
                                           .ok_or_else(|| ExchangeError::SandBox("No pre-configuration found for the given instrument.".to_string()))?;

        let meta = PositionMeta::create_from_trade(&trade);

        // 计算 isolated_margin（隔离保证金），仅在 Isolated 模式下
        let isolated_margin = if let PositionMarginMode::Isolated = future_config.pos_margin_mode {
            Some(trade.price * future_config.leverage * trade.size)
        }
        else {
            None
        };
        let new_position = FuturePosition { meta,
                                            pos_config: future_config.clone(),
                                            liquidation_price: 0.0,
                                            isolated_margin,
                                            funding_fee: 0.0 /* TODO: To Be Checked */ };

        // 插入仓位到正确的仓位映射中
        match trade.side {
            | Side::Buy => {
                self.positions.futures_pos_long.write().await.insert(trade.instrument.clone(), new_position.clone());
            }
            | Side::Sell => {
                self.positions.futures_pos_short.write().await.insert(trade.instrument.clone(), new_position.clone());
            }
        }

        Ok(new_position)
    }

    #[allow(dead_code)]
    /// 更新 OptionPosition 的方法（占位符）
    async fn create_option_position(&mut self, _trade: ClientTrade) -> Result<OptionPosition, ExchangeError>
    {
        todo!("[UniLinkEx] : Updating Option positions is not yet implemented")
    }

    #[allow(dead_code)]
    /// 更新 LeveragedTokenPosition 的方法（占位符）
    async fn create_leveraged_token_position(&mut self, _trade: ClientTrade) -> Result<LeveragedTokenPosition, ExchangeError>
    {
        todo!("[UniLinkEx] : Updating Leveraged Token positions is not yet implemented")
    }

    /// 当且仅当 `PositionDirectionMode` 是 `Net` 的时候, 允许在处理新的trade的时候继承反向仓位的configuration.并且返回.
    async fn handle_config_inheritance(&self, trade: &ClientTrade) -> Result<PerpetualPositionConfig, ExchangeError>
    {
        // 尝试获取同向仓位配置
        let same_side_config = match trade.side {
            | Side::Buy => self.get_position_long_config(&trade.instrument).await?,
            | Side::Sell => self.get_position_short_config(&trade.instrument).await?,
        };

        // 检查是否找到了同向配置
        if let Some(config) = same_side_config {
            return Ok(config);
        }

        // 如果没有找到同向配置，尝试获取反向仓位配置
        let opposite_side_config = match trade.side {
            | Side::Buy => self.get_position_short_config(&trade.instrument).await?,
            | Side::Sell => self.get_position_long_config(&trade.instrument).await?,
        };

        // 检查是否找到了反向配置
        if let Some(config) = opposite_side_config {
            // 检查配置的模式是否允许继承
            return if config.position_direction_mode == PositionDirectionMode::Net {
                Ok(config)
            }
            else {
                Err(ExchangeError::ConfigInheritanceNotAllowed)
            }
        }

        // 如果两个方向的配置都不存在，报错
        Err(ExchangeError::ConfigMissing)
    }

    async fn determine_handling_type(&self, trade: ClientTrade) -> Result<PositionHandling, ExchangeError>
    {
        // 获取仓位配置
        let config = self.handle_config_inheritance(&trade).await?;

        // 检查是否存在既有同向仓位
        let has_existing_long_position = self.get_position_long(&trade.instrument).await?.is_some();
        let has_existing_short_position = self.get_position_short(&trade.instrument).await?.is_some();

        // 获取当前仓位大小
        let current_size = if let Some(position) = self.get_position_long(&trade.instrument).await? {
            match position {
                | Position::Perpetual(perp_position) => perp_position.meta.current_size,
                | Position::LeveragedToken(lt_position) => lt_position.meta.current_size,
                | Position::Future(future_position) => future_position.meta.current_size,
                | Position::Option(option_position) => option_position.meta.current_size,
            }
        }
        else if let Some(position) = self.get_position_short(&trade.instrument).await? {
            match position {
                | Position::Perpetual(perp_position) => perp_position.meta.current_size,
                | Position::LeveragedToken(lt_position) => lt_position.meta.current_size,
                | Position::Future(future_position) => future_position.meta.current_size,
                | Position::Option(option_position) => option_position.meta.current_size,
            }
        }
        else {
            0.0 // 如果没有现有仓位，大小为0
        };

        // 确定仓位方向
        let position_side = if has_existing_long_position {
            Side::Buy
        }
        else if has_existing_short_position {
            Side::Sell
        }
        else {
            trade.side
        };

        // 根据配置的 position_direction_mode 进行分类讨论
        match config.position_direction_mode {
            | PositionDirectionMode::Net => {
                // 在 Net 模式下，仓位方向与交易方向相同，或者需要关闭反向仓位
                if position_side != trade.side {
                    // 如果方向不同，可能是反向操作
                    if current_size == trade.size {
                        Ok(PositionHandling::CloseComplete)
                    }
                    else if current_size < trade.size {
                        Ok(CloseCompleteAndReverse { reverse_size: trade.size - current_size })
                    }
                    else {
                        Ok(PositionHandling::ClosePartial)
                    }
                }
                else {
                    // 方向相同，更新或开启新仓位
                    if has_existing_long_position || has_existing_short_position {
                        Ok(PositionHandling::UpdateExisting)
                    }
                    else {
                        Ok(PositionHandling::OpenBrandNewPosition)
                    }
                }
            }
            | PositionDirectionMode::LongShort => {
                // 在 LongShort 模式下，确保新交易的方向与现有仓位的方向一致
                if (trade.side == Side::Buy && has_existing_long_position) || (trade.side == Side::Sell && has_existing_short_position) {
                    // 如果新交易的方向与现有仓位方向一致，更新现有仓位
                    Ok(PositionHandling::UpdateExisting)
                }
                else {
                    // 如果方向不一致或没有现有仓位，开启新仓位
                    Ok(PositionHandling::OpenBrandNewPosition)
                }
            }
        }
    }

    async fn update_position_from_client_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        // 通过调用 determine_handling_type 确定该交易的处理方式
        let handling_type = self.determine_handling_type(trade.clone()).await?;

        // 根据处理类型调用不同的处理逻辑
        match handling_type {
            | PositionHandling::OpenBrandNewPosition => {
                println!("executing PositionHandling::OpenBrandNewPosition");
                self.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await?;
            }
            | PositionHandling::UpdateExisting => {
                println!("executing PositionHandling::UpdateExisting");
                self.update_existing_position(trade).await?;
            }
            | PositionHandling::CloseComplete => {
                println!("executing PositionHandling::CloseComplete");
                self.close_position(trade.instrument.clone(), trade.side).await?;
            }
            | PositionHandling::CloseCompleteAndReverse { reverse_size } => {
                println!("executing PositionHandling::CloseCompleteAndReverse");
                self.close_and_reverse_position(trade, reverse_size).await?;
            }
            | PositionHandling::ClosePartial => {
                println!("executing PositionHandling::ClosePartial");
                self.partial_close_position(trade).await?;
            }
        }

        Ok(())
    }

    /// 在 create_position 过程中确保仓位的杠杆率不超过账户的最大杠杆率。  [TODO] : TO BE CHECKED & APPLIED
    fn enforce_leverage_limits(&self, new_position: &PerpetualPosition) -> Result<(), ExchangeError>
    {
        if new_position.pos_config.leverage > self.config.global_leverage_rate {
            Err(ExchangeError::InvalidLeverage(format!("Leverage is beyond configured rate: {}", new_position.pos_config.leverage)))
        }
        else {
            Ok(())
        }
    }

    async fn remove_position(&self, instrument: Instrument, side: Side) -> Option<Position>
    {
        match instrument.kind {
            | InstrumentKind::Perpetual => self.remove_perpetual_position(instrument, side).await.map(Position::Perpetual),
            | InstrumentKind::Future => self.remove_future_position(instrument, side).await.map(Position::Future),
            | InstrumentKind::CryptoLeveragedToken => self.remove_leveraged_token_position(instrument, side).await.map(Position::LeveragedToken),
            | InstrumentKind::CryptoOption => self.remove_option_position(instrument, side).await.map(Position::Option),
            | _ => None,
        }
    }

    async fn remove_perpetual_position(&self, instrument: Instrument, side: Side) -> Option<PerpetualPosition>
    {
        match side {
            | Side::Buy => {
                let mut long_positions = self.positions.perpetual_pos_long.write().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.perpetual_pos_short.write().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_future_position(&self, instrument: Instrument, side: Side) -> Option<FuturePosition>
    {
        match side {
            | Side::Buy => {
                let mut long_positions = self.positions.futures_pos_long.write().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.futures_pos_short.write().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_leveraged_token_position(&self, instrument: Instrument, side: Side) -> Option<LeveragedTokenPosition>
    {
        match side {
            | Side::Buy => {
                let mut long_positions = self.positions.margin_pos_long.write().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.margin_pos_short.write().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_option_position(&self, instrument: Instrument, side: Side) -> Option<OptionPosition>
    {
        match side {
            | Side::Buy => {
                let mut long_call_positions = self.positions.option_pos_long_call.write().await;
                let mut long_put_positions = self.positions.option_pos_long_put.write().await;

                long_call_positions.remove(&instrument).or_else(|| long_put_positions.remove(&instrument))
            }
            | Side::Sell => {
                let mut short_call_positions = self.positions.option_pos_short_call.write().await;
                let mut short_put_positions = self.positions.option_pos_short_put.write().await;

                short_call_positions.remove(&instrument).or_else(|| short_put_positions.remove(&instrument))
            }
        }
    }

    async fn exit_position_and_dump(&self, meta: &PositionMeta, side: Side) -> Result<(), ExchangeError>
    {
        // Convert `PositionMeta` into `PositionExit`
        let exited = PositionExit::from_position_meta(meta);

        // Insert into the appropriate exited positions collection
        match (meta.instrument.kind, side) {
            | (InstrumentKind::Perpetual, Side::Buy) => {
                self.exited_positions.insert_perpetual_pos_long(exited).await;
            }
            | (InstrumentKind::Perpetual, Side::Sell) => {
                self.exited_positions.insert_perpetual_pos_short(exited).await;
            }
            // You can add handling for other position types here
            | _ => return Err(ExchangeError::UnsupportedInstrumentKind),
        }

        Ok(())
    }

    async fn get_position_long_config(&self, instrument: &Instrument) -> Result<Option<PerpetualPositionConfig>, ExchangeError>
    {
        // 获取多头仓位配置的锁
        let long_configs = self.positions.perpetual_pos_long_config.read().await;

        // 查找并返回相应的多头仓位配置
        long_configs.get(instrument).cloned().map_or(Ok(None), |config| Ok(Some(config)))
    }

    async fn get_position_short_config(&self, instrument: &Instrument) -> Result<Option<PerpetualPositionConfig>, ExchangeError>
    {
        // 获取空头仓位配置的锁
        let short_configs = self.positions.perpetual_pos_short_config.read().await;

        // 查找并返回相应的空头仓位配置
        short_configs.get(instrument).cloned().map_or(Ok(None), |config| Ok(Some(config)))
    }

    // 更新已有仓位
    async fn update_existing_position(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        match trade.side {
            | Side::Buy => {
                let position = {
                    let mut long_positions = self.positions.perpetual_pos_long.write().await;
                    long_positions.get_mut(&trade.instrument).map(|p| p.clone())
                    // Clone the position out of the lock
                };

                if let Some(mut position) = position {
                    position.meta.update_from_trade(&trade);
                    self.update_isolated_margin(&mut position, &trade).await;
                    // Re-lock to update the position in the map
                    let mut long_positions = self.positions.perpetual_pos_long.write().await;
                    long_positions.insert(trade.instrument.clone(), position);
                }
            }
            | Side::Sell => {
                let position = {
                    let mut short_positions = self.positions.perpetual_pos_short.write().await;
                    short_positions.get_mut(&trade.instrument).map(|p| p.clone())
                    // Clone the position out of the lock
                };

                if let Some(mut position) = position {
                    position.meta.update_from_trade(&trade);
                    self.update_isolated_margin(&mut position, &trade).await;
                    // Re-lock to update the position in the map
                    let mut short_positions = self.positions.perpetual_pos_short.write().await;
                    short_positions.insert(trade.instrument.clone(), position);
                }
            }
        }
        Ok(())
    }

    // 关闭仓位
    async fn close_position(&mut self, instrument: Instrument, side: Side) -> Result<(), ExchangeError>
    {
        match side {
            | Side::Buy => {
                // 使用 `ok_or` 将 `Option` 转换为 `Result`
                self.remove_position(instrument, Side::Sell).await.ok_or(ExchangeError::AttemptToRemoveNonExistingPosition)?;
            }
            | Side::Sell => {
                self.remove_position(instrument, Side::Buy).await.ok_or(ExchangeError::AttemptToRemoveNonExistingPosition)?;
            }
        }
        Ok(())
    }

    // 关闭并反向开仓
    async fn close_and_reverse_position(&mut self, trade: ClientTrade, remaining: f64) -> Result<(), ExchangeError>
    {
        self.close_position(trade.instrument.clone(), trade.side).await?;
        // Ignore the returned `PerpetualPosition`
        let _ = self.create_perpetual_position(trade.clone(), CloseCompleteAndReverse { reverse_size: remaining }).await?;
        Ok(())
    }

    async fn partial_close_position(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        match trade.side {
            | Side::Sell => {
                println!("Buy side partial_close_position");
                let position = {
                    let mut long_positions = self.positions.perpetual_pos_long.write().await;
                    long_positions.get_mut(&trade.instrument).map(|p| p.clone())
                    // Clone the position out of the lock
                };

                if let Some(mut position) = position {
                    // 减少仓位大小
                    self.update_isolated_margin(&mut position, &trade).await;
                    println!("before position update : currently the size is {:#?}", position.meta.current_size);
                    position.meta.update_from_trade(&trade);
                    println!("after position update : currently the size is {:#?}", position.meta.current_size);
                    // Re-lock to update the position in the map
                    let mut long_positions = self.positions.perpetual_pos_long.write().await;
                    long_positions.insert(trade.instrument.clone(), position);
                }
            }
            | Side::Buy => {
                println!("Sell side partial_close_position");
                let position = {
                    let mut short_positions = self.positions.perpetual_pos_short.write().await;
                    short_positions.get_mut(&trade.instrument).map(|p| p.clone())
                    // Clone the position out of the lock
                };

                if let Some(mut position) = position {
                    // 减少仓位大小
                    self.update_isolated_margin(&mut position, &trade).await;
                    println!("before position update : currently the size is {:#?}", position.meta.current_size);
                    position.meta.update_from_trade(&trade);
                    println!("after position update : currently the size is {:#?}", position.meta.current_size);
                    println!("after position update : currently position is {:#?}", position);
                    // Re-lock to update the position in the map
                    let mut short_positions = self.positions.perpetual_pos_short.write().await;
                    short_positions.insert(trade.instrument.clone(), position);
                    println!("successfully inserted among short positions");
                }
            }
        }
        Ok(())
    }

    // 更新隔离保证金
    async fn update_isolated_margin(&mut self, position: &mut PerpetualPosition, trade: &ClientTrade)
    {
        if let PositionMarginMode::Isolated = position.pos_config.pos_margin_mode {
            if let Some(ref mut margin) = position.isolated_margin {
                *margin += trade.price * trade.size * position.pos_config.leverage;
            }
            else {
                position.isolated_margin = Some(trade.price * trade.size * position.pos_config.leverage);
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{order::identification::OrderId, token::Token, trade::ClientTradeId},
        test_utils::create_test_account,
        Exchange,
    };

    #[tokio::test]
    async fn test_create_new_long_position()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(1),
                                  order_id: OrderId(1),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 16999.0,
                                  size: 1.0,
                                  fees: 0.1 };

        // 插入预先配置的多头仓位 PerpetualPositionConfig
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::LongShort };

        // 将 PerpetualPositionConfig 插入到多头配置中
        account.positions.perpetual_pos_long_config.write().await.insert(instrument.clone(), preconfig);

        // 执行管理仓位逻辑
        let result = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        assert!(result.is_ok());

        // 检查多头仓位是否成功创建
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        assert!(positions.contains_key(&trade.instrument)); // 检查 HashMap 中是否有该键
        let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 1.0); // 检查仓位大小
    }

    #[tokio::test]
    async fn test_create_new_short_position()
    {
        let mut account = create_test_account().await;

        // 创建一个 trade
        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(2),
                                  order_id: OrderId(2),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Sell,
                                  price: 100.0,
                                  size: 5.0,
                                  fees: 0.05 };

        // 使用与 `trade` 相同的 `instrument` 进行插入配置
        let instrument = trade.instrument.clone();

        // 预先配置 PerpetualPositionConfig
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 确保 leverage 设置正确
                                                  position_direction_mode: PositionDirectionMode::LongShort };

        // 将配置插入 `perpetual_pos_short_config`
        account.positions.perpetual_pos_short_config.write().await.insert(instrument.clone(), preconfig);

        // 执行创建新空头仓位的逻辑
        let result = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;
        assert!(result.is_ok());

        // 检查空头仓位是否成功创建
        let positions = account.positions.perpetual_pos_short.read().await; // 获取读锁
        assert!(positions.contains_key(&trade.instrument)); // 检查 HashMap 中是否有该键
        let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 5.0); // 检查仓位大小
    }

    #[tokio::test]
    async fn test_update_existing_long_position_cross_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(3),
                                  order_id: OrderId(3),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::LongShort };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade { exchange: Exchange::SandBox,
                                             timestamp: 1690000100,
                                             trade_id: ClientTradeId(4),
                                             order_id: OrderId(4),
                                             cid: None,
                                             instrument: Instrument { base: Token("BTC".to_string()),
                                                                      quote: Token("USDT".to_string()),
                                                                      kind: InstrumentKind::Perpetual },
                                             side: Side::Buy,
                                             price: 100.0,
                                             size: 5.0,
                                             fees: 0.05 };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(3),
                                  order_id: OrderId(3),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade { exchange: Exchange::SandBox,
                                             timestamp: 1690000100,
                                             trade_id: ClientTradeId(4),
                                             order_id: OrderId(4),
                                             cid: None,
                                             instrument: Instrument { base: Token("BTC".to_string()),
                                                                      quote: Token("USDT".to_string()),
                                                                      kind: InstrumentKind::Perpetual },
                                             side: Side::Buy,
                                             price: 100.0,
                                             size: 5.0,
                                             fees: 0.05 };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_isolated_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(3),
                                  order_id: OrderId(3),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade { exchange: Exchange::SandBox,
                                             timestamp: 1690000100,
                                             trade_id: ClientTradeId(4),
                                             order_id: OrderId(4),
                                             cid: None,
                                             instrument: Instrument { base: Token("BTC".to_string()),
                                                                      quote: Token("USDT".to_string()),
                                                                      kind: InstrumentKind::Perpetual },
                                             side: Side::Buy,
                                             price: 100.0,
                                             size: 5.0,
                                             fees: 0.05 };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_isolated_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(3),
                                  order_id: OrderId(3),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::LongShort };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade { exchange: Exchange::SandBox,
                                             timestamp: 1690000100,
                                             trade_id: ClientTradeId(4),
                                             order_id: OrderId(4),
                                             cid: None,
                                             instrument: Instrument { base: Token("BTC".to_string()),
                                                                      quote: Token("USDT".to_string()),
                                                                      kind: InstrumentKind::Perpetual },
                                             side: Side::Buy,
                                             price: 100.0,
                                             size: 5.0,
                                             fees: 0.05 };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_close_long_position_partially_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let result = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;
        println!("the result is {:#?}", result);

        // 部分平仓
        let closing_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000200,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 5.0,
                                          fees: 0.05 };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&closing_trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_short_position_partially_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Sell,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;
        // 部分平仓
        let closing_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000200,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Buy,
                                          price: 100.0,
                                          size: 5.0,
                                          fees: 0.05 };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        let positions = account.positions.perpetual_pos_short.read().await; // 获取读锁
        let pos = positions.get(&closing_trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_long_position_partially_isolated_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 部分平仓
        let closing_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000200,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 5.0,
                                          fees: 0.05 };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // 检查仓位是否部分平仓
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_long_position_completely()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 完全平仓
        let closing_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000000,
                                          trade_id: ClientTradeId(5),
                                          order_id: OrderId(5),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 10.0,
                                          fees: 0.1 };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();

        // 检查仓位是否已被完全移除
        let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        assert!(!positions.contains_key(&trade.instrument));
    }

    #[tokio::test]
    async fn test_reverse_position_after_closing_long_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 反向平仓并开立新的空头仓位
        let reverse_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000100,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 15.0, // 卖出 15.0 超过当前的多头仓位
                                          fees: 0.15 };

        account.update_position_from_client_trade(reverse_trade.clone()).await.unwrap();

        // 检查多头仓位是否已被完全移除
        let long_positions = account.positions.perpetual_pos_long.read().await;
        assert!(!long_positions.contains_key(&trade.instrument));

        // 检查新的空头仓位是否已创建，并且大小正确（剩余 5.0）
        let short_positions = account.positions.perpetual_pos_short.read().await;
        assert!(short_positions.contains_key(&trade.instrument));
        let short_position = short_positions.get(&trade.instrument).unwrap();
        assert_eq!(short_position.meta.current_size, 5.0); // 剩余仓位应该是 5.0
        assert_eq!(short_position.meta.side, Side::Sell); // 检查持仓方向是否为 Sell
    }

    #[tokio::test]
    async fn test_reverse_position_after_closing_long_isolated_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Isolated,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::Net };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 反向平仓并开立新的空头仓位
        let reverse_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000100,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 15.0, // 卖出 15.0 超过当前的多头仓位
                                          fees: 0.15 };

        let _ = account.update_position_from_client_trade(reverse_trade.clone()).await;

        // 检查多头仓位是否已被完全移除
        let long_positions = account.positions.perpetual_pos_long.read().await;
        assert!(!long_positions.contains_key(&trade.instrument));

        // 检查新的空头仓位是否已创建，并且大小正确（剩余 5.0）
        let short_positions = account.positions.perpetual_pos_short.read().await;
        assert!(short_positions.contains_key(&trade.instrument));
        let short_position = short_positions.get(&trade.instrument).unwrap();
        assert_eq!(short_position.meta.current_size, 5.0); // 剩余仓位应该是 5.0
        assert_eq!(short_position.meta.side, Side::Sell); // 检查持仓方向是否为 Sell
    }

    #[tokio::test]
    async fn test_reverse_position_after_closing_long_cross_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("BTC".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Perpetual },
                                  side: Side::Buy,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                  leverage: 1.0, // 设置合理的杠杆
                                                  position_direction_mode: PositionDirectionMode::LongShort };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone(), PositionHandling::OpenBrandNewPosition).await;

        // 反向平仓并开立新的空头仓位
        let reverse_trade = ClientTrade { exchange: Exchange::SandBox,
                                          timestamp: 1690000100,
                                          trade_id: ClientTradeId(6),
                                          order_id: OrderId(6),
                                          cid: None,
                                          instrument: Instrument { base: Token("BTC".to_string()),
                                                                   quote: Token("USDT".to_string()),
                                                                   kind: InstrumentKind::Perpetual },
                                          side: Side::Sell,
                                          price: 100.0,
                                          size: 15.0, // 卖出 15.0 超过当前的多头仓位
                                          fees: 0.15 };

        let result = account.update_position_from_client_trade(reverse_trade.clone()).await;
        assert!(matches!(result, Err(ExchangeError::ConfigInheritanceNotAllowed)), "Unexpected error: {:?}", result);
    }

    #[tokio::test]
    async fn test_unsupported_instrument_kind()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade { exchange: Exchange::SandBox,
                                  timestamp: 1690000000,
                                  trade_id: ClientTradeId(5),
                                  order_id: OrderId(5),
                                  cid: None,
                                  instrument: Instrument { base: Token("RRR".to_string()),
                                                           quote: Token("USDT".to_string()),
                                                           kind: InstrumentKind::Spot /* Spot Position is either not developed or not supported. */ },
                                  side: Side::Sell,
                                  price: 100.0,
                                  size: 10.0,
                                  fees: 0.1 };

        // 执行管理仓位逻辑，应该返回错误
        let result = account.update_position_from_client_trade(trade.clone()).await;
        assert!(result.is_err());
    }
}
