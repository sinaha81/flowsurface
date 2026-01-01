// Modular Indicator System
// Provides a trait-based hot-plug architecture for technical indicators

pub mod vwap;
pub mod cvd;


use exchange::{Trade, Kline};
use iced::{Rectangle, Color, Vector};
use serde::{Deserialize, Serialize};

/// Represents a single indicator setting that can be configured via UI
use chrono::Datelike;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionMode {
    Session,
    Daily,
    Weekly,
    Monthly,
    ThreeMonth,
    SixMonth,
    Yearly,
    Chart,     // Never reset (Total)
}

impl SessionMode {
    pub fn as_str(&self) -> &str {
        match self {
            SessionMode::Session => "Session",
            SessionMode::Daily => "Daily",
            SessionMode::Weekly => "Weekly",
            SessionMode::Monthly => "Monthly",
            SessionMode::ThreeMonth => "3 Month",
            SessionMode::SixMonth => "6 Month",
            SessionMode::Yearly => "Yearly",
            SessionMode::Chart => "Total",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "Daily" => SessionMode::Daily,
            "Weekly" => SessionMode::Weekly,
            "Monthly" => SessionMode::Monthly,
            "3 Month" => SessionMode::ThreeMonth,
            "6 Month" => SessionMode::SixMonth,
            "Yearly" => SessionMode::Yearly,
            "Total" => SessionMode::Chart,
            _ => SessionMode::Session,
        }
    }

    pub fn is_new_session(&self, current_ts: u64, last_ts: u64) -> bool {
        if last_ts == 0 {
            return true;
        }

        let current_time = chrono::DateTime::from_timestamp_millis(current_ts as i64).unwrap_or_else(|| chrono::Utc::now());
        let last_time = chrono::DateTime::from_timestamp_millis(last_ts as i64).unwrap_or_else(|| chrono::Utc::now());

        match self {
            SessionMode::Session | SessionMode::Daily => current_time.date_naive() != last_time.date_naive(),
            SessionMode::Weekly => current_time.iso_week() != last_time.iso_week(),
            SessionMode::Monthly => current_time.year() != last_time.year() || current_time.month() != last_time.month(),
            SessionMode::ThreeMonth => {
                 let q_curr = (current_time.month() - 1) / 3;
                 let q_last = (last_time.month() - 1) / 3;
                 current_time.year() != last_time.year() || q_curr != q_last
            },
            SessionMode::SixMonth => {
                 let h_curr = (current_time.month() - 1) / 6;
                 let h_last = (last_time.month() - 1) / 6;
                 current_time.year() != last_time.year() || h_curr != h_last
            },
            SessionMode::Yearly => current_time.year() != last_time.year(),
            SessionMode::Chart => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Setting {
    /// Boolean toggle (checkbox)
    Bool {
        name: String,
        value: bool,
        description: Option<String>,
    },
    /// Integer value (slider)
    Int {
        name: String,
        value: i32,
        min: i32,
        max: i32,
        step: i32,
        description: Option<String>,
    },
    /// Float value (slider)
    Float {
        name: String,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        description: Option<String>,
    },
    /// Enumeration (dropdown)
    Enum {
        name: String,
        value: String,
        options: Vec<String>,
        description: Option<String>,
    },
    /// Color picker
    Color {
        name: String,
        value: Color,
        description: Option<String>,
    },
}

/// Configuration for indicator initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndicatorConfig {
    pub name: String,
    pub settings: Vec<Setting>,
    pub enabled: bool,
}

/// Simplified rendering context passed to indicators
pub struct UiContext<'a> {
    pub frame: &'a mut iced::widget::canvas::Frame,
    pub theme: &'a iced::Theme,
    pub bounds: Rectangle,
    pub scaling: f32,
    pub translation: Vector,
    pub viewport_range: (u64, u64),
    pub price_range: (f64, f64),
    
    // Precise mapping parameters for overlays
    pub base_price: f64,
    pub tick_size: f64,
    pub cell_height: f32,
    pub latest_x: u64,
    pub cell_width: f32,
    pub interval: u64,
}

/// Core trait that all indicators must implement
pub trait Indicator: Send + Sync {
    /// Create a new indicator with the given configuration
    fn new(config: IndicatorConfig) -> Self
    where
        Self: Sized;

    /// Update indicator with new candle/kline data
    fn update_kline(&mut self, kline: &Kline);

    /// Update indicator with new tick/trade data
    fn update_tick(&mut self, tick: &Trade);

    /// Render the indicator on the chart
    fn render(&self, ctx: &mut UiContext);

    /// Get the display name of the indicator
    fn name(&self) -> &str;

    /// Get indicator description
    fn description(&self) -> &str {
        ""
    }

    /// Get mutable access to settings for UI generation
    fn get_settings(&mut self) -> &mut Vec<Setting>;

    /// Get settings for UI generation (read-only)
    fn settings(&self) -> &[Setting] {
        &[]
    }

    /// Check if indicator should render as overlay (true) or sub-chart (false)
    fn is_overlay(&self) -> bool {
        true
    }

    /// Get Y-axis min/max bounds for sub-chart indicators
    fn y_bounds(&self) -> Option<(f64, f64)> {
        None
    }

    /// Reset indicator state (e.g., when changing timeframe)
    fn reset(&mut self);

    /// Update indicator with new tick size
    fn set_tick_size(&mut self, _size: f64) {}

    /// Create an Iced Element for detached indicators (sub-charts)
    fn element<'a>(&'a self, ctx: &ViewContext) -> iced::Element<'a, crate::chart::Message>;

    /// Sync internal fields with current settings vector
    fn sync_settings(&mut self) {}
}

/// Registry for all available indicators
pub struct IndicatorRegistry {
    factories: std::sync::RwLock<std::collections::HashMap<String, IndicatorFactory>>,
}

/// Factory function type for creating indicators
type IndicatorFactory = Box<dyn Fn(IndicatorConfig) -> Box<dyn Indicator> + Send + Sync>;

impl IndicatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a new indicator type
    pub fn register<T: Indicator + 'static>(
        &self,
        name: &str,
        factory: impl Fn(IndicatorConfig) -> T + Send + Sync + 'static,
    ) {
        let boxed_factory = Box::new(move |config: IndicatorConfig| {
            Box::new(factory(config)) as Box<dyn Indicator>
        });
        self.factories.write().unwrap().insert(name.to_string(), boxed_factory);
    }

    /// Get all registered indicator names
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<String> {
        let factories = self.factories.read().unwrap();
        let mut names: Vec<String> = factories.keys().cloned().collect();
        names.sort();
        names
    }

    /// Create an indicator instance by name
    pub fn create(&self, name: &str, config: IndicatorConfig) -> Option<Box<dyn Indicator>> {
        let factories = self.factories.read().unwrap();
        factories.get(name).map(|factory| factory(config))
    }
}

/// Helper for rendering modular indicators on a standalone canvas (sub-charts)
pub struct DetachedIndicator<'a> {
    pub indicator: &'a dyn Indicator,
    pub viewport_range: (u64, u64),
}

/// Context for indicator view generation (no frame)
pub struct ViewContext {
    pub bounds: Rectangle,
    pub viewport_range: (u64, u64),
}

impl<'a> iced::widget::canvas::Program<crate::chart::Message> for DetachedIndicator<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());

        let mut ctx = UiContext {
            frame: &mut frame,
            theme,
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height,
            },
            scaling: 1.0,
            translation: Vector::default(),
            viewport_range: self.viewport_range,
            price_range: self.indicator.y_bounds().filter(|(min, max)| min.is_finite() && max.is_finite()).unwrap_or((-1.0, 1.0)),
            base_price: 0.0,
            tick_size: 1.0,
            cell_height: 1.0,
            latest_x: 0,
            cell_width: 1.0,
            interval: 1,
        };

        self.indicator.render(&mut ctx);

        vec![frame.into_geometry()]
    }
}

impl Default for IndicatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry instance
pub static REGISTRY: std::sync::LazyLock<IndicatorRegistry> = std::sync::LazyLock::new(|| {
    let registry = IndicatorRegistry::new();

    // Register built-in indicators
    registry.register("VWAP", |config| vwap::Vwap::new(config));
    registry.register("CVD", |config| cvd::Cvd::new(config));


    registry
});

/// Macro for easy indicator registration
#[macro_export]
macro_rules! register_indicator {
    ($name:expr, $indicator_type:ty, $registry:expr) => {
        $registry.register($name, |config| <$indicator_type>::new(config));
    };
}
