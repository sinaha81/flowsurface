use std::fmt;

use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// انواع مناطق زمانی قابل انتخاب توسط کاربر
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum UserTimezone {
    #[default]
    Utc,   // زمان هماهنگ جهانی (UTC)
    Local, // زمان محلی سیستم کاربر
}

impl UserTimezone {
    /// تبدیل برچسب زمانی UTC به منطقه زمانی مناسب و قالب‌بندی آن بر اساس بازه زمانی (Timeframe)
    pub fn format_timestamp(&self, timestamp: i64, timeframe: exchange::Timeframe) -> String {
        if let Some(datetime) = DateTime::from_timestamp(timestamp, 0) {
            match self {
                UserTimezone::Local => {
                    let time_with_zone = datetime.with_timezone(&chrono::Local);
                    Self::format_by_timeframe(&time_with_zone, timeframe)
                }
                UserTimezone::Utc => {
                    let time_with_zone = datetime.with_timezone(&chrono::Utc);
                    Self::format_by_timeframe(&time_with_zone, timeframe)
                }
            }
        } else {
            String::new()
        }
    }

    /// قالب‌بندی یک شیء `DateTime` بر اساس بازه زمانی
    fn format_by_timeframe<Tz: chrono::TimeZone>(
        datetime: &DateTime<Tz>,
        timeframe: exchange::Timeframe,
    ) -> String
    where
        Tz::Offset: std::fmt::Display,
    {
        let interval = timeframe.to_milliseconds();

        if interval < 10000 {
            // برای بازه‌های زمانی بسیار کوتاه (کمتر از ۱۰ ثانیه)
            datetime.format("%M:%S").to_string()
        } else if datetime.format("%H:%M").to_string() == "00:00" {
            // برای شروع روز جدید
            datetime.format("%-d").to_string()
        } else {
            // برای سایر موارد
            datetime.format("%H:%M").to_string()
        }
    }

    /// قالب‌بندی برچسب زمانی برای نمایش در محل نشانگر (Crosshair) با جزئیات بیشتر
    pub fn format_crosshair_timestamp(&self, timestamp_millis: i64, interval: u64) -> String {
        if let Some(datetime) = DateTime::from_timestamp_millis(timestamp_millis) {
            if interval < 10000 {
                return datetime.format("%M:%S.%3f").to_string();
            }

            match self {
                UserTimezone::Local => datetime
                    .with_timezone(&chrono::Local)
                    .format("%a %b %-d %H:%M")
                    .to_string(),
                UserTimezone::Utc => datetime
                    .with_timezone(&chrono::Utc)
                    .format("%a %b %-d %H:%M")
                    .to_string(),
            }
        } else {
            String::new()
        }
    }
}

impl fmt::Display for UserTimezone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserTimezone::Utc => write!(f, "UTC"),
            UserTimezone::Local => {
                let local_offset = chrono::Local::now().offset().local_minus_utc();
                let hours = local_offset / 3600;
                let minutes = (local_offset % 3600) / 60;
                write!(f, "Local (UTC {hours:+03}:{minutes:02})")
            }
        }
    }
}

impl<'de> Deserialize<'de> for UserTimezone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let timezone_str = String::deserialize(deserializer)?;
        match timezone_str.to_lowercase().as_str() {
            "utc" => Ok(UserTimezone::Utc),
            "local" => Ok(UserTimezone::Local),
            _ => Err(serde::de::Error::custom("Invalid UserTimezone")),
        }
    }
}

impl Serialize for UserTimezone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            UserTimezone::Utc => serializer.serialize_str("UTC"),
            UserTimezone::Local => serializer.serialize_str("Local"),
        }
    }
}
