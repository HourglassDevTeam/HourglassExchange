use serde::{Deserialize, Serialize};

use crate::{
    common::{
        balance::TokenBalance,
        order::{
            states::{
                cancelled::Cancelled,
                fills::{FullyFill, PartialFill},
                open::Open,
            },
            Order,
        },
        position::AccountPositions,
        trade::ClientTrade,
    },
    sandbox::account::account_config::AccountConfig,
    Exchange,
};

/// NOTE: 如果需要记录交易所的时间戳，可以再添加一个专门的字段来表示交易所的时间，例如：    pub exchange_ts: DateTime<Utc> or i64
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountEvent
{
    pub exchange_timestamp: i64, // 交易所发送事件的时间,
    pub exchange: Exchange,      // 目标和源头交易所
    pub kind: AccountEventKind,  // 事件类型
}

/// 定义账户事件[`AccountEvent`]的类型。
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AccountEventKind
{
    // Order Events
    OrdersOpen(Vec<Order<Open>>),
    OrdersNew(Vec<Order<Open>>),
    OrdersCancelled(Vec<Order<Cancelled>>),
    OrdersFilled(Vec<Order<FullyFill>>),
    OrdersPartiallyFilled(Vec<Order<PartialFill>>),
    Balance(TokenBalance),
    Trade(ClientTrade),
    Balances(Vec<TokenBalance>),
    Positions(AccountPositions),
    AccountConfig(AccountConfig),
    // OrderBookUpdate(OrderBookUpdate),
    // MarketStatus(MarketStatus),
    // MarginUpdate(MarginUpdate),
    // Transfer(Transfer),
    // Deposit(Deposit),
    // Withdrawal(Withdrawal),
}


#[cfg(test)]
mod tests
{
    use super::*;
    use crate::common::{balance::Balance, token::Token};
    use uuid::Uuid;
    use crate::common::order::id::cid::ClientOrderId;

    #[test]
    fn account_event_should_serialize_and_deserialize_correctly()
    {
        let event = AccountEvent { exchange_timestamp: 1627845123,
                                   exchange: Exchange::Binance,
                                   kind: AccountEventKind::OrdersOpen(vec![]) };
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AccountEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn account_event_kind_should_serialize_and_deserialize_correctly()
    {
        let kind = AccountEventKind::OrdersNew(vec![]);
        let serialized = serde_json::to_string(&kind).unwrap();
        let deserialized: AccountEventKind = serde_json::from_str(&serialized).unwrap();
        assert_eq!(kind, deserialized);
    }

    #[test]
    fn client_order_id_should_format_correctly() {
        let client_order_id = ClientOrderId(Some(Uuid::new_v4().to_string())); // 直接生成一个字符串
        assert_eq!(format!("{}", client_order_id), client_order_id.0.clone().unwrap());
    }

    #[test]
    fn account_event_kind_should_handle_all_variants()
    {
        let kinds = vec![AccountEventKind::OrdersOpen(vec![]),
                         AccountEventKind::OrdersNew(vec![]),
                         AccountEventKind::OrdersCancelled(vec![]),
                         AccountEventKind::OrdersFilled(vec![]),
                         AccountEventKind::OrdersPartiallyFilled(vec![]),
                         AccountEventKind::Balance(TokenBalance::new(Token::from("BTC"), Balance::new(100.0, 50.0, 20000.0))),
                         // AccountEventKind::Trade(ClientTrade::default()),
                         AccountEventKind::Balances(vec![]),
                         /* AccountEventKind::Positions(AccountPositions::default()),
                          * AccountEventKind::AccountConfig(AccountConfig::default()), */];
        for kind in kinds {
            let serialized = serde_json::to_string(&kind).unwrap();
            let deserialized: AccountEventKind = serde_json::from_str(&serialized).unwrap();
            assert_eq!(kind, deserialized);
        }
    }
}
