use std::fs;
use unilink_execution::sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade;

// Define the path to the JSON file
const DATA_HISTORIC_TRADES: &str = "tests/util/sample_trades.json";

// Define a function to load the JSON and convert it to Vec<MarketTrade>
fn load_json_market_trade() -> Vec<MarketTrade> {
    let trades_data = fs::read_to_string(DATA_HISTORIC_TRADES).expect("failed to read file");

    // Deserialize directly into Vec<MarketTrade>
    let trades: Vec<MarketTrade> = serde_json::from_str(&trades_data).expect("failed to parse trades data");

    trades
}


// Define the test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_json_market_trade() {
        // Set up the expected data
        let expected_trades = vec![
            MarketTrade {
                exchange: "binance-futures".to_string(),
                symbol: "1000PEPEUSDT".to_string(),
                side: "buy".to_string(),
                price: 1000.0,
                timestamp: 1649188800000000,
                amount: 1000000000.0,
            },
            MarketTrade {
                exchange: "binance-futures".to_string(),
                symbol: "1000PEPEUSDT".to_string(),
                side: "buy".to_string(),
                price: 1050.0,
                timestamp: 1649192400000000,
                amount: 1000000000.0,
            },
            MarketTrade {
                exchange: "binance-futures".to_string(),
                symbol: "1000PEPEUSDT".to_string(),
                side: "buy".to_string(),
                price: 1060.0,
                timestamp: 1649196000000000,
                amount: 1000000000.0,
            },
            MarketTrade {
                exchange: "binance-futures".to_string(),
                symbol: "1000PEPEUSDT".to_string(),
                side: "buy".to_string(),
                price: 1200.0,
                timestamp: 1649199600000000,
                amount: 1000000000.0,
            },
        ];

        // Load the data using the function
        let actual_trades = load_json_market_trade();

        // Print the results with line spacing
        for trade in &expected_trades {
            println!("actual_trades : {:?}", trade);
            println!(); // Add a blank line between trades
        }
        // Print the results with line spacing
        for expected_trade in &expected_trades {
            println!("expected_trades : {:?}", expected_trade);
            println!(); // Add a blank line between trades
        }

        assert_eq!(actual_trades, expected_trades);

    }
}