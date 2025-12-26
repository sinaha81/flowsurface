use std::path::PathBuf;
use std::{fs, io};

use crate::data_path;

const LOG_FILE: &str = "flowsurface-current.log";

/// ایجاد یا باز کردن فایل لاگ برای نوشتن
pub fn file() -> Result<fs::File, Error> {
    let path = path()?;

    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true) // خالی کردن فایل در هر بار شروع برنامه
        .open(path)?)
}

/// دریافت مسیر کامل فایل لاگ و اطمینان از وجود پوشه مربوطه
pub fn path() -> Result<PathBuf, Error> {
    let full_path = data_path(Some(LOG_FILE));

    let parent = full_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid log file path"))?;

    // ایجاد پوشه لاگ در صورت عدم وجود
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    Ok(full_path)
}

/// انواع خطاهای مربوط به سیستم لاگینگ
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error), // خطاهای ورودی/خروجی
    #[error(transparent)]
    SetLog(#[from] log::SetLoggerError), // خطای تنظیم لاگر
    #[error(transparent)]
    ParseLevel(#[from] log::ParseLevelError), // خطای پارس کردن سطح لاگ
}
