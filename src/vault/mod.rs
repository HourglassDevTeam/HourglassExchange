use crate::common::{balance::Balance, instrument::Instrument};
use uuid::Uuid;
use crate::{vault::error::VaultError, Exchange};
use crate::common::position::Position;
use crate::common::position::position_id::PositionId;

pub mod error;
pub mod in_memory;
pub mod redis;
mod summariser;

/// 处理 [`Position`] 在持久层的读写操作。
pub trait PositionHandler {
    /// 使用 [`PositionId`] 更新或插入一个打开的 [`Position`]。
    fn set_open_position(&mut self, position: Position) -> Result<(), VaultError>;

    /// 使用提供的 [`PositionId`] 获取一个打开的 [`Position`]。
    fn get_open_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, VaultError>;

    /// 获取与一个投资组合相关联的所有打开的 [`Position`]。
    fn get_open_positions(
        &mut self,
        instance_id: Uuid,
        exchange: Exchange, instrument: Instrument
    ) -> Result<Vec<Position>, VaultError>;

    /// 移除在 [`PositionId`] 位置的 [`Position`]。
    fn remove_position(
        &mut self,
        position_id: &PositionId,
    ) -> Result<Option<Position>, VaultError>;

    /// 将一个已退出的 [`Position`] 附加到投资组合的已退出持仓列表中。
    fn set_exited_position(
        &mut self,
        instance_id: Uuid,
        position: Position,
    ) -> Result<(), VaultError>;

    /// 获取与 instance_id 相关联的所有已退出的 [`Position`]。
    fn get_exited_positions(&mut self, instance_id: Uuid) -> Result<Vec<Position>, VaultError>;
}

/// 处理投资组合当前余额在持久层的读写操作。
pub trait BalanceHandler
{
    /// 使用 instance_id 更新或插入投资组合 [`Balance`]。
    fn set_balance(&mut self, instance_id: Uuid, balance: Balance) -> Result<(), VaultError>;
    /// 使用提供的 instance_id 获取投资组合 [`Balance`]。
    fn get_balance(&mut self, instance_id: Uuid) -> Result<Balance, VaultError>;
}

/// 处理投资组合中每个市场的统计数据的读写操作。
pub trait StatisticHandler<Statistic>
{
    fn set_statistics(&mut self, exchange: Exchange, instrument: Instrument, statistic: Statistic) -> Result<(), VaultError>;
    fn get_statistics(&mut self, exchange: Exchange, instrument: Instrument) -> Result<Statistic, VaultError>;
}

/// 用于表示投资组合中所有已退出 [`Position`] 的唯一标识符的字符串类型。
/// 用于将新的已退出 [`Position`] 附加到 [`PositionHandler`] 的条目中。
pub type ExitedPositionsId = String;

/// 返回给定 instance_id 的投资组合的已退出 [`Position`] 的唯一标识符。
pub fn determine_exited_positions_id(instance_id: Uuid) -> ExitedPositionsId
{
    format!("positions_exited_{}", instance_id)
}
