CREATE TABLE binance_futures_trades_aggregated_secs
(
    second_ts       DateTime64(3),
    timestamp       UInt64,
    symbol          String,
    high_price      Float64,
    low_price       Float64,
    volume          Float64,
    trades_count    UInt64
)
    ENGINE = ReplacingMergeTree()
        ORDER BY (second_ts, symbol);

-- 假定表已存在，我们只运行插入数据的部分
INSERT INTO binance_futures_trades_aggregated_secs
SELECT
    toStartOfSecond(toDateTime64(intDiv(timestamp, 1000000), 3)) AS second_ts, -- 将微秒转换为毫秒，并取秒级时间戳
    timestamp, -- 保留原始的timestamp列
    symbol,
    max(price) AS high_price, -- 每秒钟的最高成交价
    min(price) AS low_price,  -- 每秒钟的最低成交价
    sum(amount) AS volume,    -- 每秒钟的累计成交量
    count() AS trades_count   -- 每秒钟的交易次数
FROM binance_futures_trades_secs.binance_futures_trades_2020_12_19_XRPUSDT
GROUP BY
    symbol,
    second_ts,               -- 使用second_ts进行分组
    timestamp                -- 确保timestamp也参与分组，以匹配目标表结构
ORDER BY
    second_ts ASC,          -- 根据秒级时间戳排序
    symbol ASC,              -- 根据交易对排序
    timestamp ASC;           -- 根据时间戳排序