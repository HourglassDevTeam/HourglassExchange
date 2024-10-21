
use log::info;
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use time::{Date, Duration, Month, OffsetDateTime, Time, UtcOffset};
use typed_builder::TypedBuilder;

use crate::hourglass_log::{local_timezone, LogTimezone};

/// 日志轮换频率
#[derive(Clone, Copy)]
pub enum Period
{
    /// 每分钟轮换日志
    Minute,
    /// 每小时轮换日志
    Hour,
    /// 每天轮换日志
    Day,
    /// 每月轮换日志
    Month,
    /// 每年轮换日志
    Year,
}

struct Rotate
{
    start: Instant,
    wait: Duration,
    period: Period,
    expire: Option<Duration>,
}

#[derive(TypedBuilder)]
#[builder(build_method(vis = "", name = __build), builder_method(vis = ""))]
pub struct FileAppenderBuilder
{
    #[builder(setter(transform = |x: impl AsRef<Path>| x.as_ref().to_path_buf()))]
    path: PathBuf,
    #[builder(default, setter(into))]
    rotate: Option<Period>,
    #[builder(default, setter(into))]
    expire: Option<Duration>,
    #[builder(default=LogTimezone::Local)]
    timezone: LogTimezone,
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<__rotate: typed_builder::Optional<Option<Period>>, __expire: typed_builder::Optional<Option<Duration>>, __timezone: typed_builder::Optional<LogTimezone>>
    FileAppenderBuilderBuilder<((PathBuf,), __rotate, __expire, __timezone)>
{
    pub fn build(self) -> FileAppender
    {
        let builder = self.__build();
        match (builder.rotate, builder.expire) {
            // 轮换并自动清理
            | (Some(period), Some(expire)) => {
                let (start, wait) = FileAppender::until(period, &builder.timezone);
                let path = FileAppender::file(&builder.path, period, &builder.timezone);
                let file = BufWriter::new(OpenOptions::new().create(true).append(true).open(&path).unwrap());
                FileAppender { file,
                               path: builder.path,
                               rotate: Some(Rotate { start,
                                                     wait,
                                                     period,
                                                     expire: Some(expire) }),
                               timezone: builder.timezone }
            }
            // 仅轮换
            | (Some(period), None) => {
                let (start, wait) = FileAppender::until(period, &builder.timezone);
                let path = FileAppender::file(&builder.path, period, &builder.timezone);
                let file = BufWriter::new(OpenOptions::new().create(true).append(true).open(&path).unwrap());
                FileAppender { file,
                               path: builder.path,
                               rotate: Some(Rotate { start, wait, period, expire: None }),
                               timezone: builder.timezone }
            }
            // 单一文件
            | _ => FileAppender { file: BufWriter::new(OpenOptions::new().create(true)
                                                                         .append(true)
                                                                         .open(&builder.path)
                                                                         .expect(&format!("创建日志文件失败: {}", builder.path.to_string_lossy()))),
                                  path: builder.path,
                                  rotate: None,
                                  timezone: builder.timezone },
        }
    }
}

/// 将日志输出到本地文件的Appender
pub struct FileAppender
{
    file: BufWriter<File>,
    path: PathBuf,
    rotate: Option<Rotate>,
    timezone: LogTimezone,
}

impl FileAppender
{

    pub fn builder() -> FileAppenderBuilderBuilder
    {
        FileAppenderBuilder::builder()
    }

    fn file<T: AsRef<Path>>(path: T, period: Period, timezone: &LogTimezone) -> PathBuf
    {
        let p = path.as_ref();
        let dt = OffsetDateTime::now_utc().to_offset(Self::offset_from_timezone(timezone));
        let ts = match period {
            | Period::Year => format!("{}", dt.year()),
            | Period::Month => format!("{}{:02}", dt.year(), dt.month() as u8),
            | Period::Day => format!("{}{:02}{:02}", dt.year(), dt.month() as u8, dt.day()),
            | Period::Hour => format!("{}{:02}{:02}T{:02}", dt.year(), dt.month() as u8, dt.day(), dt.hour()),
            | Period::Minute => format!("{}{:02}{:02}T{:02}{:02}", dt.year(), dt.month() as u8, dt.day(), dt.hour(), dt.minute()),
        };

        if let Some(ext) = p.extension() {
            let file_name = p.file_stem().map(|x| format!("{}-{}.{}", x.to_string_lossy(), ts, ext.to_string_lossy())).expect("无效的文件名");
            p.with_file_name(file_name)
        }
        else {
            p.with_file_name(format!("{}-{}", p.file_name().map(|x| x.to_string_lossy()).unwrap_or(Cow::from("log")), ts))
        }
    }

    fn offset_from_timezone(timezone: &LogTimezone) -> UtcOffset
    {
        match timezone {
            | LogTimezone::Local => local_timezone(),
            | LogTimezone::Utc => UtcOffset::UTC,
            | LogTimezone::Fixed(offset) => offset.clone(),
        }
    }

    fn until(period: Period, timezone: &LogTimezone) -> (Instant, Duration)
    {
        let tm_now = OffsetDateTime::now_utc().to_offset(Self::offset_from_timezone(timezone));
        let now = Instant::now();
        let tm_next = Self::next(&tm_now, period);
        (now, tm_next - tm_now)
    }

    #[inline]
    fn next(now: &OffsetDateTime, period: Period) -> OffsetDateTime
    {
        let tm_next = match period {
            | Period::Year => Date::from_ordinal_date(now.year() + 1, 1).unwrap().with_time(Time::MIDNIGHT),
            | Period::Month => {
                let year = if now.month() == Month::December { now.year() + 1 } else { now.year() };
                Date::from_calendar_date(year, now.month().next(), 1).unwrap().with_time(Time::MIDNIGHT)
            }
            | Period::Day => now.date().with_time(Time::MIDNIGHT) + Duration::DAY,
            | Period::Hour => now.date().with_hms(now.time().hour(), 0, 0).unwrap() + Duration::HOUR,
            | Period::Minute => {
                let time = now.time();
                now.date().with_hms(time.hour(), time.minute(), 0).unwrap() + Duration::MINUTE
            }
        };
        tm_next.assume_offset(now.offset())
    }

    /// 创建一个将日志写入文件的文件Appender
    pub fn new<T: AsRef<Path>>(path: T) -> Self
    {
        Self::builder().path(path).build()
    }

    /// 创建一个每给定周期轮换新文件的文件Appender
    pub fn rotate<T: AsRef<Path>>(path: T, period: Period) -> Self
    {
        Self::builder().path(path).rotate(period).build()
    }

    /// 创建一个每给定周期轮换新文件的文件Appender，
    /// 并自动删除在给定的`keep`参数之前最后修改的日志。
    pub fn rotate_with_expire<T: AsRef<Path>>(path: T, period: Period, keep: Duration) -> Self
    {
        Self::builder().path(path).rotate(period).expire(keep).build()
    }
}

impl Write for FileAppender
{
    fn write(&mut self, record: &[u8]) -> std::io::Result<usize>
    {
        if let Some(Rotate { start, wait, period, expire: keep }) = &mut self.rotate {
            if start.elapsed() > *wait {
                // 关闭当前文件并创建新文件
                self.file.flush()?;
                let path = Self::file(&self.path, *period, &self.timezone);
                // 删除过期日志文件
                if let Some(keep_duration) = keep {
                    let keep_duration = keep_duration.clone();
                    let dir = self.path.parent().unwrap().to_path_buf();
                    let dir = if dir.is_dir() { dir } else { PathBuf::from(".") };
                    let path = self.path.clone();
                    let period = period.clone();
                    std::thread::spawn(move || {
                        let to_remove = std::fs::read_dir(dir).unwrap()
                                                              .filter_map(|f| f.ok())
                                                              .filter(|x| x.file_type().map(|x| x.is_file()).unwrap_or(false))
                                                              .filter(|x| {
                                                                  let p = x.path();
                                                                  let name = p.file_stem().unwrap().to_string_lossy();
                                                                  if let Some((stem, time)) = name.rsplit_once("-") {
                                                                      let check = |(ix, x): (usize, char)| match ix {
                                                                          | 8 => x == 'T',
                                                                          | _ => x.is_digit(10),
                                                                      };
                                                                      let len = match period {
                                                                          | Period::Minute => time.len() == 13,
                                                                          | Period::Hour => time.len() == 11,
                                                                          | Period::Day => time.len() == 8,
                                                                          | Period::Month => time.len() == 6,
                                                                          | Period::Year => time.len() == 4,
                                                                      };
                                                                      len && time.chars().enumerate().all(check) && path.file_stem().map(|x| x.to_string_lossy() == stem).unwrap_or(false)
                                                                  }
                                                                  else {
                                                                      false
                                                                  }
                                                              })
                                                              .filter(|x| {
                                                                  x.metadata()
                                                                   .ok()
                                                                   .and_then(|x| x.modified().ok())
                                                                   .map(|time| time.elapsed().map(|elapsed| elapsed > keep_duration).unwrap_or(false))
                                                                   .unwrap_or(false)
                                                              });

                        let del_msg = to_remove.filter(|f| std::fs::remove_file(f.path()).is_ok())
                                               .map(|x| x.file_name().to_string_lossy().to_string())
                                               .collect::<Vec<_>>()
                                               .join(", ");
                        if !del_msg.is_empty() {
                            info!("日志文件已删除: {}", del_msg);
                        }
                    });
                };

                // 轮换文件
                self.file = BufWriter::new(OpenOptions::new().create(true).append(true).open(path).unwrap());
                (*start, *wait) = Self::until(*period, &self.timezone);
            }
        };
        self.file.write_all(record).map(|_| record.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()>
    {
        self.file.flush()
    }
}

#[cfg(test)]
mod test
{
    use super::*;

    fn format(time: OffsetDateTime) -> String
    {
        format!("{:0>4}-{:0>2}-{:0>2}T{:0>2}:{:0>2}:{:0>2}.{:0>3}",
                time.year(),
                time.month() as u8,
                time.day(),
                time.hour(),
                time.minute(),
                time.second(),
                time.millisecond())
    }

    #[test]
    fn to_wait_ms()
    {
        // 2023年10月24日16:00:00 GMT+0000
        let now = OffsetDateTime::from_unix_timestamp(1666627200).unwrap();

        let tm_next = FileAppender::next(&now, Period::Year);
        let tm = OffsetDateTime::from_unix_timestamp(1672531200).unwrap();
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        let tm_next = FileAppender::next(&now, Period::Month);
        let tm = OffsetDateTime::from_unix_timestamp(1667260800).unwrap();
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        let tm_next = FileAppender::next(&now, Period::Day);
        let tm = OffsetDateTime::from_unix_timestamp(1666656000).unwrap();
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        let tm_next = FileAppender::next(&now, Period::Hour);
        let tm = OffsetDateTime::from_unix_timestamp(1666630800).unwrap();
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        let tm_next = FileAppender::next(&now, Period::Minute);
        let tm = OffsetDateTime::from_unix_timestamp(1666627260).unwrap();
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        // 边缘情况：每月的最后一天
        let date = Date::from_calendar_date(2023, Month::January, 31).unwrap();
        let dt = date.with_time(Time::MIDNIGHT).assume_offset(now.offset());
        let tm_next = FileAppender::next(&dt, Period::Day);
        let tm = dt + Duration::DAY;
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));

        // 边缘情况：每年的最后一个月
        let date = Date::from_calendar_date(2023, Month::December, 1).unwrap();
        let dt = date.with_time(Time::MIDNIGHT).assume_offset(now.offset());
        let tm_next = FileAppender::next(&dt, Period::Month);
        let tm = Date::from_calendar_date(2024, Month::January, 1).unwrap().with_hms(0, 0, 0).unwrap().assume_offset(now.offset());
        assert_eq!(tm_next, tm, "{} != {}", format(now), format(tm_next));
    }
}
