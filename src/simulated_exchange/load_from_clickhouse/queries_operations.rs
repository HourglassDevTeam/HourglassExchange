// NOTE this module is previously built and imported into the main project as a dependency.
//      upon completion the following code should be deleted and external identical code should be used instead.

use async_stream::stream;
use chrono::{Duration, NaiveDate};
pub use clickhouse::{
    error::{Error, Result},
    Client, Row,
};
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::{
    common_skeleton::{datafeed::event::MarketEvent},
    simulated_exchange::{
        utils::chrono_operations::extract_date,
        ws_trade::{parse_base_and_quote, WsTrade},
    },
    Exchange,
};

pub struct ClickHouseClient
{
    pub client: Client,
}

impl ClickHouseClient
{
    pub fn new() -> Self
    {
        let client = Client::default().with_url("http://localhost:8123").with_user("default").with_password("");

        println!("[UnilinkExecution] : 连接到 ClickHouse 服务器成功。");

        Self { client }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ClickhouseTrade
{
    pub basequote: String,
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
                                                                                          eprintln!("[UnilinkExecution] : Error loading table names: {:?}", e);
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

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<WsTrade>, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp", full_table_path);
        println!("[UnilinkExecution] : 查询SQL语句 {}", query);
        let trade_datas = self.client.query(&query).fetch_all::<ClickhouseTrade>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<WsTrade, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp DESC LIMIT 1", full_table_path);
        println!("[UnilinkExecution] : 查询SQL语句 {}", query);
        let trade_data = self.client.query(&query).fetch_one::<ClickhouseTrade>().await?;
        Ok(WsTrade::from(trade_data))
    }

    pub async fn query_unioned_trade_table(client: &ClickHouseClient, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<WsTrade>, Error>
    {
        let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
        let database = format!("{}_{}_{}", exchange, instrument, channel);
        let query = format!("SELECT * FROM {}.{}", database, table_name);
        println!("[UnilinkExecution] : Executing query: {}", query);
        let trade_datas = client.client.query(&query).fetch_all::<ClickhouseTrade>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub fn query_unioned_trade_table_batched<'a>(&'a self,
                                                 exchange: &'a str,
                                                 instrument: &'a str,
                                                 channel: &'a str,
                                                 date: &'a str,
                                                 batch_size: usize)
                                                 -> impl Stream<Item = MarketEvent<ClickhouseTrade>> + 'a
    {
        stream! {
            let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
            let database = format!("{}_{}_{}", exchange, instrument, channel);
            let mut offset = 0;

            loop {
                let query = format!(
                    "SELECT symbol, side, price, timestamp FROM {}.{} LIMIT {} OFFSET {}",
                    database, table_name, batch_size, offset
                );
                println!("[UnilinkExecution] : Executing query: {}", query);

                match self.client.query(&query).fetch_all::<ClickhouseTrade>().await {
                    Ok(trade_datas) => {
                        for trade_data in &trade_datas {
                            let (base, quote) = parse_base_and_quote(&trade_data.basequote);
                            let market_event = MarketEvent::from_swap_trade_clickhouse(trade_data.clone(),base,quote,Exchange::from(exchange.to_string()));
                            yield market_event;
                        }

                        if trade_datas.len() < batch_size {
                            break;
                        }

                        offset += batch_size;
                    },
                    Err(e) => {
                        eprintln!("Failed query: {}", e);
                        break;
                    }
                }
            }
        }
    }

    pub fn query_unioned_trade_table_batched_for_dates<'a>(&'a self,
                                                           exchange: &'a str,
                                                           instrument: &'a str,
                                                           channel: &'a str,
                                                           start_date: &'a str,
                                                           end_date: &'a str,
                                                           batch_size: usize)
                                                           -> impl Stream<Item = MarketEvent<ClickhouseTrade>> + 'a
    {
        stream! {
            let start_date = NaiveDate::parse_from_str(start_date, "%Y_%m_%d").expect("Invalid start date format");
            let end_date = NaiveDate::parse_from_str(end_date, "%Y_%m_%d").expect("Invalid end date format");
            let mut current_date = start_date;
            while current_date <= end_date {
                let date = current_date.format("%Y_%m_%d").to_string();
                let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
                let database = format!("{}_{}_{}", exchange, instrument, channel);
                let mut offset = 0;
                loop {
                    let query = format!(
                        "SELECT symbol, side, price, timestamp FROM {}.{} ORDER BY timestamp LIMIT {} OFFSET {} ",
                        database, table_name, batch_size, offset
                    );
                    println!("[UnilinkExecution] : Executing query: {}", query);

                    match self.client.query(&query).fetch_all::<ClickhouseTrade>().await {
                        Ok(trade_datas) => {
                            for trade_data in &trade_datas {
                                                            let (base, quote) = parse_base_and_quote(&trade_data.basequote);

                                let market_event = MarketEvent::from_swap_trade_clickhouse(trade_data.clone(),base, quote,Exchange::from(exchange.to_string()));
                                yield market_event;
                            }

                            if trade_datas.len() < batch_size {
                                break;
                            }

                            offset += batch_size;
                        },
                        Err(e) => {
                            eprintln!("Failed query: {}", e);
                            break;
                        }
                    }
                }

                current_date += Duration::days(1);
            }
        }
    }
}
