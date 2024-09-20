// use hourglass::hourglass::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade;
// use std::fs;

// 定义JSON文件的路径
// const DATA_HISTORIC_TRADES: &str = "tests/util/sample_trades.json";
//
// // 定义一个函数来加载JSON并将其转换为Vec<MarketTrade>
// fn load_json_market_trade() -> Vec<MarketTrade>
// {
//     // 读取文件内容
//     let trades_data = fs::read_to_string(DATA_HISTORIC_TRADES).expect("读取文件失败");
//
//     // 直接将字符串反序列化为Vec<MarketTrade>
//     let trades: Vec<MarketTrade> = serde_json::from_str(&trades_data).expect("解析交易数据失败");
//
//     trades
// }
//
// // 定义测试
// #[cfg(test)]
// mod tests
// {
//     use super::*;
//
//     #[test]
//     fn test_load_json_market_trade()
//     {
//         // 设置预期数据
//         let expected_trades = vec![MarketTrade { exchange: "binance-futures".to_string(),
//                                                  symbol: "1000PEPEUSDT".to_string(),
//                                                  side: "buy".to_string(),
//                                                  price: 1000.0,
//                                                  timestamp: 1649188800000000,
//                                                  amount: 1000000000.0 },
//                                    MarketTrade { exchange: "binance-futures".to_string(),
//                                                  symbol: "1000PEPEUSDT".to_string(),
//                                                  side: "buy".to_string(),
//                                                  price: 1050.0,
//                                                  timestamp: 1649192400000000,
//                                                  amount: 1000000000.0 },
//                                    MarketTrade { exchange: "binance-futures".to_string(),
//                                                  symbol: "1000PEPEUSDT".to_string(),
//                                                  side: "buy".to_string(),
//                                                  price: 1060.0,
//                                                  timestamp: 1649196000000000,
//                                                  amount: 1000000000.0 },
//                                    MarketTrade { exchange: "binance-futures".to_string(),
//                                                  symbol: "1000PEPEUSDT".to_string(),
//                                                  side: "buy".to_string(),
//                                                  price: 1200.0,
//                                                  timestamp: 1649199600000000,
//                                                  amount: 1000000000.0 },];
//
//         // 使用函数加载数据
//         let actual_trades = load_json_market_trade();
//
//         // 要实现交替打印实际加载的数据和预期数据，我们可以使用迭代器来同时遍历两个向量。
//         // 这里提供了一种方法，通过使用zip函数结合迭代器来交替打印实际和预期的交易数据。
//         // 如果两个向量的长度不一致，zip将会停在较短的向量结束时。以下是修改后的打印部分代码：
//         // 打印实际加载的数据和预期数据
//         println!("{:-<159}", ""); // 输出长度为30的分隔线
//         println!("{:<<50}  Testing load_json_market_trade()  {:>>52}", "<", ">");
//         for (actual_trade, expected_trade) in actual_trades.iter().zip(expected_trades.iter()) {
//             println!("{:->159}", ""); // 输出长度为30的分隔线
//             println!("| 实际交易 : {:?} |", actual_trade);
//             println!("| 预期交易 : {:?} |", expected_trade);
//         }
//         println!("{:-<159}", ""); // 输出长度为30的分隔线
//                                   // 断言实际数据与预期数据相等
//         assert_eq!(actual_trades, expected_trades);
//     }
// }
