use crate::common::balance::Balance;
use serde::{Deserialize, Serialize};

use crate::common::trade::ClientTrade;
use crate::{
    common::{account_positions::position_id::PositionId, balance::TokenBalance, instrument::Instrument, Side},
    Exchange,
};
use crate::common::account_positions::PositionDirectionMode;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta
{
    pub position_id: PositionId,      // 静态数据
    pub enter_ts: i64,                // 静态数据
    pub update_ts: i64,               // 实时更新
    pub exit_balance: TokenBalance,   // 静态更新（退出时更新）当一个仓位被平仓（即完全退出）时，该仓位所涉及的资产或资金的最终状态。
    pub exchange: Exchange,           // 静态数据
    pub instrument: Instrument,       // 静态数据
    pub side: Side,                   // 静态数据
    pub current_size: f64,            // 实时更新
    pub current_fees_total: f64,     // 实时更新
    pub current_avg_price_gross: f64, // 实时更新，即没有考虑费用或其他扣减项的情况下计算的平均持仓价格。
    pub current_symbol_price: f64,    // 实时更新，当前交易标的（symbol，如股票、期货合约、加密货币等）的最新市场价格。
    pub current_avg_price: f64,       // 实时更新
    pub unrealised_pnl: f64,          // 实时更新
    pub realised_pnl: f64,            // 静态更新（平仓时更新）
}

/// FIXME 虽然 Net Mode 和 LongShort Mode 在很多地方可以复用相似的逻辑，
///         但为了减少未来可能的逻辑混淆和复杂性，建议进一步明确两种模式的职责，
///         尤其是在处理复杂的反向开仓和部分平仓的情况下。
impl PositionMeta {
    /// 根据 `ClientTrade` 和模式更新仓位
    pub fn update_from_trade(&mut self, trade: &ClientTrade, current_symbol_price: f64, mode: PositionDirectionMode) {
        self.update_ts = trade.timestamp;
        self.current_symbol_price = current_symbol_price;

        // 更新当前交易的总费用（独立于模式的部分）
        self.current_fees_total += trade.fees;

        match mode {
            PositionDirectionMode::Net => {
                // Net Mode 逻辑：直接更新均价和持仓大小
                self.update_avg_price(trade.price, trade.quantity);
                self.update_unrealised_pnl();
            }
            PositionDirectionMode::LongShort => {
                // LongShort Mode 逻辑：根据方向更新多头或空头
                if trade.side == Side::Buy {
                    self.update_long_position(trade.price, trade.quantity);
                } else {
                    self.update_short_position(trade.price, trade.quantity);
                }
                self.update_unrealised_pnl();  // 同样可以在 LongShort 中调用以分别计算
            }
        }
    }
    /// 创建新的 `PositionMeta` 基于 `ClientTrade`

    pub fn create_from_trade(trade: &ClientTrade, current_symbol_price: f64) -> Self {

        PositionMeta {
            position_id: PositionId::new(&trade.instrument, trade.timestamp),
            enter_ts: trade.timestamp,
            update_ts: trade.timestamp,
            exit_balance: TokenBalance::new(trade.instrument.base.clone(), Balance::new(0.0, 0.0, 0.0)),
            exchange: Exchange::SandBox,
            instrument: trade.instrument.clone(),
            side: trade.side,
            current_size: trade.quantity,
            current_fees_total: trade.fees,
            current_avg_price_gross: trade.price,
            current_symbol_price,
            current_avg_price: trade.price,
            unrealised_pnl: 0.0,
            realised_pnl: 0.0,
        }
    }


    /// Handle new position creation in reverse with remaining quantity.
    pub fn from_trade_with_remaining(trade: &ClientTrade, current_symbol_price: f64, side: Side, remaining_quantity: f64) -> Self {
        let mut new_meta = PositionMeta::create_from_trade(trade, current_symbol_price);
        new_meta.current_size = remaining_quantity;
        new_meta.side = side;
        new_meta
    }

    /// Update or create a position based on a new trade.
    /// This handles both regular updates and reverse position logic.
    pub fn update_or_create_from_trade(&mut self, trade: &ClientTrade, current_symbol_price: f64, mode: PositionDirectionMode) -> Self {
        if self.side == trade.side {
            // Update position normally if the trade is in the same direction
            self.update_from_trade(trade, current_symbol_price, mode);
            self.clone() // Return the updated position
        } else {
            // If trade side is opposite, reduce or close the current position and possibly open a new one.
            let remaining_quantity = trade.quantity - self.current_size;
            if remaining_quantity >= 0.0 {
                // Fully close the current position and reverse the position with remaining quantity
                self.update_realised_pnl(trade.price);
                PositionMeta::from_trade_with_remaining(trade, current_symbol_price, trade.side, remaining_quantity)
            } else {
                // Partial close, no reverse, just reduce the size
                self.current_size -= trade.quantity;
                self.update_realised_pnl(trade.price);
                self.clone() // Return the updated position
            }
        }
    }
}


impl PositionMeta
{

    /// Net Mode 下更新均价和持仓大小
    fn update_avg_price(&mut self, trade_price: f64, trade_size: f64) {
        let total_size = self.current_size + trade_size;

        if total_size > 0.0 {
            // 计算新的持仓均价（未考虑费用的粗略均价）
            self.current_avg_price_gross = (self.current_avg_price_gross * self.current_size + trade_price * trade_size) / total_size;
            self.current_size = total_size;
        }

        // 更新平均价格（默认 gross 作为基础）
        self.current_avg_price = self.current_avg_price_gross;
    }


    /// 更新 current_avg_price，同时考虑费用
    /// 在 update_avg_price_and_fees 方法中，您试图在计算平均价格时加入费用，但当前的计算公式可能会导致均价计算不准确。
    /// 特别是在考虑交易费用的情况下，费用应被视为独立于价格的一项成本，而不是直接加入到价格中去。
    pub fn update_avg_price_and_fees(&mut self, trade_price: f64, trade_size: f64, trade_fees: f64)
    {
        // 计算总费用（直接从 `ClientTrade` 中获取）
        self.current_fees_total += trade_fees;

        // 调用方法更新均价（不处理费用，在外部考虑费用）
        self.update_avg_price(trade_price, trade_size);

        // 考虑费用后的均价更新
        if self.current_size > 0.0 {
            self.current_avg_price = (self.current_avg_price_gross * self.current_size + self.current_fees_total) / self.current_size;
        }
    }


    /// LongShort Mode 下更新多头仓位
    fn update_long_position(&mut self, trade_price: f64, trade_size: f64) {
        // 更新多头均价和持仓大小（与 `Net Mode` 中类似，但仅限多头）
        self.update_avg_price(trade_price, trade_size);
    }

    /// LongShort Mode 下更新空头仓位
    fn update_short_position(&mut self, trade_price: f64, trade_size: f64) {
        // 更新空头逻辑，可以扩展为适应更多逻辑
        self.update_avg_price(trade_price, trade_size);
    }

    /// 更新 unrealised_pnl
    /// FIXME 在更新未实现盈亏时，现在使用 self.current_size 来计算，但是在反向仓位或部分平仓的情况下，可能会有问题，
    /// FIXME 因为仓位大小已经发生变化。建议确保每次在更新未实现盈亏时，考虑实际持仓方向和剩余仓位大小。
    pub fn update_unrealised_pnl(&mut self)
    {
        self.unrealised_pnl = (self.current_symbol_price - self.current_avg_price) * self.current_size;
    }

    /// 更新 realised_pnl 并清空持仓
    /// FIXME
    ///     - 在 update_realised_pnl 方法中，您在平仓时计算已实现盈亏后将持仓重置为 0，这对 Net Mode 是合理的，
    ///     - 但是在 LongShort Mode 或反向开仓时，可能需要根据情况保留部分持仓或反转方向，而不是直接将所有仓位重置为 0。
    ///
    pub fn update_realised_pnl(&mut self, closing_price: f64)
    {
        self.realised_pnl = (closing_price - self.current_avg_price) * self.current_size;
        // 清空当前持仓
        self.current_size = 0.0;
        self.current_avg_price = 0.0;
        self.current_avg_price_gross = 0.0;
        self.current_fees_total = 0.0;  // 清空费用
    }
}

pub struct PositionMetaBuilder
{
    position_id: Option<PositionId>,
    enter_ts: Option<i64>,
    update_ts: Option<i64>,
    exit_balance: Option<TokenBalance>,
    exchange: Option<Exchange>,
    instrument: Option<Instrument>,
    side: Option<Side>,
    current_size: Option<f64>,
    current_fees_total: Option<f64>,
    current_avg_price_gross: Option<f64>,
    current_symbol_price: Option<f64>,
    current_avg_price: Option<f64>,
    unrealised_pnl: Option<f64>,
    realised_pnl: Option<f64>,
}

#[allow(dead_code)]
impl PositionMetaBuilder
{
    pub fn new() -> Self
    {
        Self { position_id: None,
               enter_ts: None,
               update_ts: None,
               exit_balance: None,
               exchange: None,
               instrument: None,
               side: None,
               current_size: None,
               current_fees_total: None,
               current_avg_price_gross: None,
               current_symbol_price: None,
               current_avg_price: None,
               unrealised_pnl: None,
               realised_pnl: None }
    }

    pub fn position_id(mut self, position_id: PositionId) -> Self
    {
        self.position_id = Some(position_id);
        self
    }

    pub fn enter_ts(mut self, enter_ts: i64) -> Self
    {
        self.enter_ts = Some(enter_ts);
        self
    }

    pub fn update_ts(mut self, update_ts: i64) -> Self
    {
        self.update_ts = Some(update_ts);
        self
    }

    pub fn exit_balance(mut self, exit_balance: TokenBalance) -> Self
    {
        self.exit_balance = Some(exit_balance);
        self
    }

    pub fn exchange(mut self, exchange: Exchange) -> Self
    {
        self.exchange = Some(exchange);
        self
    }

    pub fn instrument(mut self, instrument: Instrument) -> Self
    {
        self.instrument = Some(instrument);
        self
    }

    pub fn side(mut self, side: Side) -> Self
    {
        self.side = Some(side);
        self
    }

    pub fn current_size(mut self, current_size: f64) -> Self
    {
        self.current_size = Some(current_size);
        self
    }

    pub fn current_fees_total(mut self, current_fees_total: f64) -> Self
    {
        self.current_fees_total = Some(current_fees_total);
        self
    }

    pub fn current_avg_price_gross(mut self, current_avg_price_gross: f64) -> Self
    {
        self.current_avg_price_gross = Some(current_avg_price_gross);
        self
    }

    pub fn current_symbol_price(mut self, current_symbol_price: f64) -> Self
    {
        self.current_symbol_price = Some(current_symbol_price);
        self
    }

    pub fn current_avg_price(mut self, current_avg_price: f64) -> Self
    {
        self.current_avg_price = Some(current_avg_price);
        self
    }

    pub fn unrealised_pnl(mut self, unrealised_pnl: f64) -> Self
    {
        self.unrealised_pnl = Some(unrealised_pnl);
        self
    }

    pub fn realised_pnl(mut self, realised_pnl: f64) -> Self
    {
        self.realised_pnl = Some(realised_pnl);
        self
    }

    pub fn build(self) -> Result<PositionMeta, &'static str>
    {
        Ok(PositionMeta { position_id: self.position_id.ok_or("position_id is required")?,
                          enter_ts: self.enter_ts.ok_or("enter_ts is required")?,
                          update_ts: self.update_ts.ok_or("update_ts is required")?,
                          exit_balance: self.exit_balance.ok_or("exit_balance is required")?,
                          exchange: self.exchange.ok_or("exchange is required")?,
                          instrument: self.instrument.ok_or("instrument is required")?,
                          side: self.side.ok_or("side is required")?,
                          current_size: self.current_size.ok_or("current_size is required")?,
                          current_fees_total: self.current_fees_total.ok_or("current_fees_total is required")?,
                          current_avg_price_gross: self.current_avg_price_gross.ok_or("current_avg_price_gross is required")?,
                          current_symbol_price: self.current_symbol_price.ok_or("current_symbol_price is required")?,
                          current_avg_price: self.current_avg_price.ok_or("current_avg_price is required")?,
                          unrealised_pnl: self.unrealised_pnl.ok_or("unrealised_pnl is required")?,
                          realised_pnl: self.realised_pnl.ok_or("realised_pnl is required")? })
    }
}

impl Default for PositionMetaBuilder
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::order::identification::OrderId;
    use crate::common::trade::{ClientTrade, ClientTradeId};
    use crate::common::{
        instrument::{kind::InstrumentKind, Instrument},
        Side,
    };

    /// Helper function to create a ClientTrade for testing
    fn create_test_trade() -> ClientTrade {
        ClientTrade {
            timestamp: 1625247600,
            trade_id: ClientTradeId::from(1),  // This works fine
            order_id: OrderId::new(1625247600, 1, 1),  // Use the constructor for OrderId
            cid: None,
            instrument: Instrument::new("BTC", "USDT", InstrumentKind::Spot),
            side: Side::Buy,
            price: 50_000.0,
            quantity: 1.0,
            fees: 2.0,
        }
    }

    #[test]
    fn test_create_position_meta_from_trade() {
        let trade = create_test_trade();
        let position_meta = PositionMeta::create_from_trade(&trade, 61_000.0);

        assert_eq!(position_meta.current_size, trade.quantity);
        assert_eq!(position_meta.current_avg_price, trade.price);
        assert_eq!(position_meta.current_symbol_price, 61_000.0);
        assert_eq!(position_meta.current_fees_total, trade.fees);
    }



    #[test]
    fn test_update_avg_price_with_fees() {
        let mut meta = PositionMeta::create_from_trade(&create_test_trade(), 61_000.0);
        meta.update_avg_price_and_fees(60_000.0, 1.0, 2.0);  // Include additional fees

        assert!(meta.current_avg_price > meta.current_avg_price_gross);  // Avg price includes fees
        assert_eq!(meta.current_size, 2.0);  // Size should be updated
    }

    #[test]
    fn test_update_unrealised_pnl() {
        let mut meta = PositionMeta::create_from_trade(&create_test_trade(), 61_000.0);
        meta.update_unrealised_pnl();

        assert_eq!(meta.unrealised_pnl, 11_000.0);  // Difference between current price and avg price
    }

    #[test]
    fn test_update_realised_pnl_and_clear_position() {
        let mut meta = PositionMeta::create_from_trade(&create_test_trade(), 61_000.0);
        meta.update_realised_pnl(55_000.0);  // Closing at 55,000

        assert_eq!(meta.realised_pnl, 5_000.0);  // Realised PnL should be 5,000
        assert_eq!(meta.current_size, 0.0);  // Position should be closed
        assert_eq!(meta.current_avg_price, 0.0);  // Avg price reset
        assert_eq!(meta.current_avg_price_gross, 0.0);  // Avg price gross reset
        assert_eq!(meta.current_fees_total, 0.0);  // Fees reset
    }

    #[test]
    fn test_update_from_trade() {
        let mut meta = PositionMeta::create_from_trade(&create_test_trade(), 61_000.0);
        let new_trade = ClientTrade {
            timestamp: 1625248600,
            trade_id: ClientTradeId::from(1),  // This works fine
            order_id: OrderId::new(1625247600, 1, 1),  // Use the constructor for OrderId
            cid: None,
            instrument: Instrument::new("BTC", "USDT", InstrumentKind::Spot),
            side: Side::Buy,
            price: 60_000.0,
            quantity: 1.0,
            fees: 2.0,
        };

        meta.update_from_trade(&new_trade, 62_000.0,PositionDirectionMode::Net);

        assert_eq!(meta.current_size, 2.0);  // Size should be updated
        assert_eq!(meta.current_avg_price, 55_000.0);  // The avg price should be exactly 55,000.0
        assert_eq!(meta.current_symbol_price, 62_000.0);  // Symbol price updated
        assert_eq!(meta.current_fees_total, 4.0);  // Fees should accumulate
    }
}
