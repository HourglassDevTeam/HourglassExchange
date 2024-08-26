use crate::common_infrastructure::datafeed::event::MarketEvent;
use crate::common_infrastructure::order::{Order, RequestCancel, RequestOpen};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;
use crate::sandbox::sandbox_client::SandBoxClientEvent;
use serde::Deserialize;
use tokio::sync::oneshot;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct NetworkEvent {
    pub(crate) event_type: String,
    pub(crate) payload: String, // TODO 解析payload的方法未实现。
    pub(crate) timestamp: i64, // UNIX 时间戳
    pub(crate) source: String, // 事件来源
    pub(crate) destination: String, // 事件目的地
    pub(crate) event_id: String, // 唯一事件 ID
    pub(crate) version: String, // 事件版本号
    pub(crate) correlation_id: Option<String>, // 关联 ID (可选)
    pub(crate) priority: Option<u8>, // 优先级 (可选)
    pub(crate) retry_count: u8, // 重试次数
}



/// FIXME : the current parsing methods of the payload are only for demonstration purposes.
#[allow(dead_code)]
impl NetworkEvent {
    pub(crate) fn parse_payload(&self) -> Result<SandBoxClientEvent, String> {
        match self.event_type.as_str() {
            "FetchOrdersOpen" => {
                let (response_tx, _response_rx) = oneshot::channel();
                Ok(SandBoxClientEvent::FetchOrdersOpen(response_tx))
            }
            "FetchBalances" => {
                let (response_tx, _response_rx) = oneshot::channel();
                Ok(SandBoxClientEvent::FetchBalances(response_tx))
            }
            "OpenOrders" => {
                // 解析 payload 为 Vec<Order<RequestOpen>> 类型
                let orders: Vec<Order<RequestOpen>> = serde_json::from_str(&self.payload)
                    .map_err(|e| format!("Failed to parse OpenOrders payload: {}", e))?;
                let (response_tx, _response_rx) = oneshot::channel();
                Ok(SandBoxClientEvent::OpenOrders((orders, response_tx)))
            }
            "CancelOrders" => {
                // 解析 payload 为 Vec<Order<RequestCancel>> 类型
                let orders: Vec<Order<RequestCancel>> = serde_json::from_str(&self.payload)
                    .map_err(|e| format!("Failed to parse CancelOrders payload: {}", e))?;
                let (response_tx, _response_rx) = oneshot::channel();
                Ok(SandBoxClientEvent::CancelOrders((orders, response_tx)))
            }
            "CancelOrdersAll" => {
                let (response_tx, _response_rx) = oneshot::channel();
                Ok(SandBoxClientEvent::CancelOrdersAll(response_tx))
            }
            "FetchMarketEvent" => {
                // 解析 payload 为 MarketEvent<ClickhousePublicTrade> 类型
                let market_event: MarketEvent<ClickhousePublicTrade> = serde_json::from_str(&self.payload)
                    .map_err(|e| format!("Failed to parse FetchMarketEvent payload: {}", e))?;
                Ok(SandBoxClientEvent::FetchMarketEvent(market_event))
            }
            _ => Err("Unknown event type".to_string()),
        }
    }
}