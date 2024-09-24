use hourglass::hourglass::clickhouse_api::queries_operations::ClickHouseClient;
use std::sync::Arc;

#[tokio::main]
async fn main()
{
    // 创建 ClickHouse 客户端实例
    let client = Arc::new(ClickHouseClient::new());

    // 调用创建用户表的函数
    let result = client.create_users_table("accounts").await;
    println!("{:?}", result);
}
