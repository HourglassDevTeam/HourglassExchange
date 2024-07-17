use pin_utils::pin_mut;
use tokio_stream::StreamExt;
use unilink_execution::simulated_exchange::load_from_clickhouse::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main()
{
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let channel = "trades";
    let date = "2024_03_03";

    let stream = client.query_union_table_batched(exchange, instrument, channel, date);
    pin_mut!(stream); // Pin the stream to make it Unpin

    while let Some(result) = stream.next().await {
        match result {
            | Ok(trades) => {
                for trade in trades {
                    println!("{:?}", trade);
                }
            }
            | Err(e) => eprintln!("Error: {}", e),
        }
    }
}
