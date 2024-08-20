use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{balance::TokenBalance, friction::Fees, instrument::Instrument, Side},
    ExchangeVariant,
};

#[derive(Debug, PartialEq)]
pub enum TransactionType
{
    Open,  // 开仓
    Close, // 平仓
}
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta
{
    pub position_id: String,          // 静态数据
    pub enter_ts: i64,                // 静态数据
    pub update_ts: i64,               // 实时更新
    pub exit_balance: TokenBalance,   // 静态更新（退出时更新）
    pub account_exchange_ts: i64,     // 实时更新
    pub exchange: ExchangeVariant,    // 静态数据
    pub instrument: Instrument,       // 静态数据
    pub side: Side,                   // 静态数据
    pub current_size: f64,            // 实时更新
    pub current_fees_total: Fees,     // 实时更新
    pub current_avg_price_gross: f64, // 实时更新，即没有考虑费用或其他扣减项的情况下计算的平均持仓价格。
    pub current_symbol_price: f64,    // 实时更新，当前交易标的（symbol，如股票、期货合约、加密货币等）的最新市场价格。
    pub current_avg_price: f64,       // 实时更新
    pub unrealised_pnl: f64,          // 实时更新
    pub realised_pnl: f64,            // 静态更新（平仓时更新）
}

impl PositionMeta
{
    // CONSIDER 是否应该吧close fees成本计算进去
    fn calculate_avg_price(&mut self, trade_price: f64, trade_size: f64, include_fees: bool, transaction_type: TransactionType)
    {
        let total_size = self.current_size + trade_size;
        if total_size > 0.0 {
            self.current_avg_price_gross = (self.current_avg_price_gross * self.current_size + trade_price * trade_size) / total_size;
            self.current_size = total_size;
        }

        let total_fees = if include_fees && transaction_type == TransactionType::Open {
            match &self.current_fees_total {
                | Fees::Spot(fee) => fee.taker_fee_rate * self.current_size,
                | Fees::Perpetual(fee) => fee.open_fee_rate * self.current_size,
                | Fees::Option(fee) => fee.trade_fee_rate * self.current_size,
            }
        }
        else {
            0.0
        };

        if self.current_size > 0.0 {
            self.current_avg_price = (self.current_avg_price_gross * self.current_size + total_fees) / self.current_size;
        }
        else {
            self.current_avg_price = self.current_avg_price_gross;
        }
    }

    /// 更新 current_avg_price_gross
    pub fn update_avg_price_gross(&mut self, trade_price: f64, trade_size: f64, transaction_type: TransactionType)
    {
        self.calculate_avg_price(trade_price, trade_size, false, transaction_type);
    }

    /// 更新 current_avg_price，同时考虑费用
    pub fn update_avg_price(&mut self, trade_price: f64, trade_size: f64, fees: Fees, transaction_type: TransactionType)
    {
        // 更新 current_fees_total，基于新交易的费用
        self.current_fees_total = fees;

        // 调用通用方法计算并更新平均价格
        self.calculate_avg_price(trade_price, trade_size, true, transaction_type);
    }

    /// 更新 current_symbol_price
    pub fn update_symbol_price(&mut self, new_symbol_price: f64)
    {
        self.current_symbol_price = new_symbol_price;
    }

    /// FIXME ：检验逻辑 更新 unrealised_pnl
    pub fn update_unrealised_pnl(&mut self)
    {
        self.unrealised_pnl = (self.current_symbol_price - self.current_avg_price) * self.current_size;
    }

    /// FIXME ：检验逻辑 更新 realised_pnl
    pub fn update_realised_pnl(&mut self, closing_price: f64)
    {
        self.realised_pnl = (closing_price - self.current_avg_price) * self.current_size;
        // 清空当前持仓
        self.current_size = 0.0;
        self.current_avg_price = 0.0;
        self.current_avg_price_gross = 0.0;
    }
}

pub struct PositionMetaBuilder
{
    position_id: Option<String>,
    enter_ts: Option<i64>,
    update_ts: Option<i64>,
    exit_balance: Option<TokenBalance>,
    account_exchange_ts: Option<i64>,
    exchange: Option<ExchangeVariant>,
    instrument: Option<Instrument>,
    side: Option<Side>,
    current_size: Option<f64>,
    current_fees_total: Option<Fees>,
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
               account_exchange_ts: None,
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

    pub fn position_id(mut self, position_id: String) -> Self
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

    pub fn account_exchange_ts(mut self, account_exchange_ts: i64) -> Self
    {
        self.account_exchange_ts = Some(account_exchange_ts);
        self
    }

    pub fn exchange(mut self, exchange: ExchangeVariant) -> Self
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

    pub fn current_fees_total(mut self, current_fees_total: Fees) -> Self
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
                          account_exchange_ts: self.account_exchange_ts.ok_or("account_exchange_ts is required")?,
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
