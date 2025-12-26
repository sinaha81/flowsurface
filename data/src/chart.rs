// ماژول‌های مربوط به اجزای مختلف نمودار
pub mod comparison;
pub mod heatmap;
pub mod indicator;
pub mod kline;

use exchange::Timeframe;
use serde::{Deserialize, Serialize};

use super::aggr::{
    self,
    ticks::TickAggr,
    time::{DataPoint, TimeSeries},
};
pub use kline::KlineChartKind;

/// انواع داده‌های قابل نمایش در نمودار
pub enum PlotData<D: DataPoint> {
    TimeBased(TimeSeries<D>), // داده‌های مبتنی بر زمان
    TickBased(TickAggr),      // داده‌های مبتنی بر تعداد تیک
}

impl<D: DataPoint> PlotData<D> {
    /// محاسبه نقطه میانی آخرین قیمت برای نمایش در محور Y
    pub fn latest_y_midpoint(&self, calculate_target_y: impl Fn(exchange::Kline) -> f32) -> f32 {
        match self {
            PlotData::TimeBased(timeseries) => timeseries
                .latest_kline()
                .map_or(0.0, |kline| calculate_target_y(*kline)),
            PlotData::TickBased(tick_aggr) => tick_aggr
                .latest_dp()
                .map_or(0.0, |(dp, _)| calculate_target_y(dp.kline)),
        }
    }

    /// محاسبه محدوده قیمت قابل مشاهده در یک بازه مشخص
    pub fn visible_price_range(
        &self,
        start_interval: u64,
        end_interval: u64,
    ) -> Option<(f32, f32)> {
        match self {
            PlotData::TimeBased(timeseries) => {
                timeseries.min_max_price_in_range(start_interval, end_interval)
            }
            PlotData::TickBased(tick_aggr) => {
                tick_aggr.min_max_price_in_range(start_interval as usize, end_interval as usize)
            }
        }
    }
}

/// تنظیمات نمایشی نمودار
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ViewConfig {
    pub splits: Vec<f32>,           // تقسیم‌بندی‌های نمودار
    pub autoscale: Option<Autoscale>, // تنظیمات مقیاس‌دهی خودکار
}

/// حالت‌های مختلف مقیاس‌دهی خودکار (Autoscale)
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq)]
pub enum Autoscale {
    #[default]
    CenterLatest, // متمرکز کردن آخرین قیمت
    FitToVisible, // برازش بر اساس داده‌های قابل مشاهده
}

/// تعیین می‌کند که داده‌های نمودار چگونه در محور افقی (X) تجمیع و نمایش داده شوند
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Basis {
    /// تجمیع مبتنی بر زمان که در آن هر نقطه داده نشان‌دهنده یک بازه زمانی ثابت است
    Time(exchange::Timeframe),

    /// تجمیع مبتنی بر معامله که در آن هر نقطه داده نشان‌دهنده تعداد مشخصی معامله (تیک) است
    Tick(aggr::TickCount),
}

impl Basis {
    pub fn is_time(&self) -> bool {
        matches!(self, Basis::Time(_))
    }

    pub fn default_heatmap_time(ticker_info: Option<exchange::TickerInfo>) -> Self {
        let fallback = Timeframe::MS500;

        let interval = ticker_info.map_or(fallback, |info| {
            let ex = info.exchange();
            Timeframe::HEATMAP
                .iter()
                .copied()
                .find(|tf| ex.supports_heatmap_timeframe(*tf))
                .unwrap_or(fallback)
        });

        interval.into()
    }
}

impl std::fmt::Display for Basis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Basis::Time(timeframe) => write!(f, "{timeframe}"),
            Basis::Tick(count) => write!(f, "{count}"),
        }
    }
}

impl From<exchange::Timeframe> for Basis {
    fn from(timeframe: exchange::Timeframe) -> Self {
        Self::Time(timeframe)
    }
}

/// انواع مطالعات یا اندیکاتورهای قابل اضافه شدن به نمودار
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Study {
    Heatmap(Vec<heatmap::HeatmapStudy>),   // نقشه حرارتی (Heatmap)
    Footprint(Vec<kline::FootprintStudy>), // نمودار فوت‌پرینت (Footprint)
}
