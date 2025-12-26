use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process,
    sync::mpsc,
    thread,
};

pub use data::log::Error;

// حداکثر اندازه فایل لاگ (50 مگابایت)
const MAX_LOG_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// انواع پیام‌های لاگ که به ترد پس‌زمینه ارسال می‌شوند
enum LogMessage {
    Content(Vec<u8>), // محتوای لاگ
    Flush,            // درخواست تخلیه بافر (Flush)
    Shutdown,         // درخواست توقف لاگر
}

/// راه‌اندازی سیستم لاگینگ برنامه
pub fn setup(is_debug: bool) -> Result<(), Error> {
    // تعیین سطح لاگ پیش‌فرض بر اساس وضعیت دیباگ
    let default_level = if is_debug {
        log::Level::Debug
    } else {
        log::Level::Info
    };

    // دریافت سطح لاگ از متغیر محیطی RUST_LOG در صورت وجود
    let level_filter = std::env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::parse::<log::Level>)
        .transpose()?
        .unwrap_or(default_level)
        .to_level_filter();

    // تنظیم فرمت نمایش لاگ‌ها
    let mut io_sink = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "{}:{} -- {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            record.level(),
            message
        ));
    });

    if is_debug {
        // در حالت دیباگ، لاگ‌ها در کنسول نمایش داده می‌شوند
        io_sink = io_sink.chain(std::io::stdout());
    } else {
        // در حالت عادی، لاگ‌ها در فایل ذخیره می‌شوند
        let log_path = data::log::path()?;
        initial_rotation(&log_path)?; // چرخش فایل‌های لاگ قدیمی

        let logger: Box<dyn Write + Send> = Box::new(BackgroundLogger::new(log_path)?);

        io_sink = io_sink.chain(logger);
    }

    // تنظیم سطوح لاگ برای ماژول‌های مختلف
    fern::Dispatch::new()
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("data", level_filter)
        .level_for("exchange", level_filter)
        .level_for("flowsurface", level_filter)
        .chain(io_sink)
        .apply()?;

    Ok(())
}

/// مدیریت چرخش اولیه فایل لاگ (انتقال لاگ فعلی به فایل قبلی)
fn initial_rotation(log_path: &PathBuf) -> io::Result<()> {
    let path = PathBuf::from(".");

    let dir = log_path.parent().unwrap_or(&path);

    let previous_log_path = dir.join("flowsurface-previous.log");

    // حذف فایل لاگ قبلی در صورت وجود
    if previous_log_path.exists() {
        fs::remove_file(&previous_log_path)?;
    }

    // تغییر نام فایل لاگ فعلی به فایل قبلی
    if log_path.exists() {
        fs::rename(log_path, &previous_log_path)?;
    }

    Ok(())
}

/// لاگری که در یک ترد مجزا (پس‌زمینه) عمل نوشتن را انجام می‌دهد تا برنامه اصلی متوقف نشود
struct BackgroundLogger {
    sender: mpsc::Sender<LogMessage>,       // ارسال‌کننده پیام به ترد لاگر
    _thread_handle: thread::JoinHandle<()>, // هندل ترد لاگر
}

impl BackgroundLogger {
    /// ایجاد یک لاگر پس‌زمینه جدید
    fn new(path: PathBuf) -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel();

        let thread_handle = thread::Builder::new()
            .name("logger-thread".to_string())
            .spawn(move || {
                let mut logger = match Logger::new(&path) {
                    Ok(logger) => logger,
                    Err(e) => {
                        eprintln!("Failed to initialize logger: {}", e);
                        return;
                    }
                };

                // حلقه اصلی ترد لاگر برای دریافت و پردازش پیام‌ها
                loop {
                    match receiver.recv() {
                        Ok(LogMessage::Content(data)) => {
                            if let Err(e) = logger.write_all(&data) {
                                eprintln!("Logging error: {}", e);
                            }
                        }
                        Ok(LogMessage::Flush) => {
                            if let Err(e) = logger.flush() {
                                eprintln!("Error flushing logs: {}", e);
                            }
                        }
                        Ok(LogMessage::Shutdown) | Err(_) => break,
                    }
                }
            })?;

        Ok(BackgroundLogger {
            sender,
            _thread_handle: thread_handle,
        })
    }
}

impl Write for BackgroundLogger {
    /// ارسال داده برای نوشتن در فایل به صورت غیرهمزمان
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        self.sender
            .send(LogMessage::Content(buf.to_vec()))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Logger thread disconnected"))?;
        Ok(len)
    }

    /// ارسال درخواست Flush به ترد لاگر
    fn flush(&mut self) -> io::Result<()> {
        self.sender
            .send(LogMessage::Flush)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Logger thread disconnected"))?;
        Ok(())
    }
}

impl Drop for BackgroundLogger {
    /// ارسال سیگنال خاموشی به ترد لاگر هنگام حذف شیء
    fn drop(&mut self) {
        let _ = self.sender.send(LogMessage::Shutdown);
    }
}

/// ساختار داخلی برای مدیریت فایل لاگ و اندازه آن
struct Logger {
    file: fs::File,
    current_size: u64,
}

impl Logger {
    /// ایجاد یک نمونه جدید از لاگر فایل
    fn new(path: &PathBuf) -> io::Result<Self> {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        let size = file.metadata()?.len();

        Ok(Logger {
            file,
            current_size: size,
        })
    }
}

impl Write for Logger {
    /// نوشتن داده در فایل با کنترل حداکثر اندازه مجاز
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_len = buf.len() as u64;

        // اگر اندازه فایل از حد مجاز فراتر رود، برنامه متوقف می‌شود
        if self.current_size + buf_len > MAX_LOG_FILE_SIZE {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let error_msg = format!(
                "\n{}:FATAL -- Log file size would exceed the maximum allowed size of {} bytes\n",
                timestamp, MAX_LOG_FILE_SIZE
            );

            eprintln!("{error_msg}");

            let _ = self.file.write_all(error_msg.as_bytes());
            let _ = self.file.flush();

            process::abort();
        }

        let bytes = self.file.write(buf)?;
        self.current_size += bytes as u64;

        Ok(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
