pub use dashboard::Dashboard;
pub use pane::Pane;
use serde::{Deserialize, Serialize};

// ماژول‌های مربوط به داشبورد و پنل‌ها
pub mod dashboard;
pub mod pane;

/// ساختار نگهدارنده اطلاعات یک چیدمان (Layout)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,         // نام چیدمان
    pub dashboard: Dashboard, // داشبورد مربوط به این چیدمان
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            dashboard: Dashboard::default(),
        }
    }
}

/// ساختار نگهدارنده ابعاد و موقعیت یک پنجره
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Window<T = f32> {
    pub width: T, // عرض پنجره
    pub height: T, // ارتفاع پنجره
    pub pos_x: T, // موقعیت افقی (X)
    pub pos_y: T, // موقعیت عمودی (Y)
}

impl<T: Copy> Window<T> {
    pub fn size(&self) -> iced_core::Size<T> {
        iced_core::Size {
            width: self.width,
            height: self.height,
        }
    }

    pub fn position(&self) -> iced_core::Point<T> {
        iced_core::Point {
            x: self.pos_x,
            y: self.pos_y,
        }
    }
}

impl Default for Window<f32> {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

/// مشخصات پنجره با مقادیر اعشاری (f32)
pub type WindowSpec = Window<f32>;

impl From<(&iced_core::Point, &iced_core::Size)> for WindowSpec {
    fn from((point, size): (&iced_core::Point, &iced_core::Size)) -> Self {
        Self {
            width: size.width,
            height: size.height,
            pos_x: point.x,
            pos_y: point.y,
        }
    }
}
