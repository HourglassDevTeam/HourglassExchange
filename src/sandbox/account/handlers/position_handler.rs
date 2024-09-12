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
    sandbox::account::{respond, Account},
};
use tokio::sync::oneshot::Sender;

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

    async fn check_position_direction_conflict(&self, instrument: &Instrument, new_order_side: Side, is_reduce_only: bool) -> Result<(), ExchangeError>;

    async fn create_perpetual_position(&mut self, trade: ClientTrade) -> Result<PerpetualPosition, ExchangeError>;

    async fn create_future_position(&mut self, trade: ClientTrade) -> Result<FuturePosition, ExchangeError>;

    async fn create_option_position(&mut self, trade: ClientTrade) -> Result<OptionPosition, ExchangeError>;

    async fn create_leveraged_token_position(&mut self, trade: ClientTrade) -> Result<LeveragedTokenPosition, ExchangeError>;

    async fn update_position_from_client_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    /// NOTE isolated margin mode not supported yet, but handling logic for isolated margin added.
    async fn update_position_long_short_mode(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    /// 注意 liquidation price 更新逻辑未实现.
    async fn update_position_net_mode(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;
    /// 在 create_position 过程中确保仓位的杠杆率不超过账户的最大杠杆率。  [TODO] : TO BE CHECKED & APPLIED
    fn enforce_leverage_limits(&self, new_position: &PerpetualPosition) -> Result<(), ExchangeError>;

    async fn remove_position(&self, instrument: Instrument, side: Side) -> Option<Position>;
    async fn remove_perpetual_position(&self, instrument: Instrument, side: Side) -> Option<PerpetualPosition>;
    async fn remove_future_position(&self, instrument: Instrument, side: Side) -> Option<FuturePosition>;
    async fn remove_leveraged_token_position(&self, instrument: Instrument, side: Side) -> Option<LeveragedTokenPosition>;
    async fn remove_option_position(&self, instrument: Instrument, side: Side) -> Option<OptionPosition>;

    async fn exit_position_and_dump(&self, meta: &PositionMeta, side: Side) -> Result<(), ExchangeError>;
}

#[async_trait]
impl PositionHandler for Account
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
                let read_lock = perpetual_positions.lock().await;

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
                let read_lock = perpetual_positions.lock().await;

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
                let long_pos_lock = positions.perpetual_pos_long.lock().await;
                let short_pos_lock = positions.perpetual_pos_short.lock().await;

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
                let long_pos_read_lock = positions_lock.perpetual_pos_long.lock().await;
                let short_pos_read_lock = positions_lock.perpetual_pos_short.lock().await;

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
                let long_pos_read_lock = positions_lock.futures_pos_long.lock().await;
                let short_pos_read_lock = positions_lock.futures_pos_short.lock().await;

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
    /// 注意 此处 `PositionMarginMode` 中初始化为`none`的 `isolated_margin` 被直接传输进来. 接下来`isolated_margin应该在此处被计算出来的.

    async fn create_perpetual_position(&mut self, trade: ClientTrade) -> Result<PerpetualPosition, ExchangeError>
    {
        // 获取预存储的配置，首先获取写锁
        let mut pos_config_lock = match trade.side {
            | Side::Buy => self.positions.perpetual_pos_long_config.write().await,
            | Side::Sell => self.positions.perpetual_pos_short_config.write().await,
        };

        // 获取该 instrument 的配置，如果没有找到则返回错误
        let perpetual_config = pos_config_lock.get_mut(&trade.instrument)
                                              .ok_or_else(|| ExchangeError::SandBox("No pre-configuration found for the given instrument.".to_string()))?;

        // 创建 PositionMeta 和新的 PerpetualPosition
        let meta = PositionMeta::create_from_trade(&trade);

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
            | Side::Buy => self.positions.perpetual_pos_long.lock().await.insert(trade.instrument, new_position.clone()),
            | Side::Sell => self.positions.perpetual_pos_short.lock().await.insert(trade.instrument, new_position.clone()),
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
                self.positions.futures_pos_long.lock().await.insert(trade.instrument.clone(), new_position.clone());
            }
            | Side::Sell => {
                self.positions.futures_pos_short.lock().await.insert(trade.instrument.clone(), new_position.clone());
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

    /// 根据[PositionDirectionMode]分流
    async fn update_position_from_client_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        // println!("[UniLinkEx] : Received a new trade: {:?}", trade);

        match trade.instrument.kind {
            | InstrumentKind::Perpetual => {
                match self.config.global_position_direction_mode {
                    | PositionDirectionMode::Net => {
                        // Net Mode 逻辑
                        self.update_position_net_mode(trade).await?;
                    }
                    | PositionDirectionMode::LongShort => {
                        // LongShort Mode 逻辑
                        self.update_position_long_short_mode(trade).await?;
                    }
                }
            }
            | _ => {
                println!("[UniLinkEx] : Unsupported yet or illegal instrument kind.");
                return Err(ExchangeError::UnsupportedInstrumentKind);
            }
        }

        Ok(())
    }

    /// NOTE isolated margin mode not supported yet, but handling logic for isolated margin added.
    async fn update_position_long_short_mode(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        match trade.side {
            // 买入处理逻辑
            | Side::Buy => {
                // 获取写锁更新或创建多头仓位
                let mut long_positions = self.positions.perpetual_pos_long.lock().await;

                if let Some(position) = long_positions.get_mut(&trade.instrument) {
                    // 更新已有多头仓位
                    println!("[UniLinkEx] : Updating existing long position...");
                    position.meta.update_from_trade(&trade);

                    // 处理隔离保证金模式
                    if let PositionMarginMode::Isolated = position.pos_config.pos_margin_mode {
                        // 初始化或更新隔离保证金
                        if position.isolated_margin.is_none() {
                            position.isolated_margin = Some(trade.price * trade.size * position.pos_config.leverage);
                        }
                        else if let Some(ref mut margin) = position.isolated_margin {
                            *margin += trade.price * trade.size * position.pos_config.leverage;
                        }
                    }
                }
                else {
                    // 释放写锁后创建新的多头仓位
                    drop(long_positions);

                    // 创建新的多头仓位
                    let new_position = self.create_perpetual_position(trade.clone()).await?;

                    // 获取写锁插入新的仓位
                    let mut long_positions = self.positions.perpetual_pos_long.lock().await;
                    long_positions.insert(trade.instrument.clone(), new_position);
                }
            }

            // 卖出处理逻辑
            | Side::Sell => {
                // 获取写锁更新或创建空头仓位
                let mut short_positions = self.positions.perpetual_pos_short.lock().await;

                if let Some(position) = short_positions.get_mut(&trade.instrument) {
                    // 更新已有空头仓位
                    println!("[UniLinkEx] : Updating existing short position...");
                    position.meta.update_from_trade(&trade);

                    // 处理隔离保证金模式
                    if let PositionMarginMode::Isolated = position.pos_config.pos_margin_mode {
                        // 初始化或更新隔离保证金
                        if position.isolated_margin.is_none() {
                            position.isolated_margin = Some(trade.price * trade.size * position.pos_config.leverage);
                        }
                        else if let Some(ref mut margin) = position.isolated_margin {
                            *margin += trade.price * trade.size * position.pos_config.leverage;
                        }
                    }
                }
                else {
                    // 释放写锁后创建新的空头仓位
                    drop(short_positions);

                    // 创建新的空头仓位
                    let new_position = self.create_perpetual_position(trade.clone()).await?;

                    // 获取写锁插入新的仓位
                    let mut short_positions = self.positions.perpetual_pos_short.lock().await;
                    short_positions.insert(trade.instrument.clone(), new_position);
                }
            }
        }

        Ok(())
    }

    /// 注意 liquidation price 更新逻辑未实现.
    async fn update_position_net_mode(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>
    {
        match trade.instrument.kind {
            | InstrumentKind::Perpetual => {
                match trade.side {
                    | Side::Buy => {
                        // 定义相关变量
                        let perfect_exit;
                        let exit_and_reverse;
                        let remaining_quantity;

                        // 获取空头仓位的锁
                        if let Some(Position::Perpetual(mut short_position)) = self.remove_position(trade.instrument.clone(), Side::Sell).await {
                            // 如果存在空头仓位，判断是否需要移除或反向开仓
                            perfect_exit = short_position.meta.current_size == trade.size;
                            exit_and_reverse = short_position.meta.current_size < trade.size;
                            remaining_quantity = trade.size - short_position.meta.current_size;

                            // 完全平仓
                            if perfect_exit {
                                short_position.isolated_margin = Some(0.0); // 暂时清零
                                short_position.meta.update_realised_pnl(trade.price);
                                let _ = self.exit_position_and_dump(&short_position.meta, Side::Sell).await;
                            }
                            // 反向开仓
                            else if exit_and_reverse {
                                let position_margin_mode = short_position.pos_config.pos_margin_mode.clone();
                                short_position.meta.update_realised_pnl(trade.price);
                                short_position.isolated_margin = Some(0.0); // 暂时清零
                                let _ = self.exit_position_and_dump(&short_position.meta, Side::Sell).await;

                                // 获取多头仓位的锁并插入新的仓位
                                let mut long_positions = self.positions.perpetual_pos_long.lock().await;
                                let new_position = PerpetualPosition { meta: PositionMeta::create_from_trade_with_remaining(&trade, remaining_quantity),
                                                                       pos_config: PerpetualPositionConfig { pos_margin_mode: position_margin_mode.clone(),
                                                                                                             leverage: short_position.pos_config.leverage,
                                                                                                             position_direction_mode: self.config.global_position_direction_mode.clone() },
                                                                       isolated_margin: Some(trade.price * remaining_quantity * short_position.pos_config.leverage),
                                                                       liquidation_price: Some(0.0) };
                                long_positions.insert(trade.instrument.clone(), new_position);
                            }
                            // 更新隔离保证金
                            else {
                                if let PositionMarginMode::Isolated = short_position.pos_config.pos_margin_mode {
                                    if short_position.isolated_margin.is_none() {
                                        short_position.isolated_margin = Some(trade.price * trade.size * short_position.pos_config.leverage);
                                    }
                                    else if let Some(ref mut margin) = short_position.isolated_margin {
                                        *margin += trade.price * remaining_quantity * short_position.pos_config.leverage;
                                    }
                                }
                                let mut short_positions = self.positions.perpetual_pos_short.lock().await;

                                // 将更新后的 short_position 放回 HashMap
                                short_positions.insert(trade.instrument.clone(), short_position);
                            }
                        }
                        else {
                            // 如果没有空头仓位，检查多头仓位，如果有，把增量保证金加入多头仓位。
                            let mut long_positions = self.positions.perpetual_pos_long.lock().await;
                            if let Some(long_position) = long_positions.get_mut(&trade.instrument) {
                                if let PositionMarginMode::Isolated = long_position.pos_config.pos_margin_mode {
                                    if long_position.isolated_margin.is_none() {
                                        long_position.isolated_margin = Some(trade.price * trade.size * long_position.pos_config.leverage);
                                    }
                                    else if let Some(ref mut margin) = long_position.isolated_margin {
                                        *margin += trade.price * trade.size * long_position.pos_config.leverage;
                                    }
                                }
                                long_position.meta.update_from_trade(&trade);
                            }
                            else {
                                // 释放 `long_positions` 锁
                                drop(long_positions);

                                // 创建新的多头仓位
                                self.create_perpetual_position(trade.clone()).await?;
                            }
                        }
                    }
                    | Side::Sell => {
                        // 定义相关变量
                        let perfect_exit;
                        let exit_and_reverse;
                        let remaining_quantity;

                        // 获取多头仓位的锁

                        if let Some(Position::Perpetual(mut long_position)) = self.remove_position(trade.instrument.clone(), Side::Buy).await {
                            perfect_exit = long_position.meta.current_size == trade.size;
                            exit_and_reverse = long_position.meta.current_size < trade.size;
                            remaining_quantity = trade.size - long_position.meta.current_size;

                            // 完全平仓
                            if perfect_exit {
                                long_position.isolated_margin = Some(0.0); // 暂时清零
                                long_position.meta.update_realised_pnl(trade.price);
                                let _ = self.exit_position_and_dump(&long_position.meta, Side::Buy).await;
                                // 注意：long_position 已经被移除了，因此不需要再次调用 remove
                            }
                            // 反向开仓
                            else if exit_and_reverse {
                                let position_margin_mode = long_position.pos_config.pos_margin_mode.clone();
                                long_position.meta.update_realised_pnl(trade.price);
                                long_position.isolated_margin = Some(0.0); // 暂时清零
                                let _ = self.exit_position_and_dump(&long_position.meta, Side::Buy).await;
                                // 获取空头仓位的锁并插入新的仓位
                                let mut short_positions = self.positions.perpetual_pos_short.lock().await;
                                let new_position = PerpetualPosition { meta: PositionMeta::create_from_trade_with_remaining(&trade, remaining_quantity),
                                                                       pos_config: PerpetualPositionConfig { pos_margin_mode: position_margin_mode.clone(),
                                                                                                             leverage: long_position.pos_config.leverage,
                                                                                                             position_direction_mode: self.config.global_position_direction_mode.clone() },
                                                                       isolated_margin: Some(trade.price * remaining_quantity * long_position.pos_config.leverage),
                                                                       liquidation_price: Some(0.0) };
                                short_positions.insert(trade.instrument.clone(), new_position);
                            }
                            // 更新隔离保证金
                            else {
                                if let PositionMarginMode::Isolated = long_position.pos_config.pos_margin_mode {
                                    if long_position.isolated_margin.is_none() {
                                        long_position.isolated_margin = Some(trade.price * trade.size * long_position.pos_config.leverage);
                                    }
                                    else if let Some(ref mut margin) = long_position.isolated_margin {
                                        *margin += trade.price * remaining_quantity * long_position.pos_config.leverage;
                                    }
                                }
                                let mut long_positions = self.positions.perpetual_pos_long.lock().await;
                                // 将更新后的 long_position 放回 HashMap
                                long_positions.insert(trade.instrument.clone(), long_position);
                            }
                        }
                        else {
                            // 如果没有多头仓位，检查空头仓位，如果有，把增量保证金加入空头仓位
                            let mut short_positions = self.positions.perpetual_pos_short.lock().await;
                            if let Some(short_position) = short_positions.get_mut(&trade.instrument) {
                                if let PositionMarginMode::Isolated = short_position.pos_config.pos_margin_mode {
                                    if short_position.isolated_margin.is_none() {
                                        short_position.isolated_margin = Some(trade.price * trade.size * short_position.pos_config.leverage);
                                    }
                                    else if let Some(ref mut margin) = short_position.isolated_margin {
                                        *margin += trade.price * trade.size * short_position.pos_config.leverage;
                                    }
                                }
                                short_position.meta.update_from_trade(&trade);
                            }
                            else {
                                drop(short_positions);
                                // 创建新的空头仓位
                                self.create_perpetual_position(trade.clone()).await?;
                            }
                        }
                    }
                }
            }

            | InstrumentKind::Future => {
                println!("[UniLinkEx] : Futures trading is not yet supported.");
                return Err(ExchangeError::UnsupportedInstrumentKind);
            }

            | InstrumentKind::Spot => {
                println!("[UniLinkEx] : Spot trading is not yet supported.");
                return Err(ExchangeError::UnsupportedInstrumentKind);
            }

            | _ => {
                println!("[UniLinkEx] : Unsupported instrument kind.");
                return Err(ExchangeError::UnsupportedInstrumentKind);
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
                let mut long_positions = self.positions.perpetual_pos_long.lock().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.perpetual_pos_short.lock().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_future_position(&self, instrument: Instrument, side: Side) -> Option<FuturePosition>
    {
        match side {
            | Side::Buy => {
                let mut long_positions = self.positions.futures_pos_long.lock().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.futures_pos_short.lock().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_leveraged_token_position(&self, instrument: Instrument, side: Side) -> Option<LeveragedTokenPosition>
    {
        match side {
            | Side::Buy => {
                let mut long_positions = self.positions.margin_pos_long.lock().await;
                long_positions.remove(&instrument)
            }
            | Side::Sell => {
                let mut short_positions = self.positions.margin_pos_short.lock().await;
                short_positions.remove(&instrument)
            }
        }
    }

    async fn remove_option_position(&self, instrument: Instrument, side: Side) -> Option<OptionPosition>
    {
        match side {
            | Side::Buy => {
                let mut long_call_positions = self.positions.option_pos_long_call.lock().await;
                let mut long_put_positions = self.positions.option_pos_long_put.lock().await;

                long_call_positions.remove(&instrument).or_else(|| long_put_positions.remove(&instrument))
            }
            | Side::Sell => {
                let mut short_call_positions = self.positions.option_pos_short_call.lock().await;
                let mut short_put_positions = self.positions.option_pos_short_put.lock().await;

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
}




#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::token::Token;
    use crate::{common::{
        order::identification::OrderId,
        trade::ClientTradeId,
    }, test_utils::create_test_account, Exchange};

    #[tokio::test]
    async fn test_create_new_long_position()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(1),
            order_id: OrderId(1),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 16999.0,
            size: 1.0,
            fees: 0.1
        };

        // 插入预先配置的多头仓位 PerpetualPositionConfig
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::LongShort
        };

        // 将 PerpetualPositionConfig 插入到多头配置中
        account.positions.perpetual_pos_long_config.write().await.insert(instrument.clone(), preconfig);

        // 执行管理仓位逻辑
        let result = account.create_perpetual_position(trade.clone()).await;
        assert!(result.is_ok());

        // 检查多头仓位是否成功创建
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        println!("positions:{:?}", positions); // 打印多头仓位
        assert!(positions.contains_key(&trade.instrument)); // 检查 HashMap 中是否有该键
        let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 1.0); // 检查仓位大小
    }

    #[tokio::test]
    async fn test_create_new_short_position()
    {
        let mut account = create_test_account().await;

        // 创建一个 trade
        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(2),
            order_id: OrderId(2),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        // 使用与 `trade` 相同的 `instrument` 进行插入配置
        let instrument = trade.instrument.clone();

        // 预先配置 PerpetualPositionConfig
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 确保 leverage 设置正确
            position_direction_mode: PositionDirectionMode::LongShort
        };

        // 将配置插入 `perpetual_pos_short_config`
        account.positions.perpetual_pos_short_config.write().await.insert(instrument.clone(), preconfig);

        // 执行创建新空头仓位的逻辑
        let result = account.create_perpetual_position(trade.clone()).await;
        println!("result: {:?}", result); // 打印结果
        assert!(result.is_ok());

        // 检查空头仓位是否成功创建
        let positions = account.positions.perpetual_pos_short.lock().await; // 获取读锁
        assert!(positions.contains_key(&trade.instrument)); // 检查 HashMap 中是否有该键
        let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        assert_eq!(pos.meta.current_size, 5.0); // 检查仓位大小
    }

    #[tokio::test]
    async fn test_update_existing_long_position_cross_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(3),
            order_id: OrderId(3),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Cross,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::LongShort
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000100,
            trade_id: ClientTradeId(4),
            order_id: OrderId(4),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(3),
            order_id: OrderId(3),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Cross,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000100,
            trade_id: ClientTradeId(4),
            order_id: OrderId(4),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_isolated_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(3),
            order_id: OrderId(3),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000100,
            trade_id: ClientTradeId(4),
            order_id: OrderId(4),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }

    #[tokio::test]
    async fn test_update_existing_long_position_isolated_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(3),
            order_id: OrderId(3),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::LongShort
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个初始的多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 再次买入增加仓位
        let additional_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000100,
            trade_id: ClientTradeId(4),
            order_id: OrderId(4),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        // 更新现有仓位
        account.update_position_from_client_trade(additional_trade.clone()).await.unwrap();

        // 检查仓位是否正确更新
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        let pos = positions.get(&trade.instrument).unwrap(); // 获取仓位
        assert_eq!(pos.meta.current_size, 15.0); // 原来的10加上新的5
    }
    #[tokio::test]
    async fn test_close_long_position_partially_cross_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Cross,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;
        // 部分平仓
        let closing_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000200,
            trade_id: ClientTradeId(6),
            order_id: OrderId(6),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        // let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        // let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        // assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_long_position_partially_cross_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Cross,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::LongShort
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;
        // 部分平仓
        let closing_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000200,
            trade_id: ClientTradeId(6),
            order_id: OrderId(6),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        // let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        // let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        // assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }
    #[tokio::test]
    async fn test_close_long_position_partially_isolated_net()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;
        // 部分平仓
        let closing_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000200,
            trade_id: ClientTradeId(6),
            order_id: OrderId(6),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        // let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        // let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        // assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_long_position_partially_isolated_longshort()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::LongShort
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);

        // 创建一个多头仓位
        let _ = account.create_perpetual_position(trade.clone()).await;
        // 部分平仓
        let closing_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000200,
            trade_id: ClientTradeId(6),
            order_id: OrderId(6),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 5.0,
            fees: 0.05
        };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();
        // // 检查仓位是否部分平仓
        // let positions = account.positions.perpetual_pos_long.read().await; // 获取读锁
        // let pos = positions.get(&trade.instrument).unwrap(); // 获取对应的仓位
        // assert_eq!(pos.meta.current_size, 5.0); // 剩余仓位为5
    }

    #[tokio::test]
    async fn test_close_long_position_completely()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Isolated,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 完全平仓
        let closing_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        account.update_position_from_client_trade(closing_trade.clone()).await.unwrap();

        // 检查仓位是否已被完全移除
        let positions = account.positions.perpetual_pos_long.lock().await; // 获取读锁
        assert!(!positions.contains_key(&trade.instrument));
    }

    #[tokio::test]
    async fn test_reverse_position_after_closing_long_cross()
    {
        let mut account = create_test_account().await;

        let trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000000,
            trade_id: ClientTradeId(5),
            order_id: OrderId(5),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Buy,
            price: 100.0,
            size: 10.0,
            fees: 0.1
        };

        // 插入多头仓位配置
        let instrument = trade.instrument.clone();
        let preconfig = PerpetualPositionConfig {
            pos_margin_mode: PositionMarginMode::Cross,
            leverage: 1.0, // 设置合理的杠杆
            position_direction_mode: PositionDirectionMode::Net
        };
        account.positions.perpetual_pos_long_config.write().await.insert(instrument, preconfig);
        let _ = account.create_perpetual_position(trade.clone()).await;

        // 反向平仓并开立新的空头仓位
        let reverse_trade = ClientTrade {
            exchange: Exchange::SandBox,
            timestamp: 1690000100,
            trade_id: ClientTradeId(6),
            order_id: OrderId(6),
            cid: None,
            instrument: Instrument {
                base: Token("BTC".to_string()),
                quote: Token("USDT".to_string()),
                kind: InstrumentKind::Perpetual
            },
            side: Side::Sell,
            price: 100.0,
            size: 15.0, // 卖出 15.0 超过当前的多头仓位
            fees: 0.15
        };

        account.update_position_from_client_trade(reverse_trade.clone()).await.unwrap();

        // 检查多头仓位是否已被完全移除
        let long_positions = account.positions.perpetual_pos_long.lock().await;
        assert!(!long_positions.contains_key(&trade.instrument));

        // 检查新的空头仓位是否已创建，并且大小正确（剩余 5.0）
        let short_positions = account.positions.perpetual_pos_short.lock().await;
        assert!(short_positions.contains_key(&trade.instrument));
        let short_position = short_positions.get(&trade.instrument).unwrap();
        assert_eq!(short_position.meta.current_size, 5.0); // 剩余仓位应该是 5.0
        assert_eq!(short_position.meta.side, Side::Sell); // 检查持仓方向是否为 Sell
    }

    #[tokio::test]
    async fn test_reverse_position_after_closing_long_isolated()
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
        let _ = account.create_perpetual_position(trade.clone()).await;

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
        let long_positions = account.positions.perpetual_pos_long.lock().await;
        assert!(!long_positions.contains_key(&trade.instrument));

        // 检查新的空头仓位是否已创建，并且大小正确（剩余 5.0）
        let short_positions = account.positions.perpetual_pos_short.lock().await;
        assert!(short_positions.contains_key(&trade.instrument));
        let short_position = short_positions.get(&trade.instrument).unwrap();
        assert_eq!(short_position.meta.current_size, 5.0); // 剩余仓位应该是 5.0
        assert_eq!(short_position.meta.side, Side::Sell); // 检查持仓方向是否为 Sell
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
        println!("result: {:?}", result);
        assert!(result.is_err());
    }
}