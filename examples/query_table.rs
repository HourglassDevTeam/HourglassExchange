use std::time::Instant;
use unilink_execution::sandbox::clickhouse_api::queries_operations::ClickHouseClient;

#[tokio::main]
async fn main() {
    let client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_05_05";
    let base = "1000RATS";
    let quote = "USDT";

    // Example 1: Fetch all trades
    println!("Fetching all trades...");
    let start_time = Instant::now();
    match client.retrieve_all_trades(exchange, instrument,date, base, quote).await {
        | Ok(trades) => {
            let duration = start_time.elapsed();
            println!("OrderBook25 data fetched in: {:?}", duration);
            println!("{:?}", trades);
        }
        | Err(err) => {
            let duration = start_time.elapsed();
            eprintln!("Error fetching trades in {:?}: {:?}", duration, err);
        }
    }

    // Example 2: Fetch latest trade
    // println!("Fetching latest trade...");
    // let start_time = Instant::now();
    // let result = client.retrieve_latest_trade(exchange, instrument, date, base, quote).await;
    // match result {
    //     | Ok(trade) => {
    //         let duration = start_time.elapsed();
    //         println!("Latest trade fetched in: {:?}", duration);
    //         println!("{:?}", trade);
    //     }
    //     | Err(e) => {
    //         let duration = start_time.elapsed();
    //         eprintln!("Error fetching latest trade in {:?}: {:?}", duration, e);
    //     }
    // }
}
