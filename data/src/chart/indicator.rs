use std::fmt::{self, Debug, Display};

use enum_map::Enum;
use exchange::adapter::MarketKind;
use serde::{Deserialize, Serialize};

/// تریت پایه برای تمامی اندیکاتورها
pub trait Indicator: PartialEq + Display + 'static {
    /// دریافت اندیکاتورهای موجود برای یک نوع بازار خاص
    fn for_market(market: MarketKind) -> &'static [Self]
    where
        Self: Sized;
}

/// اندیکاتورهای مربوط به نمودار کندل‌استیک
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Eq, Enum)]
pub enum KlineIndicator {
    Volume,       // حجم معاملات
    OpenInterest, // بهره باز (فقط برای قراردادهای آتی)
}

impl Indicator for KlineIndicator {
    fn for_market(market: MarketKind) -> &'static [Self] {
        match market {
            MarketKind::Spot => &Self::FOR_SPOT,
            MarketKind::LinearPerps | MarketKind::InversePerps => &Self::FOR_PERPS,
        }
    }
}

impl KlineIndicator {
    // Indicator togglers on UI menus depend on these arrays.
    // Every variant needs to be in either SPOT, PERPS or both.
    /// اندیکاتورهای قابل استفاده در بازار اسپات (Spot)
    const FOR_SPOT: [KlineIndicator; 1] = [KlineIndicator::Volume];
    /// اندیکاتورهای قابل استفاده در بازار قراردادهای دائمی (Perpetual)
    const FOR_PERPS: [KlineIndicator; 2] = [KlineIndicator::Volume, KlineIndicator::OpenInterest];
}

impl Display for KlineIndicator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KlineIndicator::Volume => write!(f, "Volume"),
            KlineIndicator::OpenInterest => write!(f, "Open Interest"),
        }
    }
}

/// اندیکاتورهای مربوط به نقشه حرارتی (Heatmap)
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Eq, Enum)]
pub enum HeatmapIndicator {
    Volume, // حجم معاملات
}

impl Indicator for HeatmapIndicator {
    fn for_market(market: MarketKind) -> &'static [Self] {
        match market {
            MarketKind::Spot => &Self::FOR_SPOT,
            MarketKind::LinearPerps | MarketKind::InversePerps => &Self::FOR_PERPS,
        }
    }
}

impl HeatmapIndicator {
    // Indicator togglers on UI menus depend on these arrays.
    // Every variant needs to be in either SPOT, PERPS or both.
    /// اندیکاتورهای قابل استفاده در بازار اسپات برای نقشه حرارتی
    const FOR_SPOT: [HeatmapIndicator; 1] = [HeatmapIndicator::Volume];
    /// اندیکاتورهای قابل استفاده در بازار قراردادهای دائمی برای نقشه حرارتی
    const FOR_PERPS: [HeatmapIndicator; 1] = [HeatmapIndicator::Volume];
}

impl Display for HeatmapIndicator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HeatmapIndicator::Volume => write!(f, "Volume"),
        }
    }
}

/// ساختار موقت برای نمایش هر نوع اندیکاتور در رابط کاربری
#[derive(Debug, Clone, Copy)]
pub enum UiIndicator {
    Heatmap(HeatmapIndicator),
    Kline(KlineIndicator),
}

impl From<KlineIndicator> for UiIndicator {
    fn from(k: KlineIndicator) -> Self {
        UiIndicator::Kline(k)
    }
}

impl From<HeatmapIndicator> for UiIndicator {
    fn from(h: HeatmapIndicator) -> Self {
        UiIndicator::Heatmap(h)
    }
}
