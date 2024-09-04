use crate::{
    common::{
        balance::Balance,
        instrument::Instrument,
        position::{position_id::PositionId, Position},
    },
    vault::{determine_exited_positions_id, error::VaultError, BalanceHandler, PositionHandler, StatisticHandler},
    Exchange,
};
use std::collections::HashMap;
use uuid::Uuid;

/// 用于初步概念验证的内存仓库。实现了 [`PositionHandler`]、[`BalanceHandler`] 和 [`StatisticHandler`]。
/// 用于概念验证投资组合实现，保存当前权益、可用资金、仓位和市场对的统计数据。
/// 注意：此实现无容错保证，未排除极端情况下会出现性能抖动和OOM等情况，谨慎用于生产环境！
/// 注意：此处的数据结构要重新设计。以和[`Account`]模块对齐。

#[derive(Debug, Default)]
pub struct InMemoryVault<Statistic>
{
    open_positions: HashMap<PositionId, Position>,
    closed_positions: HashMap<String, Vec<Position>>,
    current_balances: HashMap<Uuid, Balance>,
    statistics: HashMap<(Exchange, Instrument), Statistic>,
}

impl<Statistic> PositionHandler for InMemoryVault<Statistic>
{
    fn add_open_position(&mut self, position: Position) -> Result<(), VaultError>
    {
        let position_id = match &position {
            | Position::Perpetual(pos) => pos.meta.position_id,
            | Position::LeveragedToken(pos) => pos.meta.position_id,
            | Position::Future(pos) => pos.meta.position_id,
            | Position::Option(pos) => pos.meta.position_id,
        };
        self.open_positions.insert(position_id, position);
        Ok(())
    }

    fn get_open_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError>
    {
        Ok(self.open_positions.get(position_id).cloned())
    }

    fn get_open_positions(&mut self, _session_id: Uuid, exchange: Exchange, instrument: Instrument) -> Result<Vec<Position>, VaultError>
    {
        Ok(self.open_positions
               .values()
               .filter(|position| match position {
                   | Position::Perpetual(pos) => pos.meta.exchange == exchange && pos.meta.instrument == instrument,
                   | Position::LeveragedToken(pos) => pos.meta.exchange == exchange && pos.meta.instrument == instrument,
                   | Position::Future(pos) => pos.meta.exchange == exchange && pos.meta.instrument == instrument,
                   | Position::Option(pos) => pos.meta.exchange == exchange && pos.meta.instrument == instrument,
               })
               .cloned()
               .collect())
    }

    fn remove_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError>
    {
        Ok(self.open_positions.remove(position_id))
    }

    fn set_exited_position(&mut self, session_id: Uuid, position: Position) -> Result<(), VaultError>
    {
        let exited_positions_key = determine_exited_positions_id(session_id);

        match self.closed_positions.get_mut(&exited_positions_key) {
            | None => {
                self.closed_positions.insert(exited_positions_key, vec![position]);
            }
            | Some(closed_positions) => closed_positions.push(position),
        }
        Ok(())
    }

    fn get_exited_positions(&mut self, session_id: Uuid) -> Result<Vec<Position>, VaultError>
    {
        Ok(self.closed_positions.get(&determine_exited_positions_id(session_id)).cloned().unwrap_or_default())
    }
}

impl<Statistic> BalanceHandler for InMemoryVault<Statistic>
{
    fn set_balance(&mut self, session_id: Uuid, balance: Balance) -> Result<(), VaultError>
    {
        self.current_balances.insert(session_id, balance);
        Ok(())
    }

    fn get_balance(&mut self, session_id: Uuid) -> Result<Balance, VaultError>
    {
        self.current_balances.get(&session_id).copied().ok_or(VaultError::ExpectedDataNotPresentError)
    }
}

impl<Statistic: std::clone::Clone> StatisticHandler<Statistic> for InMemoryVault<Statistic>
{
    fn set_statistics(&mut self, exchange: Exchange, instrument: Instrument, statistic: Statistic) -> Result<(), VaultError>
    {
        self.statistics.insert((exchange, instrument), statistic);
        Ok(())
    }

    fn get_statistics(&mut self, exchange: Exchange, instrument: Instrument) -> Result<Statistic, VaultError>
    {
        self.statistics.get(&(exchange, instrument)).cloned().ok_or(VaultError::ExpectedDataNotPresentError)
    }
}

impl<Statistic> InMemoryVault<Statistic>
{
    /// 构建一个新的 [`InMemoryVault`] 组件。
    pub fn new() -> Self
    {
        Self { open_positions: HashMap::new(),
               closed_positions: HashMap::new(),
               current_balances: HashMap::new(),
               statistics: HashMap::new() }
    }
}
