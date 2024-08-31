CREATE TABLE binance_futures_combined_aggregated_secs
    ENGINE = MergeTree()
        ORDER BY (book_second_ts, symbol)
        PRIMARY KEY (book_second_ts, symbol)
AS
SELECT book.second_ts  AS book_second_ts,
       trade.second_ts AS trade_second_ts,
       book.timestamp  AS book_timestamp,
       trade.timestamp AS trade_timestamp,
       trade.symbol,
       trade.high_price,
       trade.low_price,
       trade.volume,
       trade.trades_count,
       book.*
FROM binance_futures_book_snapshot_25_secs.binance_futures_book_snapshot_25_aggregated AS book
         JOIN
     binance_futures_trades_secs.binance_futures_trades_aggregated_secs AS trade
     ON
         book.second_ts = trade.second_ts
