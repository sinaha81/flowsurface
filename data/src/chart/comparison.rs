use exchange::SerTicker;
use serde::{Deserialize, Serialize};

/// تنظیمات مربوط به مقایسه چندین نماد در یک نمودار
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub colors: Vec<(SerTicker, iced_core::Color)>, // رنگ‌های اختصاص داده شده به هر نماد
    pub names: Vec<(SerTicker, String)>,           // نام‌های نمایشی برای هر نماد
}
