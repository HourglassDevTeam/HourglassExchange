use crate::hourglass_log::warn;
use crate::hourglass_log::info;
use chrono::NaiveDate;
use clickhouse::query::RowCursor;
pub use clickhouse::{
    error::{Error, Result},
    Client, Row,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
/// NOTE 目前表名的构建方式都以`Tardis API`的`Binance`数据为基础。可能并不适用于其他交易所。日后**必须**扩展。
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use crate::{
    common::Side,
    hourglass::{
        clickhouse_api::{datatype::clickhouse_trade_data::MarketTrade, query_builder::ClickHouseQueryBuilder},
        utils::chrono_operations::extract_date,
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
        info!("Successfully connected to the ClickHouse server.");
        Self { client: Arc::new(RwLock::new(client)) }
    }
}

impl MarketTrade
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
impl PartialEq for MarketTrade
{
    fn eq(&self, other: &Self) -> bool
    {
        self.timestamp == other.timestamp
    }
}

impl Eq for MarketTrade {}

// 手动实现 PartialOrd 和 Ord 特性
impl PartialOrd for MarketTrade
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        Some(self.cmp(other))
    }
}

impl Ord for MarketTrade
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
            | "binance" => format!("{}_{}_{}_{}_{}", exchange, instrument, channel, date.replace("-", "_"), base.to_uppercase() + &*quote.to_uppercase()),
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
        info!("Trying to retrieve table names within database : {:}", database);
        self.client.read().await.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                          warn!("Error loading table names: {:?}", e);

                                                                                          vec![]
                                                                                      })
    }

    pub async fn get_union_table_names(&self, database: &str) -> Vec<String>
    {
        let table_names_query = format!("SHOW TABLES FROM {database} LIKE '%union%'",);
        info!("Trying to retrieve table names within the database that contain 'union': {:?}", table_names_query);
        self.client.read().await.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
                                                                                          warn!("Error loading table names: {:?}", e);

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

    pub async fn create_unioned_table(&self,
                                      database: &str,
                                      new_table_name: &str,
                                      table_names: &[String],
                                      report_progress: bool /* 新增参数，用于控制是否启用进度汇报 */)
                                      -> Result<(), Error>
    {
        // 构建UNION ALL查询
        let queries = Arc::new(Mutex::new(Vec::new()));
        let total_tables = table_names.len();

        table_names.par_iter().enumerate().for_each(|(i, table_name)| {
                                              let select_query = ClickHouseQueryBuilder::new().select("exchange, symbol, id, side, price, timestamp, amount") // Select required fields
                                                                                              .from(database, table_name) // Format the table name with database
                                                                                              .build(); // Build the individual query

                                              let mut queries_lock = queries.lock().unwrap();
                                              queries_lock.push(select_query);

                                              // 如果启用进度汇报，每处理完一个表就汇报一次进度
                                              if report_progress {
                                                  let progress = ((i + 1) as f64 / total_tables as f64) * 100.0;
                                                  info!("Progress: Processed {} / {} tables ({:.2}%)", i + 1, total_tables, progress);
                                              }
                                          });

        let queries = Arc::try_unwrap(queries).expect("Failed to unwrap Arc").into_inner().unwrap();
        let union_all_query = queries.join(" UNION ALL ");

        // 假设你要创建的表使用MergeTree引擎并按timestamp排序 NOTE this ought to be replaced with ReplacingMergeTree Engine in due course.
        let final_query = format!(
                                  "CREATE TABLE {}.{} ENGINE = ReplacingMergeTree() \
        PARTITION BY toYYYYMMDD(toDate(timestamp)) \
        ORDER BY  (timestamp,id) AS {}",
                                  database, new_table_name, union_all_query
        );

        if report_progress {
            info!("Successfully constructed the final query.");
        }

        // 执行创建新表的查询
        self.client.read().await.query(&final_query).execute().await?;

        if report_progress {
            info!("Table {}.{} created successfully.", database, new_table_name);
        }

        Ok(())
    }

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<MarketTrade>, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        // let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .build();

        info!("Constructed query {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<MarketTrade>().await?;
        Ok(trade_datas)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<MarketTrade, Error>
    {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        // let full_table_path = format!("{}.{}", database_name, table_name);
        let query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .limit(1)
                                                 .build();
        info!("Constructed query :  {}", query);
        let trade_data = self.client.read().await.query(&query).fetch_one::<MarketTrade>().await?;
        Ok(trade_data)
    }

    pub async fn query_unioned_trade_table(&self, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<MarketTrade>, Error>
    {
        let table_name = self.construct_union_table_name(exchange, instrument, channel, date);
        let database = self.construct_database_name(exchange, instrument, "trades");
        let query = format!("SELECT exchange, symbol, side, price, timestamp, amount FROM {}.{} ORDER BY timestamp", database, table_name);
        info!("Executing query: {}", query);
        let trade_datas = self.client.read().await.query(&query).fetch_all::<MarketTrade>().await?;
        Ok(trade_datas)
    }

    pub async fn cursor_public_trades<'a>(&'a self, exchange: &'a str, instrument: &'a str, date: &'a str, base: &'a str, quote: &'a str) -> Result<RowCursor<MarketTrade>>
    {
        // 构造数据库名称和表名称
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);

        // 使用 ClickHouseQueryBuilder 构造查询语句
        let query = ClickHouseQueryBuilder::new().select("exchange, symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("DESC"))
                                                 .build();

        // info!("Constructed query {}", query);

        // 获取 ClickHouse 客户端的只读引用
        let client_ref = self.client.read().await;

        // 执行查询并获取游标
        client_ref.query(&query).fetch::<MarketTrade>()
    }

    pub async fn cursor_unioned_public_trades(&self, exchange: &str, instrument: &str, date: &str) -> Result<RowCursor<MarketTrade>>
    {
        // 构造数据库名称和表名称
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_union_table_name(exchange, instrument, "trades", date);

        // 使用 ClickHouseQueryBuilder 构造查询语句
        let query = ClickHouseQueryBuilder::new().select("exchange, symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("ASC"))
                                                 .build();

        info!("Constructed query {}", query);

        // 获取 ClickHouse 客户端的只读引用
        let client_ref = self.client.read().await;

        // 执行查询并获取游标
        client_ref.query(&query).fetch::<MarketTrade>()
    }

    pub async fn cursor_unioned_public_trades_for_test(&self, exchange: &str, instrument: &str, date: &str) -> Result<RowCursor<MarketTrade>>
    {
        // 构造数据库名称和表名称
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_union_table_name(exchange, instrument, "trades", date);

        // 使用 ClickHouseQueryBuilder 构造查询语句
        let query = ClickHouseQueryBuilder::new().select("exchange, symbol, side, price, timestamp, amount")
                                                 .from(&database_name, &table_name)
                                                 .order("timestamp", Some("ASC"))
                                                 .limit(8)
                                                 .build();

        info!("Constructed query {}", query);

        // 获取 ClickHouse 客户端的只读引用
        let client_ref = self.client.read().await;

        // 执行查询并获取游标
        client_ref.query(&query).fetch::<MarketTrade>()
    }

    pub async fn optimize_table(&self, table_path: &str) -> Result<(), Error>
    {
        let optimize_query = format!("OPTIMIZE TABLE {}", table_path);
        info!("Sending optimize query for table: {}", table_path);
        // 执行优化查询
        self.client.read().await.query(&optimize_query).execute().await?;
        info!("Table {} has been optimized.", table_path);
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
                    info!("Optimizing table: {}", table_path);

                    if let Err(e) = self.optimize_table(&table_path).await {
                        warn!("Error optimizing table {}: {}", table_path, e);
                    }

                    processed_tables += 1;
                }
            }

            let progress = (processed_days as f64 / total_days as f64) * 100.0;
            info!("Date: {} - Union tables processed: {}/{}, Total progress: {:.2}%", date_str, processed_tables, table_names.len(), progress);

            start_date += chrono::Duration::days(1);
        }

        info!("Union table optimization is complete for {} days.", total_days);
        Ok(())
    }

    pub async fn insert_into_unioned_table(&self, database: &str, target_table_name: &str, additional_table_names: &[String], report_progress: bool) -> Result<(), Error>
    {
        // 构建UNION ALL查询
        let queries = Arc::new(Mutex::new(Vec::new()));
        let total_tables = additional_table_names.len();

        additional_table_names.par_iter().enumerate().for_each(|(i, table_name)| {
                                                         let select_query = ClickHouseQueryBuilder::new().select("symbol, side, price, timestamp, amount") // Select required fields
                                                                                                         .from(database, table_name) // Format the table name with database
                                                                                                         .build(); // Build the individual query

                                                         let mut queries_lock = queries.lock().unwrap();
                                                         queries_lock.push(select_query);

                                                         // 如果启用进度汇报，每处理完一个表就汇报一次进度
                                                         if report_progress {
                                                             let progress = ((i + 1) as f64 / total_tables as f64) * 100.0;
                                                             info!("Progress: Processed {} / {} tables ({:.2}%)", i + 1, total_tables, progress);
                                                         }
                                                     });

        let queries = Arc::try_unwrap(queries).expect("Failed to unwrap Arc").into_inner().unwrap();
        let union_all_query = queries.join(" UNION DISTINCT ");

        let final_query = format!("INSERT INTO {}.{} SELECT DISTINCT symbol, side, price, timestamp, amount FROM ({})",
                                  database, target_table_name, union_all_query);

        info!("The Final Query is : {}", final_query);
        if report_progress {
            info!("Successfully constructed the final insert query.");
        }

        // 执行插入数据的查询
        self.client.read().await.query(&final_query).execute().await?;

        if report_progress {
            info!("Data inserted into {}.{} successfully.", database, target_table_name);
        }

        Ok(())
    }

    pub async fn create_database_if_not_exists(&self, database: &str) -> Result<(), Error>
    {
        // 创建数据库的SQL查询
        let create_db_query = format!("CREATE DATABASE IF NOT EXISTS {}", database);

        // 执行创建数据库的SQL查询
        self.client.read().await.query(&create_db_query).execute().await?;

        info!("Database {} created successfully or already exists", database);
        Ok(())
    }

    pub async fn create_users_table(&self, database: &str) -> Result<(), Error>
    {
        // 首先创建数据库（如果不存在）
        self.create_database_if_not_exists(database).await?;

        // 创建用户表的SQL查询
        let create_table_query = format!(
                                         "CREATE TABLE IF NOT EXISTS {}.user_info ( \
        id UUID, \
        username String, \
        email String, \
        password_hash String, \
        created_at DateTime64(3), \
        last_login DateTime64(3) DEFAULT now() \
    )   ENGINE = ReplacingMergeTree() \
        PARTITION BY toYYYYMMDD(created_at) \
        ORDER BY (created_at, id)",
                                         database
        );

        // 执行创建表的SQL查询
        self.client.read().await.query(&create_table_query).execute().await?;

        info!("Table {}.user_info created successfully", database);
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    // use chrono::NaiveDate;

    async fn setup_clickhouse_client() -> ClickHouseClient
    {
        // 假设 ClickHouse 在本地运行，且使用默认设置
        ClickHouseClient::new()
    }

    #[tokio::test]
    async fn test_construct_table_name()
    {
        let client = setup_clickhouse_client().await;
        let table_name = client.construct_table_name("binance", "futures", "trades", "2024_08_24", "BTC", "USDT");
        assert_eq!(table_name, "binance_futures_trades_2024_08_24_BTCUSDT");
    }
}
