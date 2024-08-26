/// # NetworkEvent 结构体
///
/// `NetworkEvent` 是一个用于在网络中传递事件的结构体，包含了事件的类型和相关的负载数据（payload）。
/// 客户端可以通过 `NetworkEvent` 来发送不同类型的事件到服务器，以触发相应的处理逻辑。
///
/// ## 示例
///
/// 下面是一个创建并发送 `"OpenOrders"` 事件的示例：
///
/// ```rust
/// use serde_json::json;
/// use uuid::Uuid;
/// use unilink_execution::common_infrastructure::event::ClientOrderId;
/// use unilink_execution::common_infrastructure::instrument::Instrument;
/// use unilink_execution::common_infrastructure::instrument::kind::InstrumentKind;
/// use unilink_execution::common_infrastructure::order::{Order, OrderExecutionType, RequestOpen};
/// use unilink_execution::common_infrastructure::Side;
/// use unilink_execution::ExchangeVariant;
/// use unilink_execution::network::event::NetworkEvent;
///
/// fn create_open_orders_event() -> NetworkEvent {
///     // 1. 确定事件类型
///     let event_type = "OpenOrders";
///
///     // 2. 构建 payload
///     let orders = vec![
///         Order {
///             kind: OrderExecutionType::Limit,   // 订单类型，例如限价单
///             exchange: ExchangeVariant::Binance, // 交易所名称
///             instrument: Instrument::new("BTC","USDT",InstrumentKind::Perpetual), // 交易对
///             client_ts: chrono::Utc::now().timestamp_millis(), // 客户端下单时间戳
///             client_order_id: ClientOrderId(Uuid::new_v4()), // 客户端订单 ID
///             side: Side::Buy, // 买卖方向
///             state: RequestOpen {
///                 reduce_only: false, // 非减仓订单
///                 price: 50000.0,      // 下单价格
///                 size: 1.0,           // 下单数量
///             },
///         }
///     ];
///
///    // 序列化 orders 为 JSON 字符串
///     let payload = serde_json::to_string(&orders).expect("Failed to serialize orders");
///      NetworkEvent{
///            event_type: event_type.to_string(),
///             payload: payload.to_string(),
///             timestamp: chrono::Utc::now().timestamp(),
///             source: "source_module".to_string(),
///             destination: "destination_module".to_string(),
///             event_id: uuid::Uuid::new_v4().to_string(),
///             version: "1.0".to_string(),
///             correlation_id: None,
///             priority: Some(0),
///             retry_count: 0,}
///
/// }
/// ```
///
/// ## 参数
///
/// - `event_type`: 表示事件的类型，例如 `"OpenOrders"`。
/// - `payload`: 事件的负载数据，通常是 JSON 序列化后的字符串。
///
/// ## 方法
///
/// - `add_event_type(event_type: &str, payload: &str) -> Self`:
///   构建并返回一个新的 `NetworkEvent` 实例，设置事件类型和负载数据。
///
/// ## 注意事项
///
/// `NetworkEvent` 结构体旨在简化事件的创建和传递。使用 `NetworkEvent` 可以确保事件的数据格式统一，便于服务器端的解析和处理。
///
/// 客户端在构建 `NetworkEvent` 时，需要确保提供的 `event_type` 是有效的，并且 `payload` 是与该事件类型匹配的有效数据。



use crate::common_infrastructure::datafeed::event::MarketEvent;
use crate::common_infrastructure::order::{Order, RequestCancel, RequestOpen};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;
use crate::sandbox::sandbox_client::SandBoxClientEvent;
use serde::Deserialize;
use tokio::sync::oneshot;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct NetworkEvent {
    pub event_type: String,
    pub payload: String, // TODO 解析payload的方法未实现。
    pub timestamp: i64, // UNIX 时间戳
    pub source: String, // 事件来源
    pub destination: String, // 事件目的地
    pub event_id: String, // 唯一事件 ID
    pub version: String, // 事件版本号
    pub correlation_id: Option<String>, // 关联 ID (可选)
    pub priority: Option<u8>, // 优先级 (可选)
    pub retry_count: u8, // 重试次数
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