use crate::common_infrastructure::Side;
/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
/// CONSIDER: can these positions coexist, if so enums might not be ideal.
use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{
        balance::{Balance, TokenBalance},
        friction::{Fees, PerpetualFees},
        instrument::{kind::InstrumentKind, Instrument},
        position::{
            future::FuturePosition,
            leveraged_token::LeveragedTokenPosition,
            option::OptionPosition,
            perpetual::{PerpetualPosition, PerpetualPositionBuilder, PerpetualPositionConfig},
            position_meta::PositionMetaBuilder,
        },
    },
    error::ExecutionError,
    sandbox::account::account_config::AccountConfig,
    ExchangeVariant,
};
use crate::common_infrastructure::trade::ClientTrade;

pub(crate) mod future;
pub(crate) mod leveraged_token;
pub(crate) mod option;
pub mod perpetual;
pub(crate) mod position_meta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountPositions
{
    pub margin_pos: Option<Vec<LeveragedTokenPosition>>, // NOTE useless in backtest
    pub perpetual_pos: Option<Vec<PerpetualPosition>>,
    pub futures_pos: Option<Vec<FuturePosition>>,
    pub option_pos: Option<Vec<OptionPosition>>,
}

impl AccountPositions
{
    /// 创建一个新的 AccountPositions 实例
    pub fn init() -> Self
    {
        Self { margin_pos: None,
               perpetual_pos: None,
               futures_pos: None,
               option_pos: None }
    }

    pub async fn build_new_perpetual_position(
        &self,
        config: &AccountConfig,
        trade: &ClientTrade,  // 使用 ClientTrade 作为输入参数
        pos_margin_mode: PositionMarginMode,
        position_mode: PositionDirectionMode,
        exchange_ts: i64,
    ) -> Result<PerpetualPosition, ExecutionError> {
        let open_fee_rate = config.get_maker_fee_rate(&trade.instrument.kind)?;
        // 根据 Instrument 和 Side 动态生成 position_id
        let position_id = format!("{}_{}", trade.instrument, if trade.side == Side::Buy { "Long" } else { "Short" });
        let position_meta = PositionMetaBuilder::new()
            .position_id(position_id) // NOTE 使用适当的ID生成策略。 {"Instrument"} + {"Long"||"Short"}
            .enter_ts(exchange_ts)
            .update_ts(exchange_ts)
            .exit_balance(TokenBalance { // NOTE 怎么搞
                token: trade.instrument.base.clone(),
                balance: Balance {
                    current_price: trade.price,
                    total: 0.0,
                    available: 0.0,
                },
            })
            .account_exchange_ts(exchange_ts) // NOTE this may well be Redundant
            .exchange(ExchangeVariant::SandBox)
            .instrument(trade.instrument.clone())
            .side(trade.side)
            .current_size(trade.size)
            .current_fees_total(Fees::Perpetual(PerpetualFees {
                maker_rate: open_fee_rate,
                taker_rate: open_fee_rate, // 假设平仓费率与开仓费率相同
                funding_rate: 0.0,         // NOTE 应该在外面预设
            }))
            .current_avg_price_gross(trade.price)
            .current_symbol_price(trade.price)
            .current_avg_price(trade.price)
            .unrealised_pnl(0.0) // all good
            .realised_pnl(0.0) // all good
            .build()
            .map_err(|err| ExecutionError::SandBox(format!("Failed to build position meta: {}", err)))?;

        let pos_config = PerpetualPositionConfig {
            pos_margin_mode,
            leverage: config.account_leverage_rate,  // 从配置中获取杠杆
            position_mode,
        };

        let new_position = PerpetualPositionBuilder::new()
            .meta(position_meta)
            .pos_config(pos_config)
            .liquidation_price(0.0) // NOTE 初始设定为 0，稍后可以根据需要更新
            .margin(0.0) // NOTE 初始设定为 0，稍后可以根据需要更新
            .funding_fee(0.0) // NOTE Redundant?
            .build()
            .ok_or_else(|| ExecutionError::SandBox("Failed to build new position".to_string()))?;

        Ok(new_position)
    }

    /// 更新或添加新的仓位
    pub fn update_position(&mut self, new_position: Position)
    {
        match new_position {
            | Position::Perpetual(p) => {
                if let Some(ref mut positions) = self.perpetual_pos {
                    if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.push(p);
                    }
                }
                else {
                    self.perpetual_pos = Some(vec![p]);
                }
            }
            | Position::LeveragedToken(p) => {
                if let Some(ref mut positions) = self.margin_pos {
                    if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.push(p);
                    }
                }
                else {
                    self.margin_pos = Some(vec![p]);
                }
            }
            | Position::Future(p) => {
                if let Some(ref mut positions) = self.futures_pos {
                    if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.push(p);
                    }
                }
                else {
                    self.futures_pos = Some(vec![p]);
                }
            }
            | Position::Option(p) => {
                if let Some(ref mut positions) = self.option_pos {
                    if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                        *existing_position = p;
                    }
                    else {
                        positions.push(p);
                    }
                }
                else {
                    self.option_pos = Some(vec![p]);
                }
            }
        }
    }

    /// 检查账户中是否持有指定交易工具的仓位
    pub(crate) fn has_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            // 对于现货，检查余额而不是仓位
            | InstrumentKind::Spot => todo!(),
            // 商品期权
            | InstrumentKind::CommodityOption => todo!(),
            // 商品期货
            | InstrumentKind::CommodityFuture => todo!(),
            // 永续合约
            | InstrumentKind::Perpetual => self.perpetual_pos
                                               .as_ref() // 如果存在仓位列表
                                               .map_or(false, |positions| // 如果有任何一个 pos 满足条件，any 返回 true，否则返回 false。
                    positions.iter().any(|pos| pos.meta.instrument == *instrument)),

            // 普通期货
            | InstrumentKind::Future => self.futures_pos
                                            .as_ref()
                                            .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),

            // 加密期权
            | InstrumentKind::CryptoOption => self.option_pos
                                                  .as_ref()
                                                  .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),

            // 加密杠杆代币
            | InstrumentKind::CryptoLeveragedToken => self.margin_pos
                                                          .as_ref()
                                                          .map_or(false, |positions| positions.iter().any(|pos| pos.meta.instrument == *instrument)),
        }
    }
}

/// NOTE : PositionMode 枚举定义了两种交易方向模式：
///  [NetMode] : 单向模式。在这种模式下，用户只能持有一个方向的仓位（多头或空头），而不能同时持有两个方向的仓位。
///  [LongShortMode] : 双向模式。在这种模式下，用户可以同时持有多头和空头仓位。这在一些复杂的交易策略中可能会有用，例如对冲策略。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionDirectionMode
{
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

/// NOTE : PositionMarginMode has defined two modes of margin consumption.
///  [Cross]: 交叉保证金模式。在这种模式下，所有仓位共享一个保证金池，盈亏共用。如果仓位的保证金不足，将从账户余额中提取以补充不足。
///  [Isolated]: 逐仓保证金模式。在这种模式下，每个仓位都有独立的保证金，盈亏互不影响。如果某个仓位的保证金不足，该仓位将被强制平仓，而不会影响到其他仓位。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum PositionMarginMode
{
    Cross,
    Isolated,
}

/// NOTE: 可能需要多种头寸类型共存
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Position
{
    Perpetual(PerpetualPosition),
    LeveragedToken(LeveragedTokenPosition),
    Future(FuturePosition),
    Option(OptionPosition),
}
