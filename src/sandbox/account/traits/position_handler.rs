use crate::{
    common::{
        account_positions::{Position, PositionConfig},
        instrument::Instrument,
    },
    error::ExchangeError,
    sandbox::{config_request::ConfigurationRequest, sandbox_client::ConfigureInstrumentsResults},
};
use async_trait::async_trait;

use crate::common::{
    account_positions::{future::FuturePosition, leveraged_token::LeveragedTokenPosition, option::OptionPosition, perpetual::PerpetualPosition, AccountPositions},
    order::{states::open::Open, Order},
    trade::ClientTrade,
    Side,
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

    async fn any_position_open(&self, open: &Order<Open>) -> Result<bool, ExchangeError>;

    async fn update_position_from_client_trade(&mut self, trade: ClientTrade) -> Result<(), ExchangeError>;

    async fn remove_position(&self, instrument: Instrument, side: Side) -> Option<Position>;

    // /// 将一个已退出的 [`Position`] 附加到投资组合的已退出持仓列表中。
    // fn set_exited_position(&mut self, session_id: Uuid, position: Position) -> Result<(), ExchangeError>;

    // /// 获取与 session_id 相关联的所有已退出的 [`Position`]。
    // fn get_exited_positions(&mut self, session_id: Uuid) -> Result<Vec<Position>, ExchangeError>;
}
