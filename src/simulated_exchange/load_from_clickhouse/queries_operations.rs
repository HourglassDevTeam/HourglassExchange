// NOTE this module is previously built and imported into the main project as a dependency.
//      upon completion the following code should be deleted and external identical code should be used instead.

pub use clickhouse::{
    Client,
    error::{Error, Result}, Row,
};
use serde::{Deserialize, Serialize};

use crate::simulated_exchange::utils::chrono_operations::extract_date;
use crate::simulated_exchange::ws_trade::WsTrade;

pub struct ClickHouseClient {
    pub client: Client,
}

impl ClickHouseClient {
    pub fn new() -> Self {
        let client = Client::default().with_url("http://localhost:8123").with_user("default").with_password("");

        println!("[AlgoBacktest] : 连接到 ClickHouse 服务器成功。");

        Self { client }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct OrderBook25 {
    pub exchange: String,
    pub symbol: String,
    pub timestamp: i64,
    pub local_timestamp: i64,
    pub asks_0_price: f64,
    pub asks_0_amount: f64,
    pub bids_0_price: f64,
    pub bids_0_amount: f64,
    pub asks_1_price: f64,
    pub asks_1_amount: f64,
    pub bids_1_price: f64,
    pub bids_1_amount: f64,
    pub asks_2_price: f64,
    pub asks_2_amount: f64,
    pub bids_2_price: f64,
    pub bids_2_amount: f64,
    pub asks_3_price: f64,
    pub asks_3_amount: f64,
    pub bids_3_price: f64,
    pub bids_3_amount: f64,
    pub asks_4_price: f64,
    pub asks_4_amount: f64,
    pub bids_4_price: f64,
    pub bids_4_amount: f64,
    pub asks_5_price: f64,
    pub asks_5_amount: f64,
    pub bids_5_price: f64,
    pub bids_5_amount: f64,
    pub asks_6_price: f64,
    pub asks_6_amount: f64,
    pub bids_6_price: f64,
    pub bids_6_amount: f64,
    pub asks_7_price: f64,
    pub asks_7_amount: f64,
    pub bids_7_price: f64,
    pub bids_7_amount: f64,
    pub asks_8_price: f64,
    pub asks_8_amount: f64,
    pub bids_8_price: f64,
    pub bids_8_amount: f64,
    pub asks_9_price: f64,
    pub asks_9_amount: f64,
    pub bids_9_price: f64,
    pub bids_9_amount: f64,
    pub asks_10_price: f64,
    pub asks_10_amount: f64,
    pub bids_10_price: f64,
    pub bids_10_amount: f64,
    pub asks_11_price: f64,
    pub asks_11_amount: f64,
    pub bids_11_price: f64,
    pub bids_11_amount: f64,
    pub asks_12_price: f64,
    pub asks_12_amount: f64,
    pub bids_12_price: f64,
    pub bids_12_amount: f64,
    pub asks_13_price: f64,
    pub asks_13_amount: f64,
    pub bids_13_price: f64,
    pub bids_13_amount: f64,
    pub asks_14_price: f64,
    pub asks_14_amount: f64,
    pub bids_14_price: f64,
    pub bids_14_amount: f64,
    pub asks_15_price: f64,
    pub asks_15_amount: f64,
    pub bids_15_price: f64,
    pub bids_15_amount: f64,
    pub asks_16_price: f64,
    pub asks_16_amount: f64,
    pub bids_16_price: f64,
    pub bids_16_amount: f64,
    pub asks_17_price: f64,
    pub asks_17_amount: f64,
    pub bids_17_price: f64,
    pub bids_17_amount: f64,
    pub asks_18_price: f64,
    pub asks_18_amount: f64,
    pub bids_18_price: f64,
    pub bids_18_amount: f64,
    pub asks_19_price: f64,
    pub asks_19_amount: f64,
    pub bids_19_price: f64,
    pub bids_19_amount: f64,
    pub asks_20_price: f64,
    pub asks_20_amount: f64,
    pub bids_20_price: f64,
    pub bids_20_amount: f64,
    pub asks_21_price: f64,
    pub asks_21_amount: f64,
    pub bids_21_price: f64,
    pub bids_21_amount: f64,
    pub asks_22_price: f64,
    pub asks_22_amount: f64,
    pub bids_22_price: f64,
    pub bids_22_amount: f64,
    pub asks_23_price: f64,
    pub asks_23_amount: f64,
    pub bids_23_price: f64,
    pub bids_23_amount: f64,
    pub asks_24_price: f64,
    pub asks_24_amount: f64,
    pub bids_24_price: f64,
    pub bids_24_amount: f64,
}
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct DataAvailabilities {
    pub ts: i64,
    pub token_pair: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct TradeDataFromClickhouse {
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
}


impl ClickHouseClient {
    // NOTE 这些函数分为不同的交易所分为两个分支 因为不同交易所的明明逻辑是不一样的：
    // NOTE binance csv nomenclature: {exchange}_{instrument}_{channel}_{date}_{base+quote}
    // e.g.: binance_futures_trades_2024_07_03_ZRXUSDT
    // e.g.: binance_futures_book_snapshot_25_2024_07_03_ZECUSDT
    // NOTE okex csv nomenclature: {exchange}_{instrument}_{channel}_{date}_{base}_{quote}_{INSTRUMENT}
    // e.g.: okex_swap_trades_2024_03_03_BTC_USDT_SWAP
    // e.g.: okex_swap_book_snapshot_25_2024_03_03_BTC_USDT_SWAP
    fn construct_table_name(&self, exchange: &str, instrument: &str, channel: &str, date: &str, base: &str, quote: &str) -> String {
        match exchange {
            | "binance" => format!(
                "{}_{}_{}_{}_{}",
                exchange,
                instrument,
                channel,
                date.replace("-", "_"),
                base.to_uppercase() + &*quote.to_uppercase()
            ),
            | "okex" => format!(
                "{}_{}_{}_{}_{}_{}_{}",
                exchange,
                instrument,
                channel,
                date.replace("-", "_"),
                base.to_uppercase(),
                quote.to_uppercase(),
                instrument.to_uppercase()
            ),
            | _ => panic!("Unsupported exchange"),
        }
    }

    fn construct_database_name(&self, exchange: &str, instrument: &str, channel: &str) -> String {
        format!("{}_{}_{}", exchange, instrument, channel)
    }

    pub async fn get_table_names(&self, database: &str) -> Vec<String> {
        let table_names_query = format!("SHOW TABLES FROM {database}",);
        println!("{:?}", table_names_query);
        let result = self.client.query(&table_names_query).fetch_all::<String>().await.unwrap_or_else(|e| {
            eprintln!("[AlgoBacktest] : Error loading table names: {:?}", e);
            vec![]
        });

        result
    }

    pub async fn get_tables_for_date(&self, database: &str, date: &str) -> Vec<String> {
        // 获取所有表名
        let table_names = self.get_table_names(database).await;

        // 筛选出指定日期的表名
        let tables_for_date: Vec<String> = table_names
            .into_iter()
            .filter(|table_name| {
                if let Some(table_date) = extract_date(table_name) {
                    table_date == date
                } else {
                    false
                }
            })
            .collect();

        tables_for_date
    }

    pub async fn retrieve_all_trades(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<Vec<WsTrade>, Error> {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp", full_table_path);
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let trade_datas = self.client.query(&query).fetch_all::<TradeDataFromClickhouse>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub async fn retrieve_latest_trade(&self, exchange: &str, instrument: &str, date: &str, base: &str, quote: &str) -> Result<WsTrade, Error> {
        let database_name = self.construct_database_name(exchange, instrument, "trades");
        let table_name = self.construct_table_name(exchange, instrument, "trades", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!(
            "SELECT symbol, side, price, timestamp FROM {} ORDER BY timestamp DESC LIMIT 1",
            full_table_path
        );
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let trade_data = self.client.query(&query).fetch_one::<TradeDataFromClickhouse>().await?;
        Ok(WsTrade::from(trade_data))
    }

    pub async fn retrieve_all_order_books_25(
        &self,
        exchange: &str,
        instrument: &str,
        date: &str,
        base: &str,
        quote: &str,
    ) -> Result<Vec<OrderBook25>, Error> {
        let database_name = self.construct_database_name(exchange, instrument, "book_snapshot_25");
        let table_name = self.construct_table_name(exchange, instrument, "book_snapshot_25", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT * FROM {}", full_table_path);
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let result = self.client.query(&query).fetch_all::<OrderBook25>().await?;
        Ok(result)
    }

    pub async fn retrieve_latest_order_book_25(
        &self,
        exchange: &str,
        instrument: &str,
        date: &str,
        base: &str,
        quote: &str,
    ) -> Result<OrderBook25, Error> {
        let database_name = self.construct_database_name(exchange, instrument, "book_snapshot_25");
        let table_name = self.construct_table_name(exchange, instrument, "book_snapshot_25", date, base, quote);
        let full_table_path = format!("{}.{}", database_name, table_name);
        let query = format!("SELECT * FROM {} ORDER BY timestamp DESC LIMIT 1", full_table_path);
        println!("[AlgoBacktest] : 查询SQL语句 {}", query);
        let result = self.client.query(&query).fetch_one::<OrderBook25>().await?;
        Ok(result)
    }

    pub async fn availabilities_union_all(&self, database_name: &str, table_names: &Vec<String>) -> Result<Vec<DataAvailabilities>, Error> {
        let mut queries = Vec::new();

        // 遍历所有表名
        for table_name in table_names {
            // 使用下划线分割表名，获取各部分信息
            let parts: Vec<&str> = table_name.split('_').collect();
            let exchange = parts[0]; // 获取交易所名称

            // 根据交易所名称处理不同的命名逻辑
            let currency_pair = match exchange.to_lowercase().as_str() {
                // 处理 Binance 的表名
                | "binance" => {
                    let base_quote = parts[parts.len() - 1]; // 获取 base+quote 部分
                    base_quote.to_string()
                }
                // 处理 OKEx 的表名
                | "okex" => {
                    let base = parts[parts.len() - 3]; // 获取倒数第三部分作为 base
                    let quote = parts[parts.len() - 2]; // 获取倒数第二部分作为 quote
                    format!("{}_{}", base, quote)
                }
                // 如果交易所不在已知列表中，打印错误信息并跳过
                | _ => {
                    eprintln!("Unrecognized exchange in table name: {}", table_name);
                    continue;
                }
            };

            // 构建查询语句
            let query = format!("SELECT timestamp, '{}' AS tokenname FROM {}.{}", currency_pair, database_name, table_name);
            queries.push(query); // 将查询语句加入查询向量
        }

        // 使用 UNION ALL 将所有查询语句连接起来
        let final_query = queries.join(" UNION ALL ");
        println!("Constructed query: {}", final_query);

        // 执行最终的查询，并获取结果
        let result = self.client.query(&final_query).fetch_all::<DataAvailabilities>().await?;
        Ok(result)
    }


    pub async fn wstrade_union_all(&self, database_name: &str, table_names: &Vec<String>) -> Result<Vec<WsTrade>, Error> {
        let mut queries = Vec::new();

        for table_name in table_names {
            let query = format!("SELECT symbol, side, price, timestamp FROM {}.{}", database_name, table_name);
            queries.push(query);
        }

        // 将所有子查询包装在一个子查询中，然后在最外层添加排序
        let final_query = format!("SELECT * FROM ({}) ORDER BY timestamp", queries.join(" UNION ALL "));
        println!("[AlgoBacktest] : Constructed query: {}", final_query);

        let trade_datas = self.client.query(&final_query).fetch_all::<TradeDataFromClickhouse>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

    pub async fn create_unioned_tables_for_date(&self, database: &str, date: &str, new_table_name: &str) -> Result<(), Error> {
        // 获取指定日期的所有表名
        let table_names = self.get_tables_for_date(database, date).await;

        // 构建UNION ALL查询
        let mut queries = Vec::new();
        for table_name in table_names {
            let query = format!("SELECT symbol, side, price, timestamp FROM {}.{}", database, table_name);
            queries.push(query);
        }
        let union_all_query = queries.join(" UNION ALL ");

        // 假设你要创建的表使用MergeTree引擎并按timestamp排序
        let final_query = format!(
            "CREATE TABLE {}.{} ENGINE = MergeTree() ORDER BY timestamp AS {}",
            database, new_table_name, union_all_query
        );
        println!("[AlgoBacktest] : Constructed query: {}", final_query);

        // 执行创建新表的查询
        self.client.query(&final_query).execute().await?;
        Ok(())
    }

    pub async fn order_book_25_union_all(&self, database_name: &str, table_names: &Vec<String>) -> Result<Vec<OrderBook25>, Error> {
        let mut queries = Vec::new();
        for table_name in table_names {
            let query = format!("SELECT * FROM {}.{}", database_name, table_name);
            queries.push(query);
        }
        let final_query = queries.join(" UNION ALL ");
        println!("[AlgoBacktest] : Constructed query: {}", final_query);
        let result = self.client.query(&final_query).fetch_all::<OrderBook25>().await?;
        Ok(result)
    }

    pub async fn query_union_table(client: &ClickHouseClient, exchange: &str, instrument: &str, channel: &str, date: &str) -> Result<Vec<WsTrade>, clickhouse::error::Error> {
        let table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date);
        let database = format!("{}_{}_{}", exchange, instrument, channel);
        let query = format!("SELECT * FROM {}.{}", database, table_name);
        println!("[AlgoBacktest] : Executing query: {}", query);
        let trade_datas = client.client.query(&query).fetch_all::<TradeDataFromClickhouse>().await?;
        let ws_trades: Vec<WsTrade> = trade_datas.into_iter().map(WsTrade::from).collect();
        Ok(ws_trades)
    }

}
