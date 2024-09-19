use chrono::{Duration, NaiveDate};
#[cfg(feature = "lark")]
use dotenvy::dotenv;
#[cfg(feature = "lark")]
use open_lark::{custom_bot::CustomBot, service::im::v1::message::MessageText};
use rayon::prelude::*;
#[cfg(feature = "lark")]
use std::env;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use unilink_execution::hourglass::{clickhouse_api::queries_operations::ClickHouseClient, utils::chrono_operations::extract_date};

#[tokio::main]
async fn main()
{
    // 检测是否为 release 模式
    #[cfg(debug_assertions)]
    println!("Running in debug mode");

    #[cfg(not(debug_assertions))]
    println!("Running in release mode");

    #[cfg(feature = "lark")]
    {
        // 加载 .env 文件
        dotenv().expect(".env file not found");
    }

    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2019, 11, 1).expect("Invalid start date");
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 30).expect("Invalid end date");

    // 在循环外部获取所有表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    // 创建一个表名与日期的字典，并将其转换为 Arc<Mutex<_>> 以供并行使用
    let table_date_map: Arc<Mutex<HashMap<String, NaiveDate>>> = Arc::new(Mutex::new(table_names.par_iter()
                                                                                                .filter_map(|table_name| {
                                                                                                    if !table_name.contains("union") {
                                                                                                        if let Some(table_date_str) = extract_date(table_name) {
                                                                                                            if let Ok(table_date) = NaiveDate::parse_from_str(&table_date_str, "%Y_%m_%d") {
                                                                                                                return Some((table_name.clone(), table_date));
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                    None
                                                                                                })
                                                                                                .collect()));

    // 计算总的表数量，用于进度汇报
    let total_tables = table_date_map.lock().unwrap().values().filter(|&&table_date| table_date >= start_date && table_date <= end_date).count();
    let mut processed_tables = 0;

    #[cfg(feature = "lark")]
    let bot = {
        let hook_url = env::var("HOOK_URL").unwrap();
        let secret = env::var("HOOK_SECRET").ok();
        CustomBot::new(&hook_url, secret.as_deref())
    };

    // 遍历日期范围
    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();

        // 筛选出与当前日期匹配且不含 "union" 字样的表名
        let tables: Vec<String> = {
            let table_date_map = table_date_map.clone();
            let map = table_date_map.lock().unwrap();
            map.par_iter().filter(|&(_, &table_date)| table_date == current_date).map(|(table_name, _)| table_name.clone()).collect()
        };

        // 并行创建联合表
        if !tables.is_empty() {
            let new_table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date_str);
            let client = Arc::clone(&client);
            let database = database.clone();
            let tables_for_task = tables.clone();
            let table_date_map = Arc::clone(&table_date_map);

            tokio::spawn(async move {
                match client.create_unioned_table(&database, &new_table_name, &tables_for_task, true).await {
                    | Ok(_) => {
                        // 删除处理掉的表名
                        let mut map = table_date_map.lock().unwrap();
                        for table in &tables_for_task {
                            map.remove(table);
                        }
                        println!("Successfully created table: {}.{}", database, new_table_name);
                    }
                    | Err(e) => eprintln!("Error creating table: {}", e),
                }
            }).await
              .unwrap();

            // 更新进度
            processed_tables += tables.len();
            let progress = (processed_tables as f64 / total_tables as f64) * 100.0;
            println!("Overall progress: Processed {} / {} tables ({:.2}%)", processed_tables, total_tables, progress);
        }
        else {
            println!("No data for date: {}", date_str);
        }

        // 迭代到下一天
        current_date += Duration::days(1);
    }

    #[cfg(feature = "lark")]
    {
        let message = MessageText::new(&format!("Union tables creation is done for {} tables", total_tables));
        bot.send_message(message).await.unwrap();
    }

    println!("Union tables creation is done for {} tables", total_tables);
}
