use rayon::iter::ParallelIterator;
/// NOTE 目前表名的构建方式都以`Tardis API`的`Binance`数据为基础。可能并不适用于其他交易所。日后**必须**扩展。
use std::sync::Arc;

use async_stream::stream;
use chrono::NaiveDate;
use clickhouse::query::RowCursor;
pub use clickhouse::{
    error::{Error, Result},
    Client, Row,
};
use futures_core::Stream;
use rayon::iter::IntoParallelRefIterator;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver},
    RwLock,
};

use crate::{
    common_infrastructure::{datafeed::event::MarketEvent, Side},
    error::ExecutionError,
    sandbox::{
        clickhouse_api::{datatype::clickhouse_trade_data::ClickhousePublicTrade, query_builder::ClickHouseQueryBuilder},
        utils::chrono_operations::extract_date,
        ws_trade::parse_base_and_quote,
    },
};

pub struct ClickHouseClient
{
    pub client: Arc<RwLock<Client>>,
}

impl Default for ClickHouseClient
{
    fn default() -> Self
    {
        Self::new()
    }
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
    pub fn construct_table_name(&self, exchange: &str, instrument: &str, channel: &str, date: &str, base: &str, quote: &str) -> String
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

    pub fn construct_union_table_name(&self, exchange: &str, instrument: &str, channel: &str, date: &str) -> String
    {
        format!("{}_{}_{}_union_{}",
                exchange,
                instrument,
                channel,
                date.replace("-", "_") /* Replace dashes with underscores for valid table names */)
    }

    pub fn construct_database_name(&self, exchange: &str, instrument: &str, channel: &str) -> String
    {
        format!("{}_{}_{}", exchange, instrument, channel)
    }

    pub async fn get_table_names(&self, database: &str) -> Vec<String>
    {
        let table_names_query = format!("SHOW TABLES FROM {database}",);
        println!("[UniLinkExecution] : Trying to retrieve table names within the database : {:?}", table_names_query);
        self.client.read().await.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                          eprintln!("[UniLinkExecution] : Error loading table names: {:?}", e);

                                                                                          vec![]
                                                                                      })
    }

    pub async fn get_union_table_names(&self, database: &str) -> Vec<String>
    {
        let table_names_query = format!("SHOW TABLES FROM {database} LIKE '%union%'",);
        println!("[UniLinkExecution] : Trying to retrieve table names within the database that contain 'union': {:?}",
                 table_names_query);
        self.client.read().await.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                          eprintln!("[UniLinkExecution] : Error loading table names: {:?}", e);

                                                                                          vec![]
                                                                                      })
    }

    pub async fn get_tables_for_date(&self, table_names: &[String], date: &str) -> Vec<String>
    {
        // 筛选出指定日期的表名
        let tables_for_date: Vec<String> = table_names.par_iter()
                                                      .filter(|table_name| {
                                                          if let Some(table_date) = extract_date(table_name) {
                                                              table_date == date
                                                          }
                                                          else {
                                                              false
                                                          }
                                                      })
                                                      .cloned()
                                                      .collect();
        tables_for_date
    }

    pub async fn create_unioned_table_for_date(&self,
                                               database: &str,
                                               new_table_name: &str,
                                               table_names: &[String],
                                               report_progress: bool /* 新增参数，用于控制是否启用进度汇报 */)
                                               -> Result<(), Error>
    {
        // 构建UNION ALL查询
        let mut queries = Vec::new();
        let total_tables = table_names.len();

        for (i, table_name) in table_names.iter().enumerate() {
            let select_query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount") // Select required fields
                                                            .from(database, table_name) // Format the table name with database
                                                            .build(); // Build the individual query

            queries.push(select_query);

            // 如果启用进度汇报，每处理完一个表就汇报一次进度
            if report_progress {
                let progress = ((i + 1) as f64 / total_tables as f64) * 100.0;
                println!("Progress: Processed {} / {} tables ({:.2}%)", i + 1, total_tables, progress);

                // 模拟延迟以模拟长时间运行任务的进度汇报
                // sleep(Duration::from_millis(500)).await;
            }
        }

        let union_all_query = queries.join(" UNION ALL ");

        // 假设你要创建的表使用MergeTree引擎并按timestamp排序

        // NOTE : This chunk is a query without any partition
        // let final_query = format!("CREATE TABLE {}.{} ENGINE = MergeTree() ORDER BY timestamp AS {}",
        //                           database, new_table_name, union_all_query);
        //

        // NOTE : A partition syntax has been added to this query.
        let final_query = format!(
                                  "CREATE TABLE {}.{} ENGINE = MergeTree() \
            PARTITION BY toYYYYMMDD(toDate(timestamp)) \
            ORDER BY timestamp AS {}",
                                  database, new_table_name, union_all_query
        );

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

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<ClickhousePublicTrade>, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        // let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .build();

        println!("[UniLinkExecution] : Constructed query {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await?;
        Ok(trade_datas)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<ClickhousePublicTrade, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        // let full_table_path = format!("{}.{}", database_name, table_name);
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .limit(1)
                                                 .build();
        println!("[UniLinkExecution] : Constructed query :  {}", query);
        let trade_data = self.client.read().await.query(&query).fetch_one::<ClickhousePublicTrade>().await?;
        Ok(trade_data)
    }

    pub async fn query_unioned_trade_table(&self, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<ClickhousePublicTrade>, Error>
    {
        let table_name = self.construct_union_table_name(exchange, instrument, channel, date);
        let database = self.construct_database_name(exchange, instrument, "trades");
        let query = format!("SELECT * FROM {}.{} ORDER BY timestamp", database, table_name);
        println!("[UniLinkExecution] : Executing query: {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await?;
        Ok(trade_datas)
    }

    pub async fn query_unioned_trade_table_batched<'a>(&'a self,
                                                       exchange: &'a str,
                                                       instrument: &'a str,
                                                       channel: &'a str,
                                                       date: &'a str,
                                                       batch_size: usize)
                                                       -> impl Stream<Item = MarketEvent<ClickhousePublicTrade>> + 'a
    {
        stream! {
            let table_name = self.construct_union_table_name(exchange, instrument, channel, date);
            let database = self.construct_database_name(exchange, instrument, "trades");
            let mut offset = 0;

            loop {
                 let query = ClickHouseQueryBuilder::new()
                        .select("*")
                        .from(&database, &table_name)
                        .limit(batch_size)
                        .offset(offset)
                .order("timestamp",Some("DESC"))
                        .build();
                println!("[UniLinkExecution] : Executing query: {}", query);

                match self.client.read().await.query(&query).fetch_all::<ClickhousePublicTrade>().await {
                    Ok(trade_datas) => {
                        for trade_data in &trade_datas {
                            let (base, quote) = parse_base_and_quote(&trade_data.symbol);
                            let market_event = MarketEvent::from_swap_trade_clickhouse(trade_data.clone(), base, quote);
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

    pub async fn query_unioned_trade_table_batched_between_dates(self: Arc<Self>,
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
                let table_name = self.construct_union_table_name(&exchange, &instrument, &channel, &date);
                let database = self.construct_database_name(&exchange, &instrument, &channel);
                let mut offset = 0;

                loop {
                    let query = ClickHouseQueryBuilder::new().select("*")
                                                             .from(&database, &table_name)
                                                             .order("timestamp", Some("ASC"))
                                                             .limit(batch_size)
                                                             .offset(offset)
                                                             .build();
                    println!("[UniLinkExecution] : Executing query: {}", query);

                    let client = client.read().await;
                    match client.query(&query).fetch_all::<ClickhousePublicTrade>().await {
                        | Ok(trade_datas) => {
                            for trade_data in &trade_datas {
                                let (base, quote) = parse_base_and_quote(&trade_data.symbol);
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

    pub async fn cursor_public_trades<'a>(&'a self,
                                          exchange: &'a str,
                                          instrument: &'a str,
                                          date: &'a str,
                                          base: &'a str,
                                          quote: &'a str)
                                          -> Result<RowCursor<ClickhousePublicTrade>>
    {
        // 构造数据库名称和表名称
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);

        // 使用 ClickHouseQueryBuilder 构造查询语句
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .build();

        println!("[UniLinkExecution] : Constructed query {}", query);

        // 获取 ClickHouse 客户端的只读引用
        let client_ref = self.client.read().await;

        // 执行查询并获取游标
        client_ref.query(&query).fetch::<ClickhousePublicTrade>()
    }

    pub async fn cursor_unioned_public_trades(&self, exchange: &str, instrument: &str, date: &str) -> Result<RowCursor<ClickhousePublicTrade>>
    {
        // 构造数据库名称和表名称
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_union_table_name(exchange, instrument, "trades", date);

        // 使用 ClickHouseQueryBuilder 构造查询语句
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .build();

        println!("[UniLinkExecution] : Constructed query {}", query);

        // 获取 ClickHouse 客户端的只读引用
        let client_ref = self.client.read().await;

        // 执行查询并获取游标
        client_ref.query(&query).fetch::<ClickhousePublicTrade>()
    }

    pub async fn optimize_table(&self, table_path: &str) -> Result<(), Error>
    {
        let optimize_query = format!("OPTIMIZE TABLE {}", table_path);
        println!("[UniLinkExecution] : Sending optimize query for table: {}", table_path);
        // 执行优化查询
        self.client.read().await.query(&optimize_query).execute().await?;
        println!("[UniLinkExecution] : Table {} has been optimized.", table_path);
        Ok(())
    }

    // Method to optimize only "union" tables within a date range
    pub async fn optimize_union_tables_in_date_range(&self, exchange: &str, instrument: &str, channel: &str, mut start_date: NaiveDate, end_date: NaiveDate) -> Result<(), Error>
    {
        let database = format!("{}_{}_{}", exchange, instrument, channel);

        let total_days = (end_date - start_date).num_days() + 1;
        let mut processed_days = 0;

        while start_date <= end_date {
            let date_str = start_date.format("%Y_%m_%d").to_string();
            processed_days += 1;

            let table_names = self.get_table_names(&database).await;
            let mut processed_tables = 0;

            // Iterate over table names and filter for "union" tables
            for table_name in &table_names {
                if table_name.contains("union") {
                    let table_path = format!("{}.{}", database, table_name);
                    println!("Optimizing table: {}", table_path);

                    if let Err(e) = self.optimize_table(&table_path).await {
                        eprintln!("Error optimizing table {}: {}", table_path, e);
                    }

                    processed_tables += 1;
                }
            }

            let progress = (processed_days as f64 / total_days as f64) * 100.0;
            println!("Date: {} - Union tables processed: {}/{}, Total progress: {:.2}%",
                     date_str,
                     processed_tables,
                     table_names.len(),
                     progress);

            start_date += chrono::Duration::days(1);
        }

        println!("Union table optimization is complete for {} days.", total_days);
        Ok(())
    }
}
