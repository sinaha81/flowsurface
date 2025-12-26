use serde::{Deserialize, Serialize};

use super::{WindowSpec, pane::Pane};
use crate::util::ok_or_default;

/// ساختار نگهدارنده اطلاعات یک داشبورد شامل پنل اصلی و پنجره‌های پاپ‌اوت
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Dashboard {
    #[serde(deserialize_with = "ok_or_default", default)]
    pub pane: Pane, // پنل اصلی داشبورد
    #[serde(deserialize_with = "ok_or_default", default)]
    pub popout: Vec<(Pane, WindowSpec)>, // لیست پنجره‌های جدا شده (Popout)
}
