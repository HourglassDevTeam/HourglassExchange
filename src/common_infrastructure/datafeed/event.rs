use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{common_infrastructure::instrument::Instrument, Exchange};

// 定义一个泛型结构体 MarketEvent，包含各种交易市场事件信息
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<Data>
{
    pub exchange_time: i64,        // 交易所时间戳
    pub received_time: i64,        // 接收到数据的时间戳
    pub exchange: Exchange, // 交易所信息
    pub instrument: Instrument,    // 交易工具信息
    pub kind: Data,                // 事件的具体类型 `WsTrade` / `ClickhousePublicTrade` / `ClientTrade` 等
}
