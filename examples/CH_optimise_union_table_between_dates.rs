use chrono::{Duration, NaiveDate};
use std::collections::HashSet;
use std::env;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use rayon::iter::ParallelIterator;
use dotenvy::dotenv;

use open_lark::{
    custom_bot::CustomBot,
    service::im::v1::message::{
        MessageText,
    },
};

#[tokio::main]
async fn main() {
    // 创建 ClickHouse 客户端实例
    let client = ClickHouseClient::new();

    // 定义参数
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let mut start_date = NaiveDate::from_ymd_opt(2019, 11, 20).expect("Invalid start date"); // 设置开始日期
    let end_date = NaiveDate::from_ymd_opt(2024, 3, 3).expect("Invalid end date"); // 设置结束日期
    let database = format!("{}_{}_{}", exchange, instrument, channel);
    let mut table_names: HashSet<_> = client.get_union_table_names(&database).await.into_par_iter().collect(); // 将表名存入 HashSet 以便快速查找和移除
    // 加载 .env 文件
    dotenv().expect(".env file not found");
    let hook_url = &(env::var("HOOK_URL").unwrap());
    let secret = env::var("HOOK_SECRET").ok();
    // 创建 CustomBot 实例
    let bot = CustomBot::new(hook_url, secret.as_deref());

    // 计算总天数
    let total_days = (end_date - start_date).num_days() + 1;
    // 如果有表需要优化，汇报开始优化
    if !table_names.is_empty() {
        // 发送文本消息，汇报优化开始
        let message = MessageText::new(
            format!(
                "[UniLinkExecution] : Starting optimization for {} tables.",
                table_names.len(),
            )
                .as_str(),
        );
        // bot.send_message(message).await.unwrap();
    }

    // 获取所有表名总数，用于进度汇报
    let mut total_tables = 0;
    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = current_date.format("%Y_%m_%d").to_string();
        total_tables += table_names
            .par_iter()
            .filter(|table_name| table_name.contains(&date_str))
            .count();
        current_date += Duration::days(1);
    }

    // 初始化已处理的表数
    let mut processed_tables = 0;


    // 遍历日期范围
    while start_date <= end_date {
        // 将当前日期格式化为字符串
        let date_str = start_date.format("%Y_%m_%d").to_string();

        // 筛选出包含 "union" 和当前日期字样的表名
        let tables_to_remove: Vec<_> = table_names
            .par_iter()
            .filter(|table_name| table_name.contains(&date_str))
            .cloned()
            .collect();

        for table_name in &tables_to_remove {
            let table_path = format!("{}.{}", database, table_name);

            // 执行优化操作
            if let Err(e) = client.optimize_table(&table_path).await {
                eprintln!("[UniLinkExecution] : Error optimizing table {}: {}", table_path, e);
            }

            // 更新已处理的表数
            processed_tables += 1;

            // 从 HashSet 中移除已处理的表
            table_names.remove(table_name);

            // 打印当前总进度
            let progress = (processed_tables as f64 / total_tables as f64) * 100.0;
            println!(
                "[UniLinkExecution] : Date: {} - Total tables processed: {}/{} (Total progress: {:.2}%)",
                date_str, processed_tables, total_tables, progress
            );
        }

        // 迭代到下一天
        start_date += Duration::days(1);
    }

    // 发送文本消息，汇报最终结果
    let message = MessageText::new( format!(
        "[UniLinkExecution] : Clickhouse Database Optimization is complete for {} tables across {} days.",
        total_tables, total_days
    ).as_str());
    // bot.send_message(message).await.unwrap();
    println!(
        "[UniLinkExecution] : Clickhouse Database Optimization is complete for {} tables across {} days.",
        total_tables, total_days
    );
}
