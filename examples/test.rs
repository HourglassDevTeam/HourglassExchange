use std::sync::Arc;

use unilink_execution::simulated::clickhouse_api::queries_operations::{ClickHouseClient, ClickhouseTrade};

#[tokio::main]
async fn main()
{
    let client = Arc::new(ClickHouseClient::new());

    // 测试查询
    let test_query = "SELECT symbol, side, price, timestamp FROM binance_futures_trades.binance_futures_trades_union_2024_03_03 ORDER BY timestamp LIMIT 1000000 OFFSET 0";
    match client.client.read().await.query(test_query).fetch_all::<ClickhouseTrade>().await {
        | Ok(trade_datas) => {
            println!("Query succeeded: fetched {} rows", trade_datas.len());
        }
        | Err(e) => {
            eprintln!("Failed test query: {}", e);
            return;
        }
    };
}
