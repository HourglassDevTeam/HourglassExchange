
use std::{
    borrow::Cow,
    fmt::Display,
    hash::{BuildHasher, Hash, Hasher},
    io,
    io::{stderr, Error as IoError, Write},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
pub use std::{
    fs,
    path::{Path, PathBuf},
};

use arc_swap::ArcSwap;
use crossbeam_channel::{bounded, unbounded, Receiver, RecvTimeoutError, Sender, TrySendError};
use hashbrown::HashMap;
pub use log::{debug, error, info, log, log_enabled, logger, trace, warn, Level, LevelFilter, Record};
use log::{kv::Key, set_boxed_logger, set_max_level, Log, Metadata, SetLoggerError};
use time::{format_description::OwnedFormatItem, OffsetDateTime, UtcOffset};

use tm::{duration, now, to_utc, Time};

pub mod appender;

// #[cfg(not(feature = "tsc"))]
mod tm
{
    use super::*;

    pub type Time = std::time::SystemTime;
    #[inline]
    pub fn now() -> Time
    {
        std::time::SystemTime::now()
    }
    #[inline]
    pub fn to_utc(time: Time) -> OffsetDateTime
    {
        time.into()
    }

    #[inline]
    pub fn duration(from: Time, to: Time) -> Duration
    {
        to.duration_since(from).unwrap_or_default()
    }
}

// #[cfg(feature = "tsc")]
// mod tm {
//     use super::*;
//
//     pub type Time = minstant::Instant;
//     #[inline]
//     pub fn now() -> Time {
//         minstant::Instant::now()
//     }
//     #[inline]
//     pub fn to_utc(time: Time) -> OffsetDateTime {
//         static ANCHOR: once_cell::sync::Lazy<minstant::Anchor> = once_cell::sync::Lazy::new(|| minstant::Anchor::new());
//         OffsetDateTime::from_unix_timestamp_nanos(time.as_unix_nanos(&ANCHOR) as i128).unwrap()
//     }
//     #[inline]
//     pub fn duration(from: Time, to: Time) -> Duration {
//         to.duration_since(from)
//     }
// }

#[cfg(target_family = "unix")]
fn local_timezone() -> UtcOffset
{
    UtcOffset::current_local_offset().unwrap_or_else(|_| {
                                         let tz = tz::TimeZone::local().unwrap();
                                         let current_local_time_type = tz.find_current_local_time_type().unwrap();
                                         let diff_secs = current_local_time_type.ut_offset();
                                         UtcOffset::from_whole_seconds(diff_secs).unwrap()
                                     })
}

#[cfg(target_family = "unix")]
/// 手动清理项目文件夹中的所有 .log 文件。
pub fn manual_cleanup(dir: &Path) -> io::Result<()>
{
    let mut found_log_files = false;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // 递归地清理子目录中的日志文件
                manual_cleanup(&path)?;
            }
            else {
                // 检查文件是否是 .log 文件并删除它
                if let Some(ext) = path.extension() {
                    if ext == "log" {
                        found_log_files = true;
                        fs::remove_file(&path)?;
                        println!("[HourglassLog] : 删除了日志文件: {:?}", path);
                    }
                }
            }
        }
    }

    // 如果在整个目录中没有找到任何 .log 文件，打印消息
    if !found_log_files && dir == Path::new(".") {
        println!("[HourglassLog] : 在路径 '{}' 下没有找到 .log 文件。", dir.to_string_lossy());
    }
    Ok(())
}

// #[cfg(not(target_family = "unix"))]
// fn local_timezone() -> UtcOffset {
//     UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC)
// }

struct LogMsg
{
    time: Time,
    msg: Box<dyn Sync + Send + Display>,
    level: Level,
    target: String,
    limit: u32,
    limit_key: u64,
}
impl LogMsg
{
    fn write(self,
             filters: &Vec<Directive>,
             appenders: &mut HashMap<&'static str, Box<dyn Write + Send>>,
             root: &mut Box<dyn Write + Send>,
             root_level: LevelFilter,
             missed_log: &mut HashMap<u64, i64, nohash_hasher::BuildNoHashHasher<u64>>,
             last_log: &mut HashMap<u64, Time, nohash_hasher::BuildNoHashHasher<u64>>,
             offset: Option<UtcOffset>,
             time_format: &OwnedFormatItem)
    {
        let msg = self.msg.to_string();
        if msg.is_empty() {
            return;
        }

        let now = now();

        let writer = if let Some(filter) = filters.iter().find(|x| self.target.starts_with(x.path)) {
            if filter.level.map(|l| l < self.level).unwrap_or(false) {
                return;
            }
            filter.appender.and_then(|n| appenders.get_mut(n)).unwrap_or(root)
        }
        else {
            if root_level < self.level {
                return;
            }
            root
        };

        if self.limit > 0 {
            let missed_entry = missed_log.entry(self.limit_key).or_insert_with(|| 0);
            if let Some(last) = last_log.get(&self.limit_key) {
                if duration(*last, now) < Duration::from_millis(self.limit as u64) {
                    *missed_entry += 1;
                    return;
                }
            }
            last_log.insert(self.limit_key, now);
            // let delay = duration(self.time, now);
            let utc_datetime = to_utc(self.time);

            let offset_datetime = offset.map(|o| utc_datetime.to_offset(o)).unwrap_or(utc_datetime);

            let s = format!("[{}]-[{}]{}\n",
                            offset_datetime.format(&time_format)
                                           .unwrap_or_else(|_| offset_datetime.format(&time::format_description::well_known::Rfc3339).unwrap()),
                            // delay.as_millis(),
                            *missed_entry,
                            msg);
            if let Err(e) = writer.write_all(s.as_bytes()) {
                eprintln!("[HourglassLog] : 日志记录器写入消息失败: {}", e);
            };
            *missed_entry = 0;
        }
        else {
            // let delay = duration(self.time, now);
            let utc_datetime = to_utc(self.time);
            let offset_datetime = offset.map(|o| utc_datetime.to_offset(o)).unwrap_or(utc_datetime);
            let s = format!("[{}]-{}\n",
                            offset_datetime.format(&time_format)
                                           .unwrap_or_else(|_| offset_datetime.format(&time::format_description::well_known::Rfc3339).unwrap()),
                            // delay.as_millis(),
                            msg);
            if let Err(e) = writer.write_all(s.as_bytes()) {
                eprintln!("[HourglassLog] : 日志记录器写入消息失败: {}", e);
            };
        }
    }
}

enum LoggerInput
{
    LogMsg(LogMsg),
    Flush,
}

#[allow(dead_code)]
#[derive(Debug)]
enum LoggerOutput
{
    Flushed,
    FlushError(io::Error),
}

pub trait LogFormat: Send + Sync
{
    /// 将 record 引用转换为 box 对象，然后发送到 log 线程，最后格式化为字符串。
    /// 注：record 是一个记录结构体，包含了日志信息。
    fn msg(&self, record: &Record) -> Box<dyn Send + Sync + Display>;
}

pub struct TideLogFormatter;
impl LogFormat for TideLogFormatter
{
    /// 返回一个 box 对象，它包含用于稍后格式化为字符串的必要数据（例如线程名称、代码行号等）。
    #[inline]
    fn msg(&self, record: &Record) -> Box<dyn Send + Sync + Display>
    {
        Box::new(Message { level: record.level(),
                           thread: std::thread::current().name().map(|n| n.to_string()),
                           file: record.file_static()
                                       .map(|s| Cow::Borrowed(s))
                                       .or_else(|| record.file().map(|s| Cow::Owned(s.to_owned())))
                                       .unwrap_or(Cow::Borrowed("")),
                           line: record.line(),
                           args: record.args().as_str().map(|s| Cow::Borrowed(s)).unwrap_or_else(|| Cow::Owned(format!("{}", record.args()))) })
    }
}

struct Message
{
    level: Level,
    thread: Option<String>,
    file: Cow<'static, str>,
    line: Option<u32>,
    args: Cow<'static, str>,
}

impl Display for Message
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.write_str(&format!("{} {} [{}:{}] {}",
                             self.level,
                             self.thread.as_ref().map(|x| x.as_str()).unwrap_or(""),
                             self.file,
                             self.line.unwrap_or(0),
                             self.args))
    }
}

struct DiscardState
{
    last: ArcSwap<Instant>,
    count: AtomicUsize,
}

// 一个在释放时刷新与 Logger 相关日志的守卫
/// 这个守卫可以确保当应用程序退出时，所有日志都被写入到目的地。
pub struct LoggerGuard
{
    queue: Sender<LoggerInput>,
    notification: Receiver<LoggerOutput>,
}
impl Drop for LoggerGuard
{
    fn drop(&mut self)
    {
        self.queue.send(LoggerInput::Flush).expect("[HourglassLog]：在刷新时日志队列关闭了，这是一个bug");
        self.notification.recv().expect("[HourglassLog]：日志通知已关闭，这是一个bug");
    }
}

pub struct Logger
{
    format: Box<dyn LogFormat>,
    level: LevelFilter,
    queue: Sender<LoggerInput>,
    notification: Receiver<LoggerOutput>,
    block: bool,
    discard_state: Option<DiscardState>,
    stopped: AtomicBool,
}

impl Logger
{
    pub fn init(self) -> Result<LoggerGuard, SetLoggerError>
    {
        let guard = LoggerGuard { queue: self.queue.clone(),
                                  notification: self.notification.clone() };

        set_max_level(self.level);
        let boxed = Box::new(self);
        set_boxed_logger(boxed).map(|_| guard)
    }
}

impl Log for Logger
{
    #[inline]
    fn enabled(&self, metadata: &Metadata) -> bool
    {
        // 已在日志宏中进行了检查
        self.level >= metadata.level()
    }

    fn log(&self, record: &Record)
    {
        let limit = record.key_values().get(Key::from_str("limit")).and_then(|x| x.to_u64()).unwrap_or(0) as u32;

        let msg = self.format.msg(record);
        let limit_key = if limit == 0 {
            0
        }
        else {
            let mut b = hashbrown::hash_map::DefaultHashBuilder::default().build_hasher();
            if let Some(p) = record.module_path() {
                p.as_bytes().hash(&mut b);
            }
            else {
                record.file().unwrap_or("").as_bytes().hash(&mut b);
            }
            record.line().unwrap_or(0).hash(&mut b);
            b.finish()
        };
        let msg = LoggerInput::LogMsg(LogMsg { time: now(),
                                               msg,
                                               target: record.target().to_owned(),
                                               level: record.level(),
                                               limit,
                                               limit_key });
        if self.block {
            if let Err(_) = self.queue.send(msg) {
                let stop = self.stopped.load(Ordering::SeqCst);
                if !stop {
                    eprintln!("[HourglassLog] : 在记录日志时，日志队列关闭了，这是一个bug");
                    self.stopped.store(true, Ordering::SeqCst)
                }
            }
        }
        else {
            match self.queue.try_send(msg) {
                | Err(TrySendError::Full(_)) => {
                    if let Some(s) = &self.discard_state {
                        let count = s.count.fetch_add(1, Ordering::SeqCst);
                        if s.last.load().elapsed().as_secs() >= 5 {
                            eprintln!("[HourglassLog] : 日志消息过多。省略的日志数量：{}", count);
                            s.last.store(Arc::new(Instant::now()));
                        }
                    }
                }
                | Err(TrySendError::Disconnected(_)) => {
                    let stop = self.stopped.load(Ordering::SeqCst);
                    if !stop {
                        eprintln!("[HourglassLog] : 在记录日志时，日志队列关闭了，这是一个bug");
                        self.stopped.store(true, Ordering::SeqCst)
                    }
                }
                | _ => (),
            }
        }
    }

    /// 刷新函数
    /// 该函数用于刷新日志队列。它向日志队列发送一个刷新命令，并等待通知，
    /// 以确保所有待处理的日志消息都已被正确处理。
    fn flush(&self)
    {
        // 向日志队列发送一个刷新命令。如果队列已关闭，则抛出异常。
        self.queue.send(LoggerInput::Flush).expect("[HourglassLog] : 在刷新时日志队列关闭了，这是一个bug");

        // 等待通知，以确认刷新操作已完成。如果通知通道已关闭，则抛出异常。
        self.notification.recv().expect("[HourglassLog] : 日志通知关闭了，这是一个错误");
    }
}

struct BoundedChannelOption
{
    size: usize,
    block: bool,
    print: bool,
}

/// # 本地时区
/// 出于性能考虑，`hourglass_log` 只在一开始获取一次时区信息，并永久使用这个本地时区偏移量。
/// 因此，日志中的时间戳不会意识到操作系统中时区的变化。

pub struct Builder
{
    format: Box<dyn LogFormat>,
    time_format: Option<OwnedFormatItem>,
    level: Option<LevelFilter>,
    root_level: Option<LevelFilter>,
    root: Box<dyn Write + Send>,
    appenders: HashMap<&'static str, Box<dyn Write + Send + 'static>>,
    filters: Vec<Directive>,
    bounded_channel_option: Option<BoundedChannelOption>,
    timezone: LogTimezone,
}
#[inline]
pub fn builder() -> Builder
{
    Builder::new()
}

struct Directive
{
    path: &'static str,
    level: Option<LevelFilter>,
    appender: Option<&'static str>,
}
pub enum LogTimezone
{
    /// local timezone
    /// Only *unix OS is supported for now
    Local,
    Utc,
    Fixed(UtcOffset),
}

impl Builder
{
    #[inline]
    /// 使用默认设置创建一个 hourglass_log 构建器：
    /// - 全局日志级别：信息（INFO）
    /// - 根日志级别：信息（INFO）
    /// - 默认格式器：`TideLogFormatter`
    /// - 输出到标准错误输出（stderr）
    /// - 在工作线程和日志线程之间使用有界通道，大小限制为 100,000
    /// - 丢弃过多的日志消息
    /// - 使用本地时区的时间戳进行日志记录
    pub fn new() -> Builder
    {
        Builder { format: Box::new(TideLogFormatter),
                  level: None,
                  root_level: None,
                  root: Box::new(stderr()) as Box<dyn Write + Send>,
                  appenders: HashMap::new(),
                  filters: Vec::new(),
                  bounded_channel_option: Some(BoundedChannelOption { size: 100_000,
                                                                      block: false,
                                                                      print: false }),
                  timezone: LogTimezone::Local,
                  time_format: None }
    }

    /// Set custom formatter
    #[inline]
    pub fn format<F: LogFormat + 'static>(mut self, format: F) -> Builder
    {
        self.format = Box::new(format);
        self
    }

    #[inline]
    pub fn time_format(mut self, format: OwnedFormatItem) -> Builder
    {
        self.time_format = Some(format);
        self
    }

    /// 工作线程和日志线程之间的有界通道
    ///
    /// 当 `block_when_full` 为真时，它将阻塞当前线程（即调用日志宏的线程，例如 `log::info`），
    /// 直到日志线程能够处理新消息为止。否则，过多的日志消息将被丢弃。
    ///
    /// 默认情况下，过多的日志消息会被静默丢弃。要显示已丢弃的日志消息数量，
    /// 可参见 `Builder::print_omitted_count()`。
    #[inline]
    pub fn bounded(mut self, size: usize, block_when_full: bool) -> Builder
    {
        self.bounded_channel_option = Some(BoundedChannelOption { size,
                                                                  block: block_when_full,
                                                                  print: false });
        self
    }

    /// 当日志线程的通道是有界的，并且设置为丢弃过多的日志消息时，是否打印被省略的日志数量。
    #[inline]
    pub fn print_omitted_count(mut self, print: bool) -> Builder
    {
        self.bounded_channel_option.as_mut().map(|o| o.print = print);
        self
    }

    /// 设置通道大小为无限制
    /// **注意**：过多的日志消息将导致巨大的内存消耗，因为日志消息会排队等待日志线程处理。
    /// 当日志消息超过当前通道大小时，默认会将大小翻倍， 由于通道扩展需要分配内存，日志调用可能会变慢。
    #[inline]
    pub fn unbounded(mut self) -> Builder
    {
        self.bounded_channel_option = None;
        self
    }

    /// 添加一个带有名称的附加器
    ///
    /// 结合 `Builder::filter()` 使用，hourglass_log 可以将不同模块路径的日志输出到不同的目标。
    #[inline]
    pub fn appender(mut self, name: &'static str, appender: impl Write + Send + 'static) -> Builder
    {
        self.appenders.insert(name, Box::new(appender));
        self
    }

    /// 添加一个过滤器来重定向日志到不同的输出目标
    /// （例如，标准错误输出 stderr、标准输出 stdout、不同的文件）。
    ///
    /// **注意**：比 `Builder::max_log_level` 更详细的日志级别将被忽略。
    /// 假设我们将 `max_log_level` 配置为信息级别 INFO，即使过滤器的级别设置为调试级别 DEBUG，
    /// hourglass_log 仍将只记录到信息级别 INFO。
    #[inline]
    pub fn filter<A: Into<Option<&'static str>>, L: Into<Option<LevelFilter>>>(mut self, module_path: &'static str, appender: A, level: L) -> Builder
    {
        let appender = appender.into();
        let level = level.into();
        if appender.is_some() || level.is_some() {
            self.filters.push(Directive { path: module_path, appender, level });
        }
        self
    }

    #[inline]
    /// 配置默认的日志输出目标。
    /// 如果省略此方法，日志将输出到标准错误输出（stderr）。
    pub fn root(mut self, writer: impl Write + Send + 'static) -> Builder
    {
        self.root = Box::new(writer);
        self
    }

    #[inline]
    /// 设置最大日志级别
    /// 比此级别更详细的日志将不会被发送到日志线程。
    pub fn max_log_level(mut self, level: LevelFilter) -> Builder
    {
        self.level = Some(level);
        self
    }

    #[inline]
    pub fn root_log_level(mut self, level: LevelFilter) -> Builder
    {
        self.root_level = Some(level);
        self
    }

    /// 使用本地时区的时间戳进行日志记录
    /// 日志记录器设置后时区将被固定，原因如下：
    /// 1. 目前 `time` v0.3 版本不支持在类 Unix 操作系统的多线程进程中访问本地偏移量。
    /// 2. 从操作系统获取时区信息相对较慢（大约几微秒），与获取 UTC 时间戳（大约几十纳秒）相比。
    #[inline]
    pub fn local_timezone(mut self) -> Builder
    {
        self.timezone = LogTimezone::Local;
        self
    }

    #[inline]
    /// 使用 UTC 时区的 timestamps 记录日志
    pub fn utc(mut self) -> Builder
    {
        self.timezone = LogTimezone::Utc;
        self
    }

    #[inline]
    /// 使用固定时区 (例如: UTC) 的时间戳记录日志
    pub fn fixed_timezone(mut self, timezone: UtcOffset) -> Builder
    {
        self.timezone = LogTimezone::Fixed(timezone);
        self
    }

    #[inline]
    /// 指定日志消息的时间戳时区
    pub fn timezone(mut self, timezone: LogTimezone) -> Builder
    {
        self.timezone = timezone;
        self
    }

    /// 完成TideLog记录器的构建。
    /// 此调用会启动一个日志线程，将日志消息格式化为字符串，然后写入输出目标。
    pub fn build(self) -> Result<Logger, IoError>
    {
        let offset = match self.timezone {
            | LogTimezone::Local => Some(local_timezone()),
            | LogTimezone::Utc => None,
            | LogTimezone::Fixed(offset) => Some(offset),
        };
        let time_format = self.time_format
                              .unwrap_or_else(|| time::format_description::parse_owned::<1>("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]+[offset_hour]").unwrap());
        let mut filters = self.filters;
        // 确保根据最长路径匹配，对过滤器的路径进行排序。
        filters.sort_by(|a, b| a.path.len().cmp(&b.path.len()));
        filters.reverse();
        // 检查过滤器中的appender names是否有效
        for appender_name in filters.iter().filter_map(|x| x.appender) {
            if !self.appenders.contains_key(appender_name) {
                panic!("[HourglassLog] : Appender {} not configured", appender_name);
            }
        }
        let global_level = self.level.unwrap_or(LevelFilter::Info);
        let root_level = self.root_level.unwrap_or(global_level);
        if global_level < root_level {
            warn!("[HourglassLog] : 日志级别高于 {} 的将被忽略", global_level,);
        }

        let (sync_sender, receiver) = match &self.bounded_channel_option {
            | None => unbounded(),
            | Some(option) => bounded(option.size),
        };
        let (notification_sender, notification_receiver) = bounded(1);
        std::thread::Builder::new().name("logger".to_string()).spawn(move || {
                                                                   let mut appenders = self.appenders;
                                                                   let filters = filters;

                                                                   for filter in &filters {
                                                                       if let Some(level) = filter.level {
                                                                           if global_level < level {
                                                                               warn!("[HourglassLog] : 在 `{}` 中，日志级别高于 {} 的消息将被忽略。", global_level, filter.path,);
                                                                           }
                                                                       }
                                                                   }

                                                                   let mut root = self.root;
                                                                   let mut last_log = HashMap::default();
                                                                   let mut missed_log = HashMap::default();
                                                                   let mut last_flush = Instant::now();
                                                                   let timeout = Duration::from_millis(200);
                                                                   loop {
                                                                       match receiver.recv_timeout(timeout) {
                                                                           | Ok(LoggerInput::LogMsg(log_msg)) => {
                                                                               log_msg.write(&filters, &mut appenders, &mut root, root_level, &mut missed_log, &mut last_log, offset, &time_format);
                                                                           }
                                                                           | Ok(LoggerInput::Flush) => {
                                                                               let max = receiver.len();
                                                                               'queue: for _ in 1..=max {
                                                                                   if let Ok(LoggerInput::LogMsg(msg)) = receiver.try_recv() {
                                                                                       msg.write(&filters, &mut appenders, &mut root, root_level, &mut missed_log, &mut last_log, offset, &time_format)
                                                                                   }
                                                                                   else {
                                                                                       break 'queue;
                                                                                   }
                                                                               }
                                                                               let flush_result = appenders.values_mut().chain([&mut root]).find_map(|w| w.flush().err());
                                                                               if let Some(error) = flush_result {
                                                                                   notification_sender.send(LoggerOutput::FlushError(error)).expect("[HourglassLog] : 日志通知失败");
                                                                               }
                                                                               else {
                                                                                   notification_sender.send(LoggerOutput::Flushed).expect("[HourglassLog] : 日志通知失败");
                                                                               }
                                                                           }
                                                                           | Err(RecvTimeoutError::Timeout) => {
                                                                               if last_flush.elapsed() > Duration::from_millis(1000) {
                                                                                   let flush_errors = appenders.values_mut().chain([&mut root]).filter_map(|w| w.flush().err());
                                                                                   for err in flush_errors {
                                                                                       warn!("HourglassLog flush error: {}", err);
                                                                                   }
                                                                                   last_flush = Instant::now();
                                                                               };
                                                                           }
                                                                           | Err(e) => {
                                                                               eprintln!("[HourglassLog] : sender 关闭，但没有发送 quit 信号，请检查详细信息：{}，这可能有助于发现潜在的错误", e);
                                                                           }
                                                                       }
                                                                   }
                                                               })?;
        let block = self.bounded_channel_option.as_ref().map(|x| x.block).unwrap_or(false);
        let print = self.bounded_channel_option.as_ref().map(|x| x.print).unwrap_or(false);
        Ok(Logger { format: self.format,
                    level: global_level,
                    queue: sync_sender,
                    notification: notification_receiver,
                    block,
                    discard_state: if block || !print {
                        None
                    }
                    else {
                        Some(DiscardState { last: ArcSwap::new(Arc::new(Instant::now())),
                                            count: AtomicUsize::new(0) })
                    },
                    stopped: AtomicBool::new(false) })
    }

    /// 尝试构建并设置成全局日志记录器。
    pub fn try_init(self) -> Result<LoggerGuard, Box<dyn std::error::Error>>
    {
        let logger = self.build()?;
        Ok(logger.init()?)
    }
}

impl Default for Builder
{
    #[inline]
    fn default() -> Self
    {
        Builder::new()
    }
}
