// NOTE this module is previously built and imported into the main project as a dependency.
//      upon completion the following code should be deleted and external identical code should be used instead.

use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use chrono::NaiveDate;
pub use clickhouse::{
    Client,
    error::{Error, Result}, Row,
};
use futures_core::Stream;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver},
    RwLock,
};
use tokio::time::sleep;

use crate::{
    common_infrastructure::{datafeed::event::MarketEvent, Side},
    error::ExecutionError,
    sandbox::{
        utils::chrono_operations::extract_date,
        ws_trade::{parse_base_and_quote, WsTrade},
    },
};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;

pub struct ClickHouseClient
{
    pub client: Arc<RwLock<Client>>,
}
impl ClickHouseClient
{
    pub fn new() -> Self
    {
        let client = Client::default().with_url("http://localhost:8123").with_user("default").with_password("");

        println!("[UniLinkExecution] : Successfully connected to the ClickHouse server.");

        Self { client: Arc::new(RwLock::new(client)) }
    }
}


impl ClickhousePublicTrade
{
    /// 将 `side` 字符串解析为 `Side` 枚举
    pub fn parse_side(&self) -> Side
    {
        match self.side.as_str() {
            | "Buy" => Side::Buy,
            | "Sell" => Side::Sell,
            | _ => panic!("Unknown side: {}", self.side),
        }
    }
}
// 手动实现 Eq 和 PartialEq 特性
impl PartialEq for ClickhousePublicTrade
{
    fn eq(&self, other: &Self) -> bool
    {
        self.timestamp == other.timestamp
    }
}

impl Eq for ClickhousePublicTrade {}

// 手动实现 PartialOrd 和 Ord 特性
impl PartialOrd for ClickhousePublicTrade
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        Some(self.cmp(other))
    }
}

impl Ord for ClickhousePublicTrade
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.timestamp.cmp(&other.timestamp)
    }
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
        let result = self.client.read().await.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                                       eprintln!("[UniLinkExecution] : Error loading table names: {:?}", e);
                                                                                                       vec![]
                                                                                                   });

        result
    }

    pub async fn get_tables_for_date(&self, table_names: &[String], date: &str) -> Vec<String> {
        // 筛选出指定日期的表名
        let tables_for_date: Vec<String> = table_names
            .iter()
            .filter(|table_name| {
                if let Some(table_date) = extract_date(table_name) {
                    table_date == date
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        tables_for_date
    }

    pub async fn query_all_trades_on_date(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<WsTrade>, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT * FROM {} ORDER BY timestamp", full_table_path);
        println!("[UniLinkExecution] : Constructed query :  {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }



    pub async fn create_unioned_tables_for_date(
        &self,
        database: &str,
        new_table_name: &str,
        table_names: &Vec<String>,
        report_progress: bool // 新增参数，用于控制是否启用进度汇报
    ) -> Result<(), Error> {
        // 构建UNION ALL查询
        let mut queries = Vec::new();
        let total_tables = table_names.len();

        for (i, table_name) in table_names.iter().enumerate() {
            let query = format!("SELECT symbol, side, price, timestamp,amount FROM {}.{}", database, table_name);
            queries.push(query);

            // 如果启用进度汇报，每处理完一个表就汇报一次进度
            if report_progress {
                let progress = ((i + 1) as f64 / total_tables as f64) * 100.0;
                println!("Progress: Processed {} / {} tables ({:.2}%)", i + 1, total_tables, progress);

                // 模拟延迟以模拟长时间运行任务的进度汇报
                sleep(Duration::from_millis(500)).await;
            }
        }

        let union_all_query = queries.join(" UNION ALL ");

        // 假设你要创建的表使用MergeTree引擎并按timestamp排序
        let final_query = format!("CREATE TABLE {}.{} ENGINE = MergeTree() ORDER BY timestamp AS {}",
                                  database, new_table_name, union_all_query);

        if report_progress {
            println!("[UniLinkExecution] : Successfully constructed the final query.");
        }

        // 执行创建新表的查询
        self.client.read().await.query(&final_query).execute().await?;

        if report_progress {
            println!("[UniLinkExecution] : Table {}.{} created successfully.", database, new_table_name);
        }

        Ok(())
    }

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str,date: &str, base: &str, quote: &str) -> Result<Vec<WsTrade>, Error> {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        // let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp", full_table_path);
        println!("[UniLinkExecution] : Constructed query {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<WsTrade, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT * FROM {} ORDER BY timestamp DESC LIMIT 1", full_table_path);
        println!("[UniLinkExecution] : Constructed query :  {}", query);
        let trade_data = self.client.read().await.query(&query).fetch_one::<ClickhousePublicTrade>().await?;
        Ok(WsTrade::from(trade_data))
    }

    pub async fn query_unioned_trade_table(&self, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<WsTrade>, Error>
    {
        let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
        let database = format!("{}_{}_{}", exchange, instrument, channel);
        let query = format!("SELECT * FROM {}.{} ORDER BY timestamp", database, table_name);
        println!("[UniLinkExecution] : Executing query: {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub fn query_unioned_trade_table_batched<'a>(&'a self,
                                                 exchange: &'a str,
                                                 instrument: &'a str,
                                                 channel: &'a str,
                                                 date: &'a str,
                                                 batch_size: usize)
                                                 -> impl Stream<Item = MarketEvent<ClickhousePublicTrade>> + 'a
    {
        stream! {
            let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
            let database = format!("{}_{}_{}", exchange, instrument, channel);
            let mut offset = 0;

            loop {
                let query = format!(
                    "SELECT * FROM {}.{} LIMIT {} OFFSET {} ORDER BY timestamp",
                    database, table_name, batch_size, offset
                );
                println!("[UniLinkExecution] : Executing query: {}", query);

                match self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await {
                    Ok(trade_datas) => {
                        for trade_data in &trade_datas {
                            let (base, quote) = parse_base_and_quote(&trade_data.basequote);
                            let market_event = MarketEvent::from_swap_trade_clickhouse(trade_data.clone(),base,quote);
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

    pub async fn query_unioned_trade_table_batched_for_dates(self: Arc<Self>,
                                                             exchange: &str,
                                                             instrument: &str,
                                                             channel: &str,
                                                             start_date: &str,
                                                             end_date: &str,
                                                             batch_size: usize)
                                                             -> Result<UnboundedReceiver<MarketEvent<ClickhousePublicTrade>>, ExecutionError>
    {
        let (tx, rx) = unbounded_channel();
        // 处理 start_date 解析，并映射到 ExecutionError
        let start_date = NaiveDate::parse_from_str(start_date, "%Y_%m_%d").map_err(|e| ExecutionError::InvalidTradingPair(format!("Invalid start date format: {}", e)))?;

        // 处理 end_date 解析，并映射到 ExecutionError
        let end_date = NaiveDate::parse_from_str(end_date, "%Y_%m_%d").map_err(|e| ExecutionError::InvalidTradingPair(format!("Invalid end date format: {}", e)))?;

        let mut current_date = start_date;

        let client = Arc::clone(&self.client);
        let exchange = exchange.to_owned();
        let instrument = instrument.to_owned();
        let channel = channel.to_owned();

        tokio::spawn(async move {
            while current_date <= end_date {
                let date = current_date.format("%Y_%m_%d").to_string();
                let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
                let database = format!("{}_{}_{}", exchange, instrument, channel);
                let mut offset = 0;

                loop {
                    let query = format!("SELECT * FROM {}.{} ORDER BY timestamp LIMIT {} OFFSET {}", database, table_name, batch_size, offset);
                    println!("[UniLinkExecution] : Executing query: {}", query);

                    let client = client.read().await;
                    match client.query(&query).fetch_all::<ClickhousePublicTrade>().await {
                        | Ok(trade_datas) => {
                            for trade_data in &trade_datas {
                                let (base, quote) = parse_base_and_quote(&trade_data.basequote);
                                let market_event = MarketEvent::from_swap_trade_clickhouse(trade_data.clone(), base, quote);
                                if tx.send(market_event).is_err() {
                                    eprintln!("Failed to send market event");
                                    return;
                                }
                            }

                            if trade_datas.len() < batch_size {
                                break;
                            }

                            offset += batch_size;
                        }
                        | Err(e) => {
                            eprintln!("Failed query: {}", e);
                            eprintln!("Query: {}", query);
                            return;
                        }
                    }
                }

                current_date += chrono::Duration::days(1);
            }
        });

        Ok(rx)
    }
}
