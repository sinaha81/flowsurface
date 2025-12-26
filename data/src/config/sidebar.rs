use serde::{Deserialize, Deserializer, Serialize};

use crate::tickers_table;

/// تنظیمات مربوط به نوار کناری (Sidebar)
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Sidebar {
    pub position: Position,                   // موقعیت نوار کناری (چپ یا راست)
    #[serde(skip)]
    pub active_menu: Option<Menu>,            // منوی فعال فعلی
    #[serde(default)]
    pub tickers_table: Option<tickers_table::Settings>, // تنظیمات جدول نمادها
}

impl Sidebar {
    /// تنظیم منوی فعال جدید
    pub fn set_menu(&mut self, new_menu: Menu) {
        self.active_menu = Some(new_menu);
    }

    /// تنظیم موقعیت نوار کناری
    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    /// بررسی اینکه آیا یک منوی خاص فعال است یا خیر
    pub fn is_menu_active(&self, menu: Menu) -> bool {
        self.active_menu == Some(menu)
    }

    /// همگام‌سازی تنظیمات جدول نمادها
    pub fn sync_tickers_table_settings(&mut self, settings: &tickers_table::Settings) {
        self.tickers_table = Some(settings.clone());
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            position: Position::Left,
            active_menu: None,
            tickers_table: None,
        }
    }
}

/// تابع کمکی برای دی‌سریال‌سازی نوار کناری با مقدار پیش‌فرض در صورت خطا
pub fn deserialize_sidebar_fallback<'de, D>(deserializer: D) -> Result<Sidebar, D::Error>
where
    D: Deserializer<'de>,
{
    Sidebar::deserialize(deserializer).or(Ok(Sidebar::default()))
}

/// موقعیت نوار کناری در صفحه
#[derive(Default, Debug, Clone, PartialEq, Copy, Deserialize, Serialize)]
pub enum Position {
    #[default]
    Left,  // سمت چپ
    Right, // سمت راست
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Left => write!(f, "Left"),
            Position::Right => write!(f, "Right"),
        }
    }
}

/// انواع منوهای موجود در نوار کناری
#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum Menu {
    Layout,      // مدیریت چیدمان
    Settings,    // تنظیمات عمومی
    Audio,       // تنظیمات صوتی
    ThemeEditor, // ویرایشگر تم
}
