use crate::{
    common::{
        account_positions::{position_id::PositionId, Position},
        balance::Balance,
        instrument::Instrument,
    },
    vault::error::VaultError,
    Exchange,
};
use uuid::Uuid;

pub mod error;
pub mod redis;
mod summariser;

/// 处理 [`Position`] 在持久层的读写操作。
pub trait PositionProcessor
{
    /// 使用 [`PositionId`] 更新或插入一个打开的 [`Position`]。
    fn add_open_position(&mut self, position: Position) -> Result<(), VaultError>;

    /// 使用提供的 [`PositionId`] 获取一个打开的 [`Position`]。
    fn get_open_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError>;

    /// 获取与一个投资组合相关联的所有打开的 [`Position`]。
    fn get_open_positions(&mut self, session_id: Uuid, exchange: Exchange, instrument: Instrument) -> Result<Vec<Position>, VaultError>;

    /// 移除在 [`PositionId`] 位置的 [`Position`]。
    fn remove_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError>;

    /// 将一个已退出的 [`Position`] 附加到投资组合的已退出持仓列表中。
    fn set_exited_position(&mut self, session_id: Uuid, position: Position) -> Result<(), VaultError>;

    /// 获取与 session_id 相关联的所有已退出的 [`Position`]。
    fn get_exited_positions(&mut self, session_id: Uuid) -> Result<Vec<Position>, VaultError>;
}

/// 处理投资组合当前余额在持久层的读写操作。
pub trait BalanceProcessor
{
    /// 使用 session_id 更新或插入投资组合 [`Balance`]。
    fn set_balance(&mut self, session_id: Uuid, balance: Balance) -> Result<(), VaultError>;
    /// 使用提供的 session_id 获取投资组合 [`Balance`]。
    fn get_balance(&mut self, session_id: Uuid) -> Result<Balance, VaultError>;
}

/// 处理投资组合中每个市场的统计数据的读写操作。
pub trait StatisticHandler<Statistic>
{
    fn set_statistics(&mut self, exchange: Exchange, instrument: Instrument, statistic: Statistic) -> Result<(), VaultError>;
    fn get_statistics(&mut self, exchange: Exchange, instrument: Instrument) -> Result<Statistic, VaultError>;
}

/// 用于表示投资组合中所有已退出 [`Position`] 的唯一标识符的字符串类型。
/// 用于将新的已退出 [`Position`] 附加到 [`PositionProcessor`] 的条目中。
pub type ExitedPositionsId = String;

/// 返回给定 session_id 的投资组合的已退出 [`Position`] 的唯一标识符。
pub fn determine_exited_positions_id(session_id: Uuid) -> ExitedPositionsId
{
    format!("positions_exited_{}", session_id)
}
