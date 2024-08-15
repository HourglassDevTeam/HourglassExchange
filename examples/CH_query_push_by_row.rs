use std::time::Instant;
use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;
use unilink_execution::sandbox::clickhouse_api::query_builder::ClickHouseQueryBuilder;

#[tokio::main]
async fn main() {
    // 创建 ClickHouseClient 实例，用于与 ClickHouse 数据库进行交互
    let client = ClickHouseClient::new();

    // 定义查询参数
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_05_05";
    let base = "1000RATS";
    let quote = "USDT";

    // 构造数据库名称和表名称
    let database_name = client.construct_database_name(exchange, instrument, "trades");
    let table_name = client.construct_table_name(exchange, instrument, "trades", date, base, quote);

    // 使用 ClickHouseQueryBuilder 构造查询语句
    let query = ClickHouseQueryBuilder::new()
        .select("symbol, side, price, timestamp, amount")  // 选择需要的字段
        .from(&database_name, &table_name)  // 指定数据表，包含数据库名称
        .order("timestamp", Some("DESC"))  // 按 timestamp 字段降序排序
        .build();  // 构建最终的查询语句

    // 打印构造的查询语句
    println!("[UniLinkExecution] : Constructed query {}", query);

    // 获取 ClickHouse 客户端的只读引用
    let client_ref = client.client.read().await;

    // 执行查询并获取游标（cursor）用于迭代结果
    // `fetch::<ClickhousePublicTrade>()` 返回一个异步迭代器，`cursor` 是这个迭代器的实例
    let mut cursor = client_ref.query(&query).fetch::<ClickhousePublicTrade>().unwrap();

    // 记录开始时间以计算查询和数据获取的耗时
    let start_time = Instant::now();

    // 异步迭代游标中的数据行
    // `cursor.next().await` 返回一个 `Result<Option<T>, E>`，`T` 是数据行类型，`E` 是错误类型
    // `Ok(Some(row))` 表示成功获取到一行数据
    while let Ok(Some(row)) = cursor.next().await {
        // 打印获取到的每一行数据
        println!("{:?}", row);
    }

    // 计算并打印数据获取的总耗时
    let duration = start_time.elapsed();
    println!("ClickhousePublicTrade data fetched in: {:?}", duration);
}
