use chrono::{Duration, NaiveDate};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::utils::chrono_operations::extract_date;

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let start_date = NaiveDate::from_ymd_opt(2024, 3, 3).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 7, 3).expect("Invalid end date"); // 设置结束日期
    let mut current_date = start_date;

    // 在循环外部获取所有的表名
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let table_names = client.get_table_names(&database).await;

    // 遍历日期范围
    while current_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = current_date.format("%Y_%m_%d").to_string();
        println!("数据库: {}", database); // 打印数据库名称以供调试

        // 筛选出与当前日期匹配的表名
        let tables: Vec<String> = table_names
            .iter()
            .filter(|table_name| {
                if let Some(table_date) = extract_date(table_name) {
                    table_date == date_str // 仅保留匹配当前日期的表
                } else {
                    false
                }
            })
            .cloned() // 克隆以避免引用问题
            .collect();

        println!("处理日期 {} 的表: {:?}", date_str, tables); // 打印当前处理的日期和对应的表名

        // 如果找到对应的表，则创建联合表
        if !tables.is_empty() {
            let new_table_name = format!("{}_{}_{}_union_{}", exchange, instrument, channel, date_str);
            match client.create_unioned_tables_for_date(&database, &new_table_name, &tables).await {
                Ok(_) => println!("成功创建联合表: {}.{}", database, new_table_name),
                Err(e) => eprintln!("创建联合表时出错: {}", e),
            }
        } else {
            println!("日期 {} 没有数据", date_str); // 如果没有找到表，则输出提示信息
        }

        // 迭代到下一天
        current_date += Duration::days(1);
    }
}
