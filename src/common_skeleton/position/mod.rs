use serde::{Deserialize, Serialize};
use crate::common_skeleton::balance::TokenBalance;
use crate::common_skeleton::friction::Fees;
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::Side;
use crate::ExchangeVariant;

pub(crate) mod positon_meta;
pub mod perpetual;
mod option;
mod future;
mod leveraged_token;

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PositionMeta {
    pub position_id: String,
    pub enter_ts: i64,
    pub update_ts: i64,
    pub exit_balance: TokenBalance, // NOTE necessary but unsure currently
    pub account_exchange_ts: i64,
    pub exchange: ExchangeVariant,
    pub instrument: Instrument,
    pub side: Side, // NOTE 注意和DirectionMode之间的兼容性
    pub current_size: f64,
    pub current_fees_total: Fees,
    pub current_avg_price_gross: f64,
    pub current_symbol_price: f64,
    pub current_avg_price: f64,
    pub unrealised_pnl: f64,
    pub realised_pnl: f64,
}

pub struct PositionMetaBuilder {
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

impl PositionMetaBuilder {
    pub fn new() -> Self {
        Self {
            position_id: None,
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
            realised_pnl: None,
        }
    }

    pub fn position_id(mut self, position_id: String) -> Self {
        self.position_id = Some(position_id);
        self
    }

    pub fn enter_ts(mut self, enter_ts: i64) -> Self {
        self.enter_ts = Some(enter_ts);
        self
    }

    pub fn update_ts(mut self, update_ts: i64) -> Self {
        self.update_ts = Some(update_ts);
        self
    }

    pub fn exit_balance(mut self, exit_balance: TokenBalance) -> Self {
        self.exit_balance = Some(exit_balance);
        self
    }

    pub fn account_exchange_ts(mut self, account_exchange_ts: i64) -> Self {
        self.account_exchange_ts = Some(account_exchange_ts);
        self
    }

    pub fn exchange(mut self, exchange: ExchangeVariant) -> Self {
        self.exchange = Some(exchange);
        self
    }

    pub fn instrument(mut self, instrument: Instrument) -> Self {
        self.instrument = Some(instrument);
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn current_size(mut self, current_size: f64) -> Self {
        self.current_size = Some(current_size);
        self
    }

    pub fn current_fees_total(mut self, current_fees_total: Fees) -> Self {
        self.current_fees_total = Some(current_fees_total);
        self
    }

    pub fn current_avg_price_gross(mut self, current_avg_price_gross: f64) -> Self {
        self.current_avg_price_gross = Some(current_avg_price_gross);
        self
    }

    pub fn current_symbol_price(mut self, current_symbol_price: f64) -> Self {
        self.current_symbol_price = Some(current_symbol_price);
        self
    }

    pub fn current_avg_price(mut self, current_avg_price: f64) -> Self {
        self.current_avg_price = Some(current_avg_price);
        self
    }

    pub fn unrealised_pnl(mut self, unrealised_pnl: f64) -> Self {
        self.unrealised_pnl = Some(unrealised_pnl);
        self
    }

    pub fn realised_pnl(mut self, realised_pnl: f64) -> Self {
        self.realised_pnl = Some(realised_pnl);
        self
    }

    pub fn build(self) -> Result<PositionMeta, &'static str> {
        Ok(PositionMeta {
            position_id: self.position_id.ok_or("position_id is required")?,
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
            realised_pnl: self.realised_pnl.ok_or("realised_pnl is required")?,
        })
    }}