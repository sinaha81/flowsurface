use std::time::Duration;

use exchange::util::Price;
use serde::{Deserialize, Serialize};

use crate::util::ok_or_default;

const TRADE_RETENTION_MS: u64 = 120_000;

/// تنظیمات مربوط به لیست معاملات (Time and Sales)
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub trade_size_filter: f32, // فیلتر حداقل اندازه معامله برای نمایش
    #[serde(default = "default_buffer_filter")]
    pub trade_retention: Duration, // مدت زمان نگهداشت معاملات در لیست
    #[serde(deserialize_with = "ok_or_default", default)]
    pub stacked_bar: Option<StackedBar>, // تنظیمات نوار انباشته (Stacked Bar) در پایین لیست
}

impl Default for Config {
    fn default() -> Self {
        Config {
            trade_size_filter: 0.0,
            trade_retention: Duration::from_millis(TRADE_RETENTION_MS),
            stacked_bar: StackedBar::Compact(StackedBarRatio::default()).into(),
        }
    }
}

fn default_buffer_filter() -> Duration {
    Duration::from_millis(TRADE_RETENTION_MS)
}

/// ساختار داده‌ای برای نمایش یک معامله در لیست
#[derive(Debug, Clone)]
pub struct TradeDisplay {
    pub time_str: String, // رشته متنی زمان معامله
    pub price: Price,     // قیمت معامله
    pub qty: f32,         // مقدار معامله
    pub is_sell: bool,    // آیا معامله فروش است؟
}

/// ورودی یک معامله در حافظه به همراه برچسب زمانی خام
#[derive(Debug, Clone)]
pub struct TradeEntry {
    pub ts_ms: u64,           // برچسب زمانی به میلی‌ثانیه
    pub display: TradeDisplay, // اطلاعات نمایشی معامله
}

/// انواع نمایش نوار انباشته (Stacked Bar)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Copy)]
pub enum StackedBar {
    Compact(StackedBarRatio), // حالت فشرده
    Full(StackedBarRatio),    // حالت کامل
}

impl StackedBar {
    pub fn ratio(self) -> StackedBarRatio {
        match self {
            StackedBar::Compact(r) | StackedBar::Full(r) => r,
        }
    }

    pub fn with_ratio(self, r: StackedBarRatio) -> Self {
        match self {
            StackedBar::Compact(_) => StackedBar::Compact(r),
            StackedBar::Full(_) => StackedBar::Full(r),
        }
    }
}

/// مبنای محاسبه نسبت در نوار انباشته
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default, Copy)]
pub enum StackedBarRatio {
    Count,       // بر اساس تعداد معاملات
    #[default]
    Volume,      // بر اساس حجم معاملات
    AverageSize, // بر اساس میانگین اندازه معاملات
}

impl std::fmt::Display for StackedBarRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackedBarRatio::Count => write!(f, "Count"),
            StackedBarRatio::AverageSize => write!(f, "Average trade size"),
            StackedBarRatio::Volume => write!(f, "Volume"),
        }
    }
}

impl StackedBarRatio {
    pub const ALL: [StackedBarRatio; 3] = [
        StackedBarRatio::Count,
        StackedBarRatio::Volume,
        StackedBarRatio::AverageSize,
    ];
}

/// ساختار نگهدارنده تجمیع تاریخچه معاملات برای محاسبه نسبت‌ها
#[derive(Default)]
pub struct HistAgg {
    buy_count: u64,  // تعداد کل خریدها
    sell_count: u64, // تعداد کل فروش‌ها
    buy_sum: f64,    // مجموع حجم خریدها
    sell_sum: f64,   // مجموع حجم فروش‌ها
}

impl HistAgg {
    pub fn add(&mut self, trade: &TradeDisplay) {
        let qty = trade.qty as f64;

        if trade.is_sell {
            self.sell_count += 1;
            self.sell_sum += qty;
        } else {
            self.buy_count += 1;
            self.buy_sum += qty;
        }
    }

    pub fn remove(&mut self, trade: &TradeDisplay) {
        let qty = trade.qty as f64;

        if trade.is_sell {
            self.sell_count = self.sell_count.saturating_sub(1);
            self.sell_sum -= qty;
        } else {
            self.buy_count = self.buy_count.saturating_sub(1);
            self.buy_sum -= qty;
        }
    }

    /// دریافت مقادیر و نسبت خرید بر اساس نوع مبنای انتخاب شده
    pub fn values_for(&self, ratio_kind: StackedBarRatio) -> Option<(f64, f64, f32)> {
        match ratio_kind {
            StackedBarRatio::Count => {
                let buy = self.buy_count as f64;
                let sell = self.sell_count as f64;
                let total = buy + sell;

                if total <= 0.0 {
                    return None;
                }
                let buy_ratio = (buy / total) as f32;

                Some((buy, sell, buy_ratio))
            }
            StackedBarRatio::Volume => {
                let buy = self.buy_sum;
                let sell = self.sell_sum;
                let total = buy + sell;

                if total <= 0.0 {
                    return None;
                }
                let buy_ratio = (buy / total) as f32;

                Some((buy, sell, buy_ratio))
            }
            StackedBarRatio::AverageSize => {
                let buy_avg = if self.buy_count > 0 {
                    self.buy_sum / self.buy_count as f64
                } else {
                    0.0
                };
                let sell_avg = if self.sell_count > 0 {
                    self.sell_sum / self.sell_count as f64
                } else {
                    0.0
                };

                let denom = buy_avg + sell_avg;
                if denom <= 0.0 {
                    return None;
                }
                let buy_ratio = (buy_avg / denom) as f32;

                Some((buy_avg, sell_avg, buy_ratio))
            }
        }
    }
}
