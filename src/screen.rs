pub mod dashboard;

/// خطاهای مربوط به داشبورد
#[derive(thiserror::Error, Debug, Clone)]
pub enum DashboardError {
    #[error("Fetch error: {0}")]
    Fetch(String), // خطای دریافت داده
    #[error("Pane set error: {0}")]
    PaneSet(String), // خطای تنظیم پنل
    #[error("Unknown error: {0}")]
    Unknown(String), // خطای ناشناخته
}

/// ساختار دیالوگ تایید (Confirmation)
#[derive(Debug, Clone)]
pub struct ConfirmDialog<M> {
    pub message: String,                   // متن پیام دیالوگ
    pub on_confirm: Box<M>,                // پیامی که در صورت تایید ارسال می‌شود
    pub on_confirm_btn_text: Option<String>, // متن سفارشی برای دکمه تایید
}

impl<M> ConfirmDialog<M> {
    /// ایجاد یک دیالوگ تایید جدید
    pub fn new(message: String, on_confirm: Box<M>) -> Self {
        Self {
            message,
            on_confirm,
            on_confirm_btn_text: None,
        }
    }

    /// تنظیم متن دکمه تایید
    pub fn with_confirm_btn_text(mut self, on_confirm_btn_text: String) -> Self {
        self.on_confirm_btn_text = Some(on_confirm_btn_text);
        self
    }
}
