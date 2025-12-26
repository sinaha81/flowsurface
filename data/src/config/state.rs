use super::ScaleFactor;
use super::sidebar::Sidebar;
use super::timezone::UserTimezone;
use crate::layout::WindowSpec;
use crate::{AudioStream, Layout, Theme};

use serde::{Deserialize, Serialize};

/// ساختار مدیریت چیدمان‌ها (Layouts)
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Layouts {
    pub layouts: Vec<Layout>,          // لیست تمامی چیدمان‌های ذخیره شده
    pub active_layout: Option<String>, // نام چیدمان فعال فعلی
}

/// ساختار کلی وضعیت برنامه (Application State) برای ذخیره‌سازی و بازیابی
#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct State {
    pub layout_manager: Layouts,          // مدیریت چیدمان‌ها
    pub selected_theme: Theme,            // تم انتخاب شده
    pub custom_theme: Option<Theme>,      // تم سفارشی (در صورت وجود)
    pub main_window: Option<WindowSpec>,  // مشخصات پنجره اصلی
    pub timezone: UserTimezone,           // منطقه زمانی کاربر
    pub sidebar: Sidebar,                 // تنظیمات نوار کناری
    pub scale_factor: ScaleFactor,        // ضریب مقیاس رابط کاربری
    pub audio_cfg: AudioStream,           // تنظیمات صوتی
    pub trade_fetch_enabled: bool,        // آیا دریافت تاریخچه معاملات فعال است؟
    pub size_in_quote_ccy: exchange::SizeUnit, // واحد نمایش حجم (پایه یا کوت)
}

impl State {
    /// ایجاد یک نمونه جدید از وضعیت برنامه با استفاده از اجزای مختلف
    pub fn from_parts(
        layout_manager: Layouts,
        selected_theme: Theme,
        custom_theme: Option<Theme>,
        main_window: Option<WindowSpec>,
        timezone: UserTimezone,
        sidebar: Sidebar,
        scale_factor: ScaleFactor,
        audio_cfg: AudioStream,
        volume_size_unit: exchange::SizeUnit,
    ) -> Self {
        State {
            layout_manager,
            selected_theme: Theme(selected_theme.0),
            custom_theme: custom_theme.map(|t| Theme(t.0)),
            main_window,
            timezone,
            sidebar,
            scale_factor,
            audio_cfg,
            trade_fetch_enabled: exchange::fetcher::is_trade_fetch_enabled(),
            size_in_quote_ccy: volume_size_unit,
        }
    }
}
