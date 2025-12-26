use crate::util::ok_or_default;
use exchange::SerTicker;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// آستانه (Threshold) برای پخش صدا
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Threshold {
    Count(usize), // بر اساس تعداد معاملات
    Qty(f32),     // بر اساس حجم معاملات
}

impl std::fmt::Display for Threshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Threshold::Count(count) => write!(f, "Count based: {}", count),
            Threshold::Qty(qty) => write!(f, "Qty based: {:.2}", qty),
        }
    }
}

/// تنظیمات پخش صدا برای یک استریم خاص
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct StreamCfg {
    pub enabled: bool,        // آیا پخش صدا فعال است؟
    pub threshold: Threshold, // آستانه پخش صدا
}

impl Default for StreamCfg {
    fn default() -> Self {
        StreamCfg {
            enabled: true,
            threshold: Threshold::Count(10),
        }
    }
}

/// ساختار کلی تنظیمات صوتی برنامه
#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AudioStream {
    // تنظیمات مربوط به هر نماد معاملاتی
    #[serde(deserialize_with = "ok_or_default")]
    pub streams: FxHashMap<SerTicker, StreamCfg>,
    // میزان صدای کلی برنامه
    #[serde(deserialize_with = "ok_or_default")]
    pub volume: Option<f32>,
}
