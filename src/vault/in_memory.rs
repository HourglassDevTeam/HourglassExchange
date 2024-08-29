// use crate::vault::StatisticHandler;
// use crate::vault::BalanceHandler;
// use crate::vault::determine_exited_positions_id;
// use std::collections::HashMap;
// use uuid::Uuid;
// use crate::common::position::{Position, position_id::PositionId};
// use crate::vault::error::VaultError;
// use crate::common::{balance::Balance, instrument::Instrument};
// use crate::Exchange;
// use crate::vault::PositionHandler;
//
// /// 用于 Proof Of Concept 的内存仓库。实现了 [`PositionHandler`]、[`BalanceHandler`] 和 [`StatisticHandler`]。
// /// 用于概念验证投资组合实现，保存当前权益、可用资金、仓位和市场对的统计数据。
// /// **注意：此实现无容错保证，未排除极端情况下会出现性能抖动和OOM等情况，谨慎用于生产环境！**
// #[derive(Debug, Default)]
// pub struct InMemoryVault<Statistic> {
//     open_positions: HashMap<PositionId, Position>,
//     closed_positions: HashMap<String, Vec<Position>>,
//     current_balances: HashMap<Uuid, Balance>,
//     statistics: HashMap<(Exchange, Instrument), Statistic>,
// }
//
// impl<Statistic> PositionHandler for InMemoryVault<Statistic> {
//     fn set_open_position(&mut self, position: Position) -> Result<(), VaultError> {
//         self.open_positions.insert(position.clone(), position);
//         Ok(())
//     }
//
//     fn get_open_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError> {
//         Ok(self.open_positions.get(position_id).cloned())
//     }
//
//     fn get_open_positions(
//         &mut self,
//         instance_id: Uuid,
//         exchange: Exchange,
//         instrument: Instrument
//     ) -> Result<Vec<Position>, VaultError> {
//         Ok(self.open_positions.values()
//             .filter(|position| position.instance_id == instance_id
//                 && position.exchange == exchange
//                 && position.instrument == instrument)
//             .cloned()
//             .collect())
//     }
//
//     fn remove_position(&mut self, position_id: &PositionId) -> Result<Option<Position>, VaultError> {
//         Ok(self.open_positions.remove(position_id))
//     }
//
//     fn set_exited_position(&mut self, instance_id: Uuid, position: Position) -> Result<(), VaultError> {
//         let exited_positions_key = determine_exited_positions_id(instance_id);
//
//         match self.closed_positions.get_mut(&exited_positions_key) {
//             None => {
//                 self.closed_positions.insert(exited_positions_key, vec![position]);
//             }
//             Some(closed_positions) => closed_positions.push(position),
//         }
//         Ok(())
//     }
//
//     fn get_exited_positions(&mut self, instance_id: Uuid) -> Result<Vec<Position>, VaultError> {
//         Ok(self.closed_positions
//             .get(&determine_exited_positions_id(instance_id))
//             .cloned()
//             .unwrap_or_default())
//     }
// }
//
// impl<Statistic> BalanceHandler for InMemoryVault<Statistic> {
//     fn set_balance(&mut self, instance_id: Uuid, balance: Balance) -> Result<(), VaultError> {
//         self.current_balances.insert(instance_id, balance);
//         Ok(())
//     }
//
//     fn get_balance(&mut self, instance_id: Uuid) -> Result<Balance, VaultError> {
//         self.current_balances
//             .get(&instance_id)
//             .copied()
//             .ok_or(VaultError::ExpectedDataNotPresentError)
//     }
// }
//
// impl<Statistic> StatisticHandler<Statistic> for InMemoryVault<Statistic> {
//     fn set_statistics(&mut self, exchange: Exchange, instrument: Instrument, statistic: Statistic) -> Result<(), VaultError> {
//         self.statistics.insert((exchange, instrument), statistic);
//         Ok(())
//     }
//
//     fn get_statistics(&mut self, exchange: Exchange, instrument: Instrument) -> Result<Statistic, VaultError> {
//         self.statistics
//             .get(&(exchange, instrument))
//             .cloned()
//             .ok_or(VaultError::ExpectedDataNotPresentError)
//     }
// }
//
// impl<Statistic> InMemoryVault<Statistic> {
//     /// 构建一个新的 [`InMemoryVault`] 组件。
//     pub fn new() -> Self {
//         Self {
//             open_positions: HashMap::new(),
//             closed_positions: HashMap::new(),
//             current_balances: HashMap::new(),
//             statistics: HashMap::new(),
//         }
//     }
// }
