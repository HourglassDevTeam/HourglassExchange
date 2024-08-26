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
/// use std::net::Ipv4Addr;
/// use serde_json::json;
/// use uuid::Uuid;
/// use unilink_execution::common_infrastructure::event::ClientOrderId;
/// use unilink_execution::common_infrastructure::instrument::Instrument;
/// use unilink_execution::common_infrastructure::instrument::kind::InstrumentKind;
/// use unilink_execution::common_infrastructure::order::{Order, OrderExecutionType, RequestOpen};
/// use unilink_execution::common_infrastructure::Side;
/// use unilink_execution::Exchange;
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
///             exchange: Exchange::Binance, // 交易所名称
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
///     // 3. 定义 source 和 destination 为 IP 地址
///     let source = Ipv4Addr::new(192, 168, 110, 95).to_string(); // 假设这是客户端的 IP
///     let destination = Ipv4Addr::new(192, 168, 110, 130).to_string(); // 假设这是服务器的 IP
///
///     // 4. 创建 NetworkEvent
///     NetworkEvent {
///         event_type: event_type.to_string(),
///         payload,
///         timestamp: chrono::Utc::now().timestamp(),
///         source,
///         destination,
///         event_id: uuid::Uuid::new_v4().to_string(),
///         // 其他字段可以根据需要添加，例如 `version`, `correlation_id`, `priority`, `retry_count` 等
///     }
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_infrastructure::instrument::Instrument;
    use crate::common_infrastructure::order::{Order, OrderExecutionType, RequestOpen};
    use crate::common_infrastructure::Side;
    use crate::Exchange;
    use std::net::Ipv4Addr;
    use uuid::Uuid;
    use crate::common_infrastructure::event::ClientOrderId;
    use crate::common_infrastructure::instrument::kind::InstrumentKind;

    /// 测试 `NetworkEvent` 的创建和有效性
    #[test]
    fn test_create_open_orders_event() {
        // 1. 确定事件类型
        let event_type = "OpenOrders";

        // 2. 构建 payload
        let orders = vec![
            Order {
                kind: OrderExecutionType::Limit,   // 订单类型，例如限价单
                exchange: Exchange::Binance, // 交易所名称
                instrument: Instrument::new("BTC", "USDT", InstrumentKind::Perpetual), // 交易对
                client_ts: chrono::Utc::now().timestamp_millis(), // 客户端下单时间戳
                client_order_id: ClientOrderId(Uuid::new_v4()), // 客户端订单 ID
                side: Side::Buy, // 买卖方向
                state: RequestOpen {
                    reduce_only: false, // 非减仓订单
                    price: 50000.0,      // 下单价格
                    size: 1.0,           // 下单数量
                },
            }
        ];

        // 序列化 orders 为 JSON 字符串
        let payload = serde_json::to_string(&orders).expect("Failed to serialize orders");

        // 3. 定义 source 和 destination 为 IP 地址
        let source = Ipv4Addr::new(192, 168, 110, 95).to_string(); // 假设这是客户端的 IP
        let destination = Ipv4Addr::new(192, 168, 110, 130).to_string(); // 假设这是服务器的 IP

        // 4. 创建 NetworkEvent
        let network_event = NetworkEvent {
            event_type: event_type.to_string(),
            payload: payload.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            source,
            destination,
            event_id: Uuid::new_v4().to_string(),
        };

        // 验证 `NetworkEvent` 的各个字段
        assert_eq!(network_event.event_type, event_type);
        assert_eq!(network_event.payload, payload);
        assert_eq!(network_event.source, "192.168.110.95");
        assert_eq!(network_event.destination, "192.168.110.130");

        // 验证事件的解析功能
        let parsed_event = network_event.parse_payload();
        assert!(parsed_event.is_ok());

        if let Ok(SandBoxClientEvent::OpenOrders((parsed_orders, _))) = parsed_event {
            assert_eq!(parsed_orders.len(), 1);
            assert_eq!(parsed_orders[0].kind, OrderExecutionType::Limit);
            assert_eq!(parsed_orders[0].exchange, Exchange::Binance);
            assert_eq!(parsed_orders[0].instrument.base, "BTC".into());
            assert_eq!(parsed_orders[0].instrument.quote, "USDT".into());
            assert_eq!(parsed_orders[0].state.price, 50000.0);
        } else {
            panic!("Failed to parse OpenOrders event");
        }
    }

    /// 测试解析未知事件类型
    #[test]
    fn test_unknown_event_type() {
        let network_event = NetworkEvent {
            event_type: "UnknownEvent".to_string(),
            payload: "".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            source: "192.168.110.95".to_string(),
            destination: "192.168.110.130".to_string(),
            event_id: Uuid::new_v4().to_string(),
        };

        let parsed_event = network_event.parse_payload();
        assert!(parsed_event.is_err());
    }
}