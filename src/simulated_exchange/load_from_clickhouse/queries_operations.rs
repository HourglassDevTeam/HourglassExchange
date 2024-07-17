// NOTE this module is previously built and imported into the main project as a dependency.
//      upon completion the following code should be deleted and external identical code should be used instead.

use std::sync::Arc;
use async_stream::stream;
pub use clickhouse::{
    Client,
    error::Result, Row,
};
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::error::ExecutionError;
use crate::simulated_exchange::{utils::chrono_operations::extract_date, ws_trade::WsTrade};


pub struct ClickHouseClient {
    client: Arc<Client>,  // Using Arc to safely share and own the client across contexts
}

impl ClickHouseClient {
    pub fn new() -> Self {
        let client = Client::default()
            .with_url("http://localhost:8123")
            .with_user("default")
            .with_password("");

        println!("[AlgoBacktest] : 连接到 ClickHouse 服务器成功。");

        Self { client: Arc::new(client) }  // Wrap the client in Arc
    }
}
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct TradeDataFromClickhouse
{
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
}

impl ClickHouseClient
{
    fn construct_table_name(&self, exchange: &str, instrument: &str, channel: &str, date: &str, base: &str, quote: &str) -> String
    {
        match exchange {
            | "binance" => format!("{}_{}_{}_{}_{}",
                                   exchange,
                                   instrument,
                                   channel,
                                   date.replace("-", "_"),
                                   base.to_uppercase() + &*quote.to_uppercase()),
            | "okex" => format!("{}_{}_{}_{}_{}_{}_{}",
                                exchange,
                                instrument,
                                channel,
                                date.replace("-", "_"),
                                base.to_uppercase(),
                                quote.to_uppercase(),
                                instrument.to_uppercase()),
            | _ => panic!("Unsupported exchange"),
        }
    }

    fn construct_database_name(&self, exchange: &str, instrument: &str, channel: &str) -> String
    {
        format!("{}_{}_{}", exchange, instrument, channel)
    }

    pub async fn get_table_names(&self, database: &str) -> Vec<String>
    {
        let table_names_query = format!("SHOW TABLES FROM {database}",);
        println!("{:?}", table_names_query);
        let result = self.client.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                          eprintln!("[AlgoBacktest] : ExecutionError loading table names: {:?}", e);
                                                                                          vec![]
                                                                                      });

        result
    }

    pub async fn get_tables_for_date(&self, database: &str, date: &str) -> Vec<String>
    {
        // 获取所有表名
        let table_names = self.get_table_names(database).await;

        // 筛选出指定日期的表名
        let tables_for_date: Vec<String> = table_names.into_iter()
                                                      .filter(|table_name| {
                                                          if let Some(table_date) = extract_date(table_name) {
                                                              table_date == date
                                                          }
                                                          else {
                                                              false
                                                          }
                                                      })
                                                      .collect();

        tables_for_date
    }

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<WsTrade>, ExecutionError>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp", full_table_path);
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let trade_datas = self.client.query(&query).fetch_all::<TradeDataFromClickhouse>().await.or_else(|e| Err(ExecutionError::InternalError(format!("Failed query: {}", e))))?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<WsTrade, ExecutionError>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp DESC LIMIT 1", full_table_path);
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let trade_data = self.client.query(&query).fetch_one::<TradeDataFromClickhouse>().await.or_else(|e| Err(ExecutionError::InternalError(format!("Failed query: {}", e))))?;
        Ok(WsTrade::from(trade_data))
    }

    pub async fn query_union_table(client: &ClickHouseClient, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<WsTrade>, ExecutionError>
    {
        let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
        let database = format!("{}_{}_{}", exchange, instrument, channel);
        let query = format!("SELECT * FROM {}.{}", database, table_name);
        println!("[AlgoBacktest] : Executing query: {}", query);
        let trade_datas = client.client.query(&query).fetch_all::<TradeDataFromClickhouse>().await.or_else(|e| Err(ExecutionError::InternalError(format!("Failed query: {}", e))))?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub fn query_union_table_batched<'a>(&'a self, exchange: &'a str, instrument: &'a str, channel: &'a str, date: &'a str)
                                         -> impl Stream<Item = Result<Vec<WsTrade>, ExecutionError>> + 'a
    {
        stream! {
            let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
            let database = format!("{}_{}_{}", exchange, instrument, channel);
            let mut offset = 0;
            let limit = 100000;

            loop {
                let query = format!(
                    "SELECT symbol, side, price, timestamp FROM {}.{} LIMIT {} OFFSET {}",
                    database, table_name, limit, offset
                );
                println!("[AlgoBacktest] : Executing query: {}", query);

                match self.client.query(&query).fetch_all::<TradeDataFromClickhouse>().await.or_else( |e| Err(ExecutionError::InternalError(format!("Failed query: {}", e)))) {
                    Ok(trade_datas) => {
                        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
                        let batch_size = ws_trades.len();
                        yield Ok(ws_trades);

                        if batch_size < limit {
                            break;
                        }

                        offset += limit;
                    },
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        }
    }
}
