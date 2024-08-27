use crate::{
    common::{
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
        trade::ClientTrade,
        Side,
    },
    error::ExecutionError,
    sandbox::account::account_config::AccountConfig,
    Exchange,
};
/// FIXME  : code below needs to be restructured and fitted to the framework. need to provide enums?
/// CONSIDER: can these positions coexist, if so enums might not be ideal.
use serde::{Deserialize, Serialize};

pub(crate) mod future;
pub(crate) mod leveraged_token;
pub(crate) mod option;
pub mod perpetual;
pub(crate) mod position_meta;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountPositions
{
    pub margin_pos: Vec<LeveragedTokenPosition>, // NOTE useless in backtest
    pub perpetual_pos: Vec<PerpetualPosition>,
    pub futures_pos: Vec<FuturePosition>,
    pub option_pos: Vec<OptionPosition>,
}

impl AccountPositions
{
    /// 创建一个新的 AccountPositions 实例
    pub fn init() -> Self
    {
        Self { margin_pos: Vec::new(),
               perpetual_pos: Vec::new(),
               futures_pos: Vec::new(),
               option_pos: Vec::new() }
    }

    pub async fn build_new_perpetual_position(&self,
                                              config: &AccountConfig,
                                              trade: &ClientTrade, // 使用 ClientTrade 作为输入参数
                                              pos_margin_mode: PositionMarginMode,
                                              position_mode: PositionDirectionMode,
                                              exchange_ts: i64)
                                              -> Result<PerpetualPosition, ExecutionError>
    {
        let maker_rate = config.get_maker_fee_rate(&trade.instrument.kind)?;
        let taker_rate = config.get_taker_fee_rate(&trade.instrument.kind)?;
        // 计算初始保证金
        let initial_margin = trade.price * trade.quantity / config.account_leverage_rate;
        // 计算费用
        let maker_fee = trade.quantity * trade.price * maker_rate;
        let taker_fee = trade.quantity * trade.price * taker_rate;
        let funding_fee = trade.quantity * trade.price * config.funding_rate;

        // 根据 Instrument 和 Side 动态生成 position_id
        let position_meta = PositionMetaBuilder::new().position_id(format!("{}_{}", trade.instrument, if trade.side == Side::Buy { "Long" } else { "Short" }))
                                                      .enter_ts(exchange_ts)
                                                      .update_ts(exchange_ts)
                                                      .exit_balance(TokenBalance { // 初始化为 exit_balance
                                                                                   token: trade.instrument.base.clone(),
                                                                                   balance: Balance { current_price: trade.price,
                                                                                                      total: trade.quantity,
                                                                                                      available: trade.quantity } })
                                                      .exchange(Exchange::SandBox)
                                                      .instrument(trade.instrument.clone())
                                                      .side(trade.side)
                                                      .current_size(trade.quantity)
                                                      .current_fees_total(Fees::Perpetual(PerpetualFees { maker_fee,
                                                                                                          taker_fee, // 假设平仓费率与开仓费率相同
                                                                                                          funding_fee }))
                                                      .current_avg_price_gross(trade.price)
                                                      .current_symbol_price(trade.price)
                                                      .current_avg_price(trade.price)
                                                      .unrealised_pnl(0.0) // 初始化为 0.0
                                                      .realised_pnl(0.0) // 初始化为 0.0
                                                      .build()
                                                      .map_err(|err| ExecutionError::SandBox(format!("Failed to build position meta: {}", err)))?;

        // 计算 liquidation_price
        let liquidation_price = if trade.side == Side::Buy {
            trade.price * (1.0 - initial_margin / (trade.quantity * trade.price))
        }
        else {
            trade.price * (1.0 + initial_margin / (trade.quantity * trade.price))
        };
        let pos_config = PerpetualPositionConfig { pos_margin_mode,
                                                   leverage: config.account_leverage_rate,
                                                   position_mode };

        let new_position = PerpetualPositionBuilder::new().meta(position_meta)
                                                          .pos_config(pos_config)
                                                          .liquidation_price(liquidation_price)
                                                          .margin(initial_margin) // NOTE DOUBLE CHECK
                                                          .build()
                                                          .ok_or_else(|| ExecutionError::SandBox("Failed to build new position".to_string()))?;

        Ok(new_position)
    }

    /// 更新或添加新的仓位
    pub fn update_position(&mut self, new_position: Position)
    {
        match new_position {
            | Position::Perpetual(p) => {
                let ref mut positions = self.perpetual_pos;
                if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                    *existing_position = p;
                }
                else {
                    positions.push(p);
                }
            }

            | Position::LeveragedToken(p) => {
                let ref mut positions = self.margin_pos;
                if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                    *existing_position = p;
                }
                else {
                    positions.push(p);
                }
            }

            | Position::Future(p) => {
                let ref mut positions = self.futures_pos;
                if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                    *existing_position = p;
                }
                else {
                    positions.push(p);
                }
            }

            | Position::Option(p) => {
                let ref mut positions = self.option_pos;
                if let Some(existing_position) = positions.iter_mut().find(|pos| pos.meta.instrument == p.meta.instrument) {
                    *existing_position = p;
                }
                else {
                    positions.push(p);
                }
            }
        }
    }

    /// 检查账户中是否持有指定交易工具的仓位
    /// 检查账户中是否持有指定交易工具的仓位
    pub(crate) fn has_position(&self, instrument: &Instrument) -> bool
    {
        match instrument.kind {
            // 对于现货，检查余额而不是仓位
            | InstrumentKind::Spot => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of Spot as of yet."),
            // 商品期权
            | InstrumentKind::CommodityOption => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityOption as of yet."),
            // 商品期货
            | InstrumentKind::CommodityFuture => todo!("[UniLinkExecution] : The system does not support creation or processing of positions of CommodityFuture as of yet."),
            // 永续合约
            | InstrumentKind::Perpetual => self.perpetual_pos
                                               .iter() // 直接迭代 Vec<PerpetualPosition>
                                               .any(|pos| pos.meta.instrument == *instrument),

            // 普通期货
            | InstrumentKind::Future => self.futures_pos
                                            .iter() // 直接迭代 Vec<FuturePosition>
                                            .any(|pos| pos.meta.instrument == *instrument),

            // 加密期权
            | InstrumentKind::CryptoOption => self.option_pos
                                                  .iter() // 直接迭代 Vec<OptionPosition>
                                                  .any(|pos| pos.meta.instrument == *instrument),

            // 加密杠杆代币
            | InstrumentKind::CryptoLeveragedToken => self.margin_pos
                                                          .iter() // 直接迭代 Vec<LeveragedTokenPosition>
                                                          .any(|pos| pos.meta.instrument == *instrument),
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

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::token::Token;

    fn create_instrument(kind: InstrumentKind) -> Instrument
    {
        Instrument { base: Token::from("BTC"),
                     quote: Token::from("USDT"),
                     kind }
    }

    fn create_perpetual_position(instrument: &Instrument) -> PerpetualPosition
    {
        // 假设初始交易价格和仓位大小
        let initial_trade_price = 50000.0;
        let trade_size = 1.0;

        // 设置杠杆率
        let leverage = 10.0;

        // 计算初始保证金
        let initial_margin = initial_trade_price * trade_size / leverage;

        // 假设当前市场价格略有波动
        let current_market_price = 50500.0;

        // 计算当前仓位的未实现盈亏
        let unrealised_pnl = (current_market_price - initial_trade_price) * trade_size;

        // 计算清算价格 (liquidation_price)
        let liquidation_price = initial_trade_price * (1.0 - initial_margin / (trade_size * initial_trade_price));

        PerpetualPosition { meta: PositionMetaBuilder::new().position_id("test_position".into())
                                                            .instrument(instrument.clone())
                                                            .side(Side::Buy)
                                                            .enter_ts(1625097600000)
                                                            .update_ts(1625097600000)
                                                            .exit_balance(TokenBalance { token: instrument.base.clone(),
                                                                                         balance: Balance { current_price: current_market_price,
                                                                                                            total: trade_size,
                                                                                                            available: trade_size } })
                                                            .exchange(Exchange::Binance)
                                                            .current_size(trade_size)
                                                            .current_fees_total(Fees::Perpetual(PerpetualFees { maker_fee: 0.1 * trade_size,
                                                                                                                taker_fee: 0.1 * trade_size,
                                                                                                                funding_fee: 0.0 }))
                                                            .current_avg_price_gross(initial_trade_price)
                                                            .current_symbol_price(current_market_price)
                                                            .current_avg_price(initial_trade_price)
                                                            .unrealised_pnl(unrealised_pnl)
                                                            .realised_pnl(0.0)
                                                            .build()
                                                            .unwrap(),
                            liquidation_price,
                            margin: initial_margin,
                            pos_config: PerpetualPositionConfig { pos_margin_mode: PositionMarginMode::Cross,
                                                                  leverage,
                                                                  position_mode: PositionDirectionMode::NetMode } }
    }

    #[test]
    fn test_has_position()
    {
        let mut account_positions = AccountPositions::init();

        let perpetual_instrument = create_instrument(InstrumentKind::Perpetual);
        let future_instrument = create_instrument(InstrumentKind::Future);

        // 初始情况下，没有任何仓位
        assert!(!account_positions.has_position(&perpetual_instrument));
        assert!(!account_positions.has_position(&future_instrument));

        // 添加 PerpetualPosition
        let perpetual_position = create_perpetual_position(&perpetual_instrument);
        account_positions.update_position(Position::Perpetual(perpetual_position));

        // 现在应该持有 PerpetualPosition，但不持有 FuturePosition
        assert!(account_positions.has_position(&perpetual_instrument));
        assert!(!account_positions.has_position(&future_instrument));
    }

    #[test]
    fn test_update_existing_position()
    {
        let mut account_positions = AccountPositions::init();

        let perpetual_instrument = create_instrument(InstrumentKind::Perpetual);

        // 添加初始的 PerpetualPosition
        let perpetual_position = create_perpetual_position(&perpetual_instrument);
        account_positions.update_position(Position::Perpetual(perpetual_position.clone()));

        // 确保初始 PerpetualPosition 已正确添加
        assert!(account_positions.has_position(&perpetual_instrument));
        assert_eq!(account_positions.perpetual_pos.len(), 1);

        // 更新相同的 PerpetualPosition，修改 `margin`
        let mut updated_position = perpetual_position.clone();
        updated_position.margin = 2000.0; // 修改仓位的保证金

        account_positions.update_position(Position::Perpetual(updated_position.clone()));

        // 确保仓位已更新而不是新添加
        if !account_positions.perpetual_pos.is_empty() {
            assert_eq!(account_positions.perpetual_pos.len(), 1); // 确保仓位数量未增加
            let pos = &account_positions.perpetual_pos[0];
            assert_eq!(pos.margin, 2000.0); // 检查仓位是否已正确更新
        }
        else {
            panic!("PerpetualPosition should exist but was not found.");
        }
    }

    #[test]
    fn test_add_new_position()
    {
        let mut account_positions = AccountPositions::init();

        // 创建两个不同的 Instrument
        let perpetual_instrument_1 = Instrument { base: Token::from("BTC"),
                                                  quote: Token::from("USDT"),
                                                  kind: InstrumentKind::Perpetual };

        let perpetual_instrument_2 = Instrument { base: Token::from("ETH"),
                                                  quote: Token::from("USDT"),
                                                  kind: InstrumentKind::Perpetual };

        // 添加初始的 PerpetualPosition
        let perpetual_position_1 = create_perpetual_position(&perpetual_instrument_1);
        account_positions.update_position(Position::Perpetual(perpetual_position_1.clone()));

        // 添加新的 PerpetualPosition
        let perpetual_position_2 = create_perpetual_position(&perpetual_instrument_2);
        account_positions.update_position(Position::Perpetual(perpetual_position_2.clone()));

        // 确保新仓位已正确添加
        assert!(account_positions.has_position(&perpetual_instrument_1));
        assert!(account_positions.has_position(&perpetual_instrument_2));
        assert_eq!(account_positions.perpetual_pos.len(), 2);
    }
}
