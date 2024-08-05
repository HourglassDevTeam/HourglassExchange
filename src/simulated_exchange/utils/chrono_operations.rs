use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc};
use regex::Regex;

#[allow(dead_code)]
pub fn extract_date(table_name: &str) -> Option<String> {
    // 定义正则表达式模式
    let binance_pattern = Regex::new(r"(?i)binance_.+_(\d{4}_\d{2}_\d{2})").unwrap();
    let okex_pattern = Regex::new(r"(?i)okex_.+_(\d{4}_\d{2}_\d{2})").unwrap();

    // 根据不同的交易所模式匹配并提取日期
    if table_name.starts_with("binance") {
        if let Some(caps) = binance_pattern.captures(table_name) {
            return Some(caps[1].to_string());
        }
    } else if table_name.starts_with("okex") {
        if let Some(caps) = okex_pattern.captures(table_name) {
            return Some(caps[1].to_string());
        }
    }
    // 如果不匹配任何模式，返回 None
    None
}
#[allow(dead_code)]
// 定义一个函数，接受UNIX时间戳并返回东八区精确时间
pub fn local_datetime_from_unix(unix_time: i64) -> DateTime<Local> {
    // 将UNIX时间戳转换为UTC时间
    let utc_datetime = Utc.timestamp_millis_opt(unix_time).unwrap();

    // 将UTC时间转换为东八区时间
    let east_eight_datetime = Local.from_utc_datetime(&utc_datetime.naive_utc());

    east_eight_datetime
}
#[allow(dead_code)]
// 定义一个函数，接受short UNIX时间戳并返回东八区精确时间
pub fn local_datetime_from_short_unix(unix_time: i64) -> DateTime<Local> {
    // 将UNIX时间戳转换为UTC时间
    let utc_datetime = DateTime::<Utc>::from_timestamp(unix_time, 0).unwrap();

    // 将UTC时间转换为东八区时间
    let east_eight_datetime = Local.from_utc_datetime(&utc_datetime.naive_utc());

    east_eight_datetime
}
#[allow(dead_code)]
// 定义一个函数，接受UNIX时间戳并返回东八区小时数
pub fn local_hour_from_unix(unix_time: i64) -> u32 {
    // 将UNIX时间戳转换为UTC时间
    let local_datetime = local_datetime_from_unix(unix_time);

    // 提取小时部分
    local_datetime.hour()
}
#[allow(dead_code)]
pub fn local_minute_from_unix(unix_time: i64) -> u32 {
    // 将UNIX时间戳转换为UTC时间
    let local_datetime = local_datetime_from_unix(unix_time);

    // 提取小时部分
    local_datetime.minute()
}

/// [注意] 此处返回时间格式为: 20220312
#[allow(dead_code)]
pub fn local_date_from_unix(unix_time: i64) -> u32 {
    // 将UNIX时间戳转换为UTC时间
    let local_datetime = local_datetime_from_unix(unix_time);

    // 提取日期部分
    let year = local_datetime.year() as u32;

    let month = local_datetime.month();

    let day = local_datetime.day();

    // 将年、月、日组合成一个u32整数，形如20220312
    year * 10000 + month * 100 + day
}
#[allow(dead_code)]
pub fn expand_date_str(input_str: &str) -> String {
    if input_str.len() == 8 {
        // 确保输入字符串的长度是 8
        let year = &input_str[0..4];

        let month = &input_str[4..6];

        let day = &input_str[6..8];

        // 构建格式化后的日期时间字符串
        let formatted_date_str = format!("{}-{}-{} 00:00:00", year, month, day);

        formatted_date_str
    } else {
        // 如果输入字符串不是有效的日期格式，可以返回错误消息或默认值，根据需要
        "Invalid Date".to_string()
    }
}

/// TODO: parse date string to unix timestamp
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_datetime_from_unix() {
        let unix_time: i64 = 1634817600000; // Replace with your desired UNIX timestamp
        let local_datetime = local_datetime_from_unix(unix_time);

        assert_eq!(local_datetime.to_string(), "2021-10-21 20:00:00 +08:00");
    }

    #[test]
    fn test_local_datetime_from_short_unix() {
        let short_unix_time: i64 = 1634817600; // Replace with your desired short UNIX timestamp
        let short_local_datetime = local_datetime_from_short_unix(short_unix_time);

        assert_eq!(short_local_datetime.to_string(), "2021-10-21 20:00:00 +08:00");
    }

    #[test]
    fn test_local_hour_from_unix() {
        let unix_time: i64 = 1634817600000; // Replace with your desired UNIX timestamp
        let hour = local_hour_from_unix(unix_time);

        assert_eq!(hour, 20);
    }

    #[test]
    fn test_local_date_from_unix() {
        let unix_time: i64 = 1634817600000; // Replace with your desired UNIX timestamp
        let date = local_date_from_unix(unix_time);

        assert_eq!(date, 20211021);
    }

    #[test]
    fn test_convert_date_str() {
        let date_str = "20230314"; // 你的输入日期字符串
        let formatted_date_str = expand_date_str(date_str);

        assert_eq!(formatted_date_str, "2023-03-14 00:00:00");
    }

    #[test]
    fn test_local_minute_from_unix() {
        let unix_time: i64 = 1634817600000; // Replace with your desired UNIX timestamp
        let local_datetime = local_datetime_from_unix(unix_time);

        let minute = local_minute_from_unix(unix_time);

        assert_eq!(local_datetime.minute(), minute);
    }

    #[test]
    fn test_extract_date_binance() {
        let file_name = "binance_futures_trades_2024_07_03_ZRXUSDT";
        let date = extract_date(file_name);
        assert_eq!(date, Some("2024_07_03".to_string()));
    }

    #[test]
    fn test_extract_date_okex() {
        let file_name = "okex_swap_trades_2024_03_03_BTC_USDT_SWAP";
        let date = extract_date(file_name);
        assert_eq!(date, Some("2024_03_03".to_string()));
    }

    #[test]
    fn test_extract_date_invalid_binance() {
        let file_name = "binance_futures_trades_invalid_ZRXUSDT";
        let date = extract_date(file_name);
        assert_eq!(date, None);
    }

    #[test]
    fn test_extract_date_invalid_okex() {
        let file_name = "okex_swap_trades_invalid_BTC_USDT_SWAP";
        let date = extract_date(file_name);
        assert_eq!(date, None);
    }

    #[test]
    fn test_extract_date_unrecognized_exchange() {
        let file_name = "unrecognized_futures_trades_2024_07_03_ZRXUSDT";
        let date = extract_date(file_name);
        assert_eq!(date, None);
    }
}
