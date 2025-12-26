use exchange::adapter::PersistStreamKind;
use exchange::{TickMultiplier, TickerInfo, Timeframe};
use serde::{Deserialize, Serialize};

use crate::chart::{comparison, heatmap, kline};
use crate::panel::{ladder, timeandsales};
use crate::util::ok_or_default;

use crate::chart::{
    Basis, ViewConfig,
    heatmap::HeatmapStudy,
    indicator::{HeatmapIndicator, KlineIndicator},
    kline::KlineChartKind,
};

/// محور تقسیم‌بندی پنل‌ها
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Axis {
    Horizontal, // افقی
    Vertical,   // عمودی
}

/// انواع مختلف پنل‌ها در رابط کاربری
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Pane {
    /// پنل تقسیم شده به دو بخش
    Split {
        axis: Axis,      // محور تقسیم
        ratio: f32,      // نسبت تقسیم
        a: Box<Pane>,    // بخش اول
        b: Box<Pane>,    // بخش دوم
    },
    /// پنل شروع (Starter) برای انتخاب نوع محتوا
    Starter {
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>, // گروه پیوند (Link Group)
    },
    /// نمودار نقشه حرارتی (Heatmap)
    HeatmapChart {
        layout: ViewConfig,
        #[serde(deserialize_with = "ok_or_default", default)]
        studies: Vec<HeatmapStudy>,
        #[serde(deserialize_with = "ok_or_default", default)]
        stream_type: Vec<PersistStreamKind>,
        #[serde(deserialize_with = "ok_or_default")]
        settings: Settings,
        #[serde(deserialize_with = "ok_or_default", default)]
        indicators: Vec<HeatmapIndicator>,
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>,
    },
    /// نمودار کندل‌استیک یا فوت‌پرینت
    KlineChart {
        layout: ViewConfig,
        kind: KlineChartKind,
        #[serde(deserialize_with = "ok_or_default", default)]
        stream_type: Vec<PersistStreamKind>,
        #[serde(deserialize_with = "ok_or_default")]
        settings: Settings,
        #[serde(deserialize_with = "ok_or_default", default)]
        indicators: Vec<KlineIndicator>,
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>,
    },
    /// نمودار مقایسه‌ای
    ComparisonChart {
        stream_type: Vec<PersistStreamKind>,
        #[serde(deserialize_with = "ok_or_default")]
        settings: Settings,
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>,
    },
    /// لیست معاملات (Time and Sales)
    TimeAndSales {
        stream_type: Vec<PersistStreamKind>,
        settings: Settings,
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>,
    },
    /// نردبان قیمت (Ladder/DOM)
    Ladder {
        stream_type: Vec<PersistStreamKind>,
        settings: Settings,
        #[serde(deserialize_with = "ok_or_default", default)]
        link_group: Option<LinkGroup>,
    },
}

impl Default for Pane {
    fn default() -> Self {
        Pane::Starter { link_group: None }
    }
}

/// تنظیمات عمومی یک پنل
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Settings {
    pub tick_multiply: Option<exchange::TickMultiplier>, // ضریب گام قیمت
    pub visual_config: Option<VisualConfig>,             // تنظیمات بصری اختصاصی
    pub selected_basis: Option<Basis>,                   // مبنای انتخاب شده (زمان یا تیک)
}

/// گروه‌های پیوند برای همگام‌سازی نمادها بین پنل‌های مختلف
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum LinkGroup {
    A, B, C, D, E, F, G, H, I,
}

impl LinkGroup {
    pub const ALL: [LinkGroup; 9] = [
        LinkGroup::A,
        LinkGroup::B,
        LinkGroup::C,
        LinkGroup::D,
        LinkGroup::E,
        LinkGroup::F,
        LinkGroup::G,
        LinkGroup::H,
        LinkGroup::I,
    ];
}

impl std::fmt::Display for LinkGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            LinkGroup::A => "1",
            LinkGroup::B => "2",
            LinkGroup::C => "3",
            LinkGroup::D => "4",
            LinkGroup::E => "5",
            LinkGroup::F => "6",
            LinkGroup::G => "7",
            LinkGroup::H => "8",
            LinkGroup::I => "9",
        };
        write!(f, "{c}")
    }
}

/// Defines the specific configuration for different types of pane settings.
/// تنظیمات بصری اختصاصی برای هر نوع پنل
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum VisualConfig {
    Heatmap(heatmap::Config),           // تنظیمات نقشه حرارتی
    TimeAndSales(timeandsales::Config), // تنظیمات لیست معاملات
    Kline(kline::Config),               // تنظیمات کندل‌استیک
    Ladder(ladder::Config),             // تنظیمات نردبان قیمت
    Comparison(comparison::Config),     // تنظیمات نمودار مقایسه‌ای
}

impl VisualConfig {
    pub fn heatmap(&self) -> Option<heatmap::Config> {
        match self {
            Self::Heatmap(cfg) => Some(*cfg),
            _ => None,
        }
    }

    pub fn time_and_sales(&self) -> Option<timeandsales::Config> {
        match self {
            Self::TimeAndSales(cfg) => Some(*cfg),
            _ => None,
        }
    }

    pub fn kline(&self) -> Option<kline::Config> {
        match self {
            Self::Kline(cfg) => Some(*cfg),
            _ => None,
        }
    }

    pub fn ladder(&self) -> Option<ladder::Config> {
        match self {
            Self::Ladder(cfg) => Some(*cfg),
            _ => None,
        }
    }

    pub fn comparison(&self) -> Option<comparison::Config> {
        match self {
            Self::Comparison(cfg) => Some(cfg.clone()),
            _ => None,
        }
    }
}

/// انواع محتواهای قابل نمایش در پنل‌ها
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Starter,          // پنل شروع
    HeatmapChart,     // نمودار نقشه حرارتی
    FootprintChart,   // نمودار فوت‌پرینت
    CandlestickChart, // نمودار کندل‌استیک
    ComparisonChart,  // نمودار مقایسه‌ای
    TimeAndSales,     // لیست معاملات
    Ladder,           // نردبان قیمت
}

impl ContentKind {
    pub const ALL: [ContentKind; 7] = [
        ContentKind::Starter,
        ContentKind::HeatmapChart,
        ContentKind::FootprintChart,
        ContentKind::CandlestickChart,
        ContentKind::ComparisonChart,
        ContentKind::TimeAndSales,
        ContentKind::Ladder,
    ];
}

impl std::fmt::Display for ContentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ContentKind::Starter => "Starter Pane",
            ContentKind::HeatmapChart => "Heatmap Chart",
            ContentKind::FootprintChart => "Footprint Chart",
            ContentKind::CandlestickChart => "Candlestick Chart",
            ContentKind::ComparisonChart => "Comparison Chart",
            ContentKind::TimeAndSales => "Time&Sales",
            ContentKind::Ladder => "DOM/Ladder",
        };
        write!(f, "{s}")
    }
}

/// ساختار کمکی برای راه‌اندازی و تنظیم اولیه یک پنل
#[derive(Clone, Copy)]
pub struct PaneSetup {
    pub ticker_info: exchange::TickerInfo,            // اطلاعات نماد
    pub basis: Option<Basis>,                         // مبنای زمانی یا تیکی
    pub tick_multiplier: Option<TickMultiplier>,      // ضریب گام قیمت
    pub tick_size: f32,                               // اندازه گام قیمت نهایی
    pub depth_aggr: exchange::adapter::StreamTicksize, // تنظیمات تجمیع عمق بازار
    pub push_freq: exchange::PushFrequency,           // فرکانس ارسال داده‌ها
}

impl PaneSetup {
    pub fn new(
        content_kind: ContentKind,
        base_ticker: TickerInfo,
        prev_base_ticker: Option<TickerInfo>,
        current_basis: Option<Basis>,
        current_tick_multiplier: Option<TickMultiplier>,
    ) -> Self {
        let exchange = base_ticker.ticker.exchange;

        let is_client_aggr = exchange.is_depth_client_aggr();
        let prev_is_client_aggr = prev_base_ticker
            .map(|ti| ti.ticker.exchange.is_depth_client_aggr())
            .unwrap_or(is_client_aggr);

        let basis = match content_kind {
            ContentKind::HeatmapChart => {
                let current = current_basis.and_then(|b| match b {
                    Basis::Time(tf) if exchange.supports_heatmap_timeframe(tf) => Some(b),
                    _ => None,
                });
                Some(current.unwrap_or_else(|| Basis::default_heatmap_time(Some(base_ticker))))
            }
            ContentKind::Ladder => Some(
                current_basis.unwrap_or_else(|| Basis::default_heatmap_time(Some(base_ticker))),
            ),
            ContentKind::FootprintChart => {
                Some(current_basis.unwrap_or(Basis::Time(Timeframe::M5)))
            }
            ContentKind::CandlestickChart | ContentKind::ComparisonChart => {
                Some(current_basis.unwrap_or(Basis::Time(Timeframe::M15)))
            }
            ContentKind::Starter | ContentKind::TimeAndSales => None,
        };

        let tick_multiplier = match content_kind {
            ContentKind::HeatmapChart | ContentKind::Ladder => {
                let tm = if !is_client_aggr && prev_is_client_aggr {
                    TickMultiplier(10)
                } else if let Some(tm) = current_tick_multiplier {
                    tm
                } else if is_client_aggr {
                    TickMultiplier(5)
                } else {
                    TickMultiplier(10)
                };
                Some(tm)
            }
            ContentKind::FootprintChart => {
                Some(current_tick_multiplier.unwrap_or(TickMultiplier(50)))
            }
            ContentKind::CandlestickChart
            | ContentKind::ComparisonChart
            | ContentKind::TimeAndSales
            | ContentKind::Starter => current_tick_multiplier,
        };

        let tick_size = match tick_multiplier {
            Some(tm) => tm.multiply_with_min_tick_size(base_ticker),
            None => base_ticker.min_ticksize.into(),
        };

        let depth_aggr = exchange.stream_ticksize(tick_multiplier, TickMultiplier(50));

        let push_freq = match content_kind {
            ContentKind::HeatmapChart if exchange.is_custom_push_freq() => match basis {
                Some(Basis::Time(tf)) if exchange.supports_heatmap_timeframe(tf) => {
                    exchange::PushFrequency::Custom(tf)
                }
                _ => exchange::PushFrequency::ServerDefault,
            },
            _ => exchange::PushFrequency::ServerDefault,
        };

        Self {
            ticker_info: base_ticker,
            basis,
            tick_multiplier,
            tick_size,
            depth_aggr,
            push_freq,
        }
    }
}
