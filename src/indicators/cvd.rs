// CVD (Cumulative Volume Delta) Indicator
// Tracks the cumulative difference between buy and sell volume

use super::{Indicator, IndicatorConfig, Setting, UiContext, SessionMode};
use exchange::{Trade, Kline};
use iced::widget::canvas::{Path, Stroke};
use iced::{Color, Point, Size};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Line,
    Histogram,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalculationMode {
    Trades,
    Ohlc,
}

impl CalculationMode {
    fn from_str(s: &str) -> Self {
        match s {
            "OHLC" => CalculationMode::Ohlc,
            _ => CalculationMode::Trades,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            CalculationMode::Trades => "Trades",
            CalculationMode::Ohlc => "OHLC",
        }
    }
}

impl DisplayMode {
    fn from_str(s: &str) -> Self {
        match s {
            "Line" => DisplayMode::Line,
            "Histogram" => DisplayMode::Histogram,
            _ => DisplayMode::Histogram,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            DisplayMode::Line => "Line",
            DisplayMode::Histogram => "Histogram",
        }
    }
}

pub struct Cvd {
    name: String,
    settings: Vec<Setting>,
    
    // Core state
    cumulative_delta: f64,
    delta_points: BTreeMap<u64, f64>,
    
    // Visualization
    up_color: Color,
    down_color: Color,
    use_gradient: bool,
    display_mode: DisplayMode,
    calculation_mode: CalculationMode,
    show_divergence: bool,
    divergence_threshold: f64,
    
    // Non-double-counting for live updates
    last_candle_time: Option<u64>,
    last_candle_delta: f64,
    last_candle_ohlc: (f32, f32, f32, f32), // O, H, L, C
    prev_candle_ohlc: (f32, f32, f32, f32),
    
    // Divergence detection
    price_points: BTreeMap<u64, f64>,
    
    // Session management
    session_mode: SessionMode,
    last_session_ts: u64,
    last_interval: u64,
}

impl Cvd {
    fn is_new_session(&self, timestamp: u64) -> bool {
        self.session_mode.is_new_session(timestamp, self.last_session_ts)
    }

    fn push_point(&mut self, timestamp: u64, value: f64) {
        if !value.is_finite() { return; }
        self.delta_points.insert(timestamp, value);
        
        if self.delta_points.len() > 10000 {
            let keys: Vec<_> = self.delta_points.keys().take(1000).cloned().collect();
            for k in keys { self.delta_points.remove(&k); }
        }
    }


    fn y_bounds_in_range(&self, start: u64, end: u64) -> Option<(f64, f64)> {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut found = false;

        for (&_t, &v) in self.delta_points.range(start..=end) {
            if v < min { min = v; }
            if v > max { max = v; }
            found = true;
        }

        if !found { return None; }

        if self.display_mode == DisplayMode::Histogram {
            min = min.min(0.0);
            max = max.max(0.0);
        }

        let range = max - min;
        let padding = if range > 0.0 { range * 0.15 } else { 1.0 };
        
        let y_min = min - padding;
        let y_max = max + padding;

        Some((y_min, y_max))
    }

    fn calculate_ohlc_delta(&self, open: f32, high: f32, low: f32, close: f32, prev_open: f32, prev_close: f32, volume: f64) -> f64 {
        // Pine Script Logic Translation
        let bull_power = if close < open {
            if prev_close < prev_open {
                (high - prev_close).max(close - low)
            } else {
                (high - prev_open).max(close - low)
            }
        } else {
            if prev_close > prev_open {
                high - low
            } else {
                if high - close < close - low {
                    high - low
                } else {
                    if high - close > close - low {
                        high - open
                    } else {
                        (high - open).max(close - low)
                    }
                }
            }
        };

        let bear_power = if close < open {
            if prev_close > prev_open {
                (prev_close - open).max(high - low)
            } else {
                high - low
            }
        } else {
            if prev_close > prev_open {
                (prev_close - low).max(high - close)
            } else {
                if high - close < close - low {
                    open - low
                } else {
                    if high - close > close - low {
                        (open - low).max(high - close)
                    } else {
                        (prev_open - low).max(high - close)
                    }
                }
            }
        };

        let total_power = (bull_power + bear_power) as f64;
        if total_power > 0.0 {
            let bull_vol = (bull_power as f64 / total_power) * volume;
            let bear_vol = (bear_power as f64 / total_power) * volume;
            bull_vol - bear_vol
        } else {
            0.0
        }
    }

    fn find_pivots(&self, points: &BTreeMap<u64, f64>, window: usize) -> Vec<(u64, f64, bool)> {
        let values: Vec<(&u64, &f64)> = points.iter().collect();
        let mut pivots = Vec::new();

        if values.len() < window * 2 + 1 {
            return pivots;
        }

        for i in window..values.len() - window {
            let current = *values[i].1;
            let mut is_high = true;
            let mut is_low = true;

            for j in (i - window)..=(i + window) {
                if i == j { continue; }
                if *values[j].1 >= current { is_high = false; }
                if *values[j].1 <= current { is_low = false; }
            }

            if is_high { pivots.push((*values[i].0, current, true)); } // true for high pivot
            if is_low { pivots.push((*values[i].0, current, false)); } // false for low pivot
        }
        pivots
    }
}

impl Indicator for Cvd {
    fn new(config: IndicatorConfig) -> Self {
        let mut up_color = Color::from_rgb(0.1, 0.8, 0.4);
        let mut down_color = Color::from_rgb(0.9, 0.2, 0.3);
        let mut display_mode = DisplayMode::Line;
        let mut calculation_mode = CalculationMode::Trades;
        let mut use_gradient = true;
        let mut session_mode = SessionMode::Daily;
        let mut show_divergence = false;
        let mut divergence_threshold = 0.5;

        for setting in &config.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Display Mode" => {
                    display_mode = DisplayMode::from_str(value);
                }
                Setting::Enum { name, value, .. } if name == "Calculation Mode" => {
                    calculation_mode = CalculationMode::from_str(value);
                }
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    session_mode = SessionMode::from_str(value);
                }
                Setting::Color { name, value, .. } if name == "Up Color" => {
                    up_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Down Color" => {
                    down_color = *value;
                }
                Setting::Bool { name, value, .. } if name == "Use Gradient" => {
                    use_gradient = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Divergence" => {
                    show_divergence = *value;
                }
                Setting::Float { name, value, .. } if name == "Divergence Threshold" => {
                    divergence_threshold = *value;
                }
                _ => {}
            }
        }

        let settings = vec![
            Setting::Enum {
                name: "Display Mode".to_string(),
                value: display_mode.as_str().to_string(),
                options: vec!["Line".to_string(), "Histogram".to_string()],
                description: Some("Rendering style".to_string()),
            },
            Setting::Enum {
                name: "Session Mode".to_string(),
                value: session_mode.as_str().to_string(),
                options: vec!["Total".to_string(), "Session".to_string(), "Daily".to_string(), "Weekly".to_string(), "Monthly".to_string(), "Yearly".to_string()],
                description: Some("Reset cumulation period".to_string()),
            },
            Setting::Enum {
                name: "Calculation Mode".to_string(),
                value: calculation_mode.as_str().to_string(),
                options: vec!["Trades".to_string(), "OHLC".to_string()],
                description: Some("Source of delta calculation".to_string()),
            },
            Setting::Bool {
                name: "Use Gradient".to_string(),
                value: use_gradient,
                description: Some("Color by slope instead of value".to_string()),
            },
            Setting::Bool {
                name: "Show Divergence".to_string(),
                value: show_divergence,
                description: Some("Show price vs CVD divergence markers".to_string()),
            },
            Setting::Float {
                name: "Divergence Threshold".to_string(),
                value: divergence_threshold,
                min: 0.1,
                max: 5.0,
                step: 0.1,
                description: Some("Sensitivity of divergence detection".to_string()),
            },
            Setting::Color {
                name: "Up Color".to_string(),
                value: up_color,
                description: None,
            },
            Setting::Color {
                name: "Down Color".to_string(),
                value: down_color,
                description: None,
            },
        ];

        Self {
            name: config.name,
            settings,
            cumulative_delta: 0.0,
            delta_points: BTreeMap::new(),
            up_color,
            down_color,
            use_gradient,
            display_mode,
            calculation_mode,
            show_divergence,
            divergence_threshold,
            last_candle_time: None,
            last_candle_delta: 0.0,
            last_candle_ohlc: (0.0, 0.0, 0.0, 0.0),
            prev_candle_ohlc: (0.0, 0.0, 0.0, 0.0),
            price_points: BTreeMap::new(),
            session_mode,
            last_session_ts: 0,
            last_interval: 0,
        }
    }

    fn update_kline(&mut self, kline: &Kline) {
        if self.session_mode.is_new_session(kline.time, self.last_session_ts) {
            self.reset();
            self.last_session_ts = kline.time;
        }

        let o = kline.open.to_f32();
        let h = kline.high.to_f32();
        let l = kline.low.to_f32();
        let c = kline.close.to_f32();
        let vol = (kline.volume.0 + kline.volume.1) as f64;

        let delta = match self.calculation_mode {
            CalculationMode::Trades => kline.volume.0 as f64 - kline.volume.1 as f64,
            CalculationMode::Ohlc => self.calculate_ohlc_delta(o, h, l, c, self.prev_candle_ohlc.0, self.prev_candle_ohlc.3, vol),
        };
        
        // Handle partial kline updates for the same timestamp
        if let Some(last_time) = self.last_candle_time {
            if last_time == kline.time {
                self.cumulative_delta -= self.last_candle_delta;
            } else {
                if kline.time > last_time {
                    self.last_interval = kline.time - last_time;
                }
                self.prev_candle_ohlc = self.last_candle_ohlc;
                // Moving to a new candle
                self.last_candle_time = Some(kline.time);
            }
        } else {
            self.last_candle_time = Some(kline.time);
        }

        self.cumulative_delta += delta;
        self.last_candle_delta = delta;
        self.last_candle_ohlc = (o, h, l, c);
        
        self.delta_points.insert(kline.time, self.cumulative_delta);
        self.price_points.insert(kline.time, kline.close.to_f32() as f64);
    }

    fn update_tick(&mut self, tick: &Trade) {
        if self.is_new_session(tick.time) {
            self.reset();
            self.last_session_ts = tick.time;
        }

        if self.calculation_mode == CalculationMode::Ohlc {
            return; // OHLC mode only updates on klines
        }

        let delta = if !tick.is_sell { tick.qty as f64 } else { -(tick.qty as f64) };
        if delta.is_finite() {
            self.cumulative_delta += delta;

            let candle_start = if self.last_interval > 0 {
                (tick.time / self.last_interval) * self.last_interval
            } else {
                tick.time
            };

            if let Some(last_time) = self.last_candle_time {
                if candle_start == last_time {
                    self.last_candle_delta += delta;
                } else if candle_start > last_time {
                    self.last_candle_time = Some(candle_start);
                    self.last_candle_delta = delta;
                }
            } else {
                self.last_candle_time = Some(candle_start);
                self.last_candle_delta = delta;
            }

            self.push_point(tick.time, self.cumulative_delta);
        }
    }

    fn render(&self, ctx: &mut UiContext) {
        if self.delta_points.is_empty() {
            return;
        }

        let (earliest, latest) = ctx.viewport_range;
        let (y_min, y_max) = self.y_bounds_in_range(earliest, latest).unwrap_or((-1.0, 1.0));
        let value_range = (y_max - y_min).max(1e-12);
        let time_range = (latest as f64 - earliest as f64).max(1.0);

        let bounds = ctx.bounds;
        let scaling = ctx.scaling;

        let get_x = |ts: u64| -> f32 {
            let normalized = (ts as f64 - earliest as f64) / time_range;
            bounds.x + (normalized as f32 * bounds.width)
        };

        let get_y = |val: f64| -> f32 {
            let normalized = (val - y_min) / value_range;
            bounds.y + ((1.0 - normalized as f32) * bounds.height)
        };

        let points: Vec<(u64, f64)> = self.delta_points.range(earliest..=latest)
            .map(|(&t, &v)| (t, v))
            .collect();

        if points.is_empty() {
            return;
        }

        let cell_width = bounds.width / points.len().max(1) as f32;

        if self.display_mode == DisplayMode::Line {
            let path = Path::new(|builder| {
                let mut first = true;
                for &(t, v) in &points {
                    let x = get_x(t);
                    let y = get_y(v);
                    if !x.is_finite() || !y.is_finite() { continue; }
                    let p = Point::new(x, y);
                    if first {
                        builder.move_to(p);
                        first = false;
                    } else {
                        builder.line_to(p);
                    }
                }
            });

            ctx.frame.stroke(&path, Stroke::default()
                .with_color(if self.cumulative_delta >= 0.0 { self.up_color } else { self.down_color })
                .with_width(2.0 / scaling));
        } else {
            // Histogram
            let zero_y = get_y(0.0).clamp(bounds.y, bounds.y + bounds.height);
            for i in 0..points.len() {
                let (t, v) = points[i];
                let prev_v = if i > 0 { points[i-1].1 } else { 0.0 };
                
                let x = get_x(t);
                let y = get_y(v);
                
                let color = if self.use_gradient {
                    if v >= prev_v { self.up_color } else { self.down_color }
                } else {
                    if v >= 0.0 { self.up_color } else { self.down_color }
                };

                let top = y.min(zero_y);
                let height = (y - zero_y).abs().max(1.0 / scaling);

                if top.is_finite() && height.is_finite() && x.is_finite() {
                    ctx.frame.fill_rectangle(
                        Point::new(x - cell_width * 0.4, top),
                        Size::new(cell_width * 0.8, height),
                        color
                    );
                }
            }
        }

        // Divergence Rendering
        if self.show_divergence && points.len() > 10 {
            let price_pivots = self.find_pivots(&self.price_points, 5); 
            
            for i in 1..price_pivots.len() {
                let (t2, p_v2, is_high2) = price_pivots[i];
                let (t1, p_v1, is_high1) = price_pivots[i-1];
                
                if is_high2 != is_high1 { continue; }
                if t2 < earliest || t1 > latest { continue; }

                if let (Some(c_v2), Some(c_v1)) = (self.delta_points.get(&t2), self.delta_points.get(&t1)) {
                    let mut div_color = None;
                    
                    if is_high2 {
                        if p_v2 > p_v1 && *c_v2 < *c_v1 {
                            div_color = Some(Color::from_rgb(1.0, 0.2, 0.2));
                        }
                    } else {
                        if p_v2 < p_v1 && *c_v2 > *c_v1 {
                            div_color = Some(Color::from_rgb(0.2, 1.0, 0.2));
                        }
                    }

                    if let Some(color) = div_color {
                        let div_path = Path::new(|builder| {
                            builder.move_to(Point::new(get_x(t1), get_y(*c_v1)));
                            builder.line_to(Point::new(get_x(t2), get_y(*c_v2)));
                        });
                        ctx.frame.stroke(&div_path, Stroke::default().with_color(color).with_width(2.0 / scaling));
                        
                        ctx.frame.fill_rectangle(Point::new(get_x(t1)-2.0, get_y(*c_v1)-2.0), Size::new(4.0, 4.0), color);
                        ctx.frame.fill_rectangle(Point::new(get_x(t2)-2.0, get_y(*c_v2)-2.0), Size::new(4.0, 4.0), color);
                    }
                }
            }
        }
    }

    fn y_bounds(&self) -> Option<(f64, f64)> {
        if self.delta_points.is_empty() { return None; }
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for &v in self.delta_points.values() {
            if v < min { min = v; }
            if v > max { max = v; }
        }

        if self.display_mode == DisplayMode::Histogram {
            min = min.min(0.0);
            max = max.max(0.0);
        }

        let range = max - min;
        let padding = if range > 0.0 { range * 0.15 } else { 10.0 };
        Some((min - padding, max + padding))
    }

    fn name(&self) -> &str { &self.name }
    fn get_settings(&mut self) -> &mut Vec<Setting> { &mut self.settings }
    fn settings(&self) -> &[Setting] { &self.settings }
    fn is_overlay(&self) -> bool { false }

    fn reset(&mut self) {
        self.cumulative_delta = 0.0;
        self.last_candle_time = None;
        self.last_candle_delta = 0.0;
        // self.delta_points.clear();
        // self.price_points.clear();
    }

    fn element<'a>(&'a self, ctx: &super::ViewContext) -> iced::Element<'a, crate::chart::Message> {
        iced::widget::canvas::Canvas::new(super::DetachedIndicator {
            indicator: self,
            viewport_range: ctx.viewport_range,
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
    }

    fn sync_settings(&mut self) {
        let mut session_changed = false;
        for setting in &self.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Display Mode" => {
                    self.display_mode = DisplayMode::from_str(value);
                }
                Setting::Enum { name, value, .. } if name == "Calculation Mode" => {
                    self.calculation_mode = CalculationMode::from_str(value);
                }
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    let new_mode = SessionMode::from_str(value);
                    if self.session_mode != new_mode {
                        self.session_mode = new_mode;
                        session_changed = true;
                    }
                }
                Setting::Color { name, value, .. } if name == "Up Color" => {
                    self.up_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Down Color" => {
                    self.down_color = *value;
                }
                Setting::Bool { name, value, .. } if name == "Use Gradient" => {
                    self.use_gradient = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Divergence" => {
                    self.show_divergence = *value;
                }
                Setting::Float { name, value, .. } if name == "Divergence Threshold" => {
                    self.divergence_threshold = *value;
                }
                _ => {}
            }
        }
        if session_changed {
            self.reset();
        }
    }
}
