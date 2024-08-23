use chrono::{Duration, NaiveDate};
use dotenvy::dotenv;
use open_lark::{custom_bot::CustomBot, service::im::v1::message::MessageText};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::{collections::HashSet, env};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main()
{
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let mut start_date = NaiveDate::from_ymd_opt(2021, 04, 10).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 3, 3).expect("Invalid end date"); // 设置结束日期
    let database = format!("{}_{}_{}", exchange, instrument, channel);

    // 获取所有包含 "union" 的表名
    let all_table_names: HashSet<_> = client.get_union_table_names(&database).await.into_par_iter().collect();
    let total_days = (end_date - start_date).num_days() + 1;

    // 筛选符合日期范围的表名
    let mut valid_table_names = HashSet::new();
    let mut current_date = start_date;

    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();
        let matching_tables: Vec<_> = all_table_names.par_iter().filter(|table_name| table_name.contains(&date_str)).cloned().collect(); // 将结果收集到 Vec 中
        valid_table_names.extend(matching_tables); // 扩展到 HashSet 中
        current_date += Duration::days(1);
    }

    // 计算要优化的表格数量
    let total_tables = valid_table_names.len();

    // 加载 .env 文件
    dotenv().expect(".env file not found");
    let hook_url = &(env::var("HOOK_URL").unwrap());
    let secret = env::var("HOOK_SECRET").ok();
    // 创建 CustomBot 实例
    let bot = CustomBot::new(hook_url, secret.as_deref());

    // 如果有表需要优化，汇报开始优化
    if total_tables > 0 {
        let message = MessageText::new(format!("[UniLinkExecution] : Starting optimization for {} tables.", total_tables,).as_str());
        println!("[UniLinkExecution] : Starting optimization for {} tables.", total_tables);
        bot.send_message(message).await.unwrap();
    }

    // 初始化已处理的表数
    let mut processed_tables = 0;

    // 开始优化表格
    while start_date <= end_date {
        let date_str = start_date.format("%Y_%m_%d").to_string();
        let tables_to_remove: Vec<_> = valid_table_names.par_iter().filter(|table_name| table_name.contains(&date_str)).cloned().collect();

        for table_name in &tables_to_remove {
            let table_path = format!("{}.{}", database, table_name);

            // 执行优化操作
            if let Err(e) = client.optimize_table(&table_path).await {
                eprintln!("[UniLinkExecution] : Error optimizing table {}: {}", table_path, e);
            }

            // 更新已处理的表数
            processed_tables += 1;

            // 从 HashSet 中移除已处理的表
            valid_table_names.remove(table_name);

            // 打印当前总进度
            let progress = (processed_tables as f64 / total_tables as f64) * 100.0;
            println!("[UniLinkExecution] : Date: {} - Total tables processed: {}/{} (Total progress: {:.2}%)",
                     date_str, processed_tables, total_tables, progress);
        }

        // 迭代到下一天
        start_date += Duration::days(1);
    }

    // 发送文本消息，汇报最终结果
    let message = MessageText::new(format!("[UniLinkExecution] : Clickhouse Database Optimization is complete for {} tables across {} days.",
                                           total_tables, total_days).as_str());
    bot.send_message(message).await.unwrap();
    println!("[UniLinkExecution] : Clickhouse Database Optimization is complete for {} tables across {} days.",
             total_tables, total_days);
}
