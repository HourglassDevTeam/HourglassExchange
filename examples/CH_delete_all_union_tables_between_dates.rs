use chrono::{Duration, NaiveDate};
use rayon::prelude::*;
use std::collections::HashMap;
use unilink_execution::sandbox::{clickhouse_api::queries_operations::ClickHouseClient, utils::chrono_operations::extract_date};

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();

    // 设置参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2024, 3, 4).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 9).expect("Invalid end date"); // 设置结束日期
    let mut current_date = start_date;

    // 将上述变量拼接成数据库名
    let database = client.construct_database_name(exchange, instrument, channel);

    // 获取所有表名
    let all_tables = client.get_table_names(&database).await;

    // 创建一个表名与日期的字典
    let table_date_map: HashMap<String, String> = all_tables.par_iter()
                                                            .filter_map(|table_name| {
                                                                if let Some(table_date) = extract_date(table_name) {
                                                                    Some((table_name.clone(), table_date))
                                                                }
                                                                else {
                                                                    None
                                                                }
                                                            })
                                                            .collect();

    // 计算总天数，用于进度汇报
    let total_days = (end_date - start_date).num_days() + 1;
    let mut processed_days = 0;

    // 遍历日期范围并删除对应日期的 union 表
    while current_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = current_date.format("%Y_%m_%d").to_string();

        // 更新进度
        processed_days += 1;
        let progress = (processed_days as f64 / total_days as f64) * 100.0;
        println!("Processing tables on date: {} ({:.2}%)", date_str, progress); // 打印当前处理的日期和进度

        // 筛选出与当前日期匹配的 union 表名
        let tables_to_delete: Vec<String> = table_date_map.par_iter()
                                                          .filter(|&(table_name, table_date)| table_name.contains("union") && table_date == &date_str)
                                                          .map(|(table_name, _)| table_name.clone())
                                                          .collect();

        // 如果找到对应的表，则删除表
        for table in tables_to_delete {
            let drop_query = format!("DROP TABLE {}.{}", database, table);
            println!("[UniLinkExecution] : Executing query: {}", drop_query);

            match client.client.read().await.query(&drop_query).execute().await {
                | Ok(_) => println!("Successfully dropped table: {}.{}", database, table),
                | Err(e) => eprintln!("Error dropping table: {}", e),
            }
        }

        // 迭代到下一天
        current_date += Duration::days(1);
    }

    println!("Union tables deletion is done for: {} days", total_days); // 最终进度汇报
}
