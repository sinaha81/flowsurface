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
    
    // Non-double-counting for live updates
    last_candle_time: Option<u64>,
    last_candle_delta: f64,
    last_candle_ohlc: (f32, f32, f32, f32), // O, H, L, C
    prev_candle_ohlc: (f32, f32, f32, f32),
    
    // Divergence detection
    price_points: BTreeMap<u64, f64>,
    cvd_ma_points: BTreeMap<u64, f64>,
    ma_length: usize,
    plot_ma: bool,
    lb_r: usize,
    lb_l: usize,
    range_upper: usize,
    range_lower: usize,
    plot_bull: bool,
    plot_hidden_bull: bool,
    plot_bear: bool,
    plot_hidden_bear: bool,
    
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

    fn calculate_ohlc_delta(&self, open: f32, high: f32, low: f32, close: f32, _prev_open: f32, prev_close: f32, volume: f64) -> f64 {
        let diff_hl = high - low;
        let diff_hc_prev = high - prev_close;
        let diff_cl = close - low;
        let diff_ho = high - open;
        let diff_oc_prev = open - prev_close;
        let diff_hc = high - close;
        let diff_ol = open - low;
        let diff_c_prev_o = prev_close - open;
        let diff_c_prev_l = prev_close - low;

        let iff_1 = if prev_close < open { diff_hc_prev.max(diff_cl) } else { diff_ho.max(diff_cl) };
        let iff_2 = if prev_close > open { diff_hl } else { diff_oc_prev.max(diff_hl) };
        let iff_3 = if prev_close < open { diff_hc_prev.max(diff_cl) } else { diff_ho };
        let iff_4 = if prev_close > open { diff_hl } else { diff_oc_prev.max(diff_hl) };
        let iff_5 = if prev_close < open { diff_oc_prev.max(diff_hl) } else { diff_hl };
        let iff_6 = if prev_close > open { diff_ho.max(diff_cl) } else { iff_5 };
        let iff_7 = if diff_hc < diff_cl { iff_4 } else { iff_6 };
        let iff_8 = if diff_hc > diff_cl { iff_3 } else { iff_7 };
        let iff_9 = if close > open { iff_2 } else { iff_8 };
        let bull_power = if close < open { iff_1 } else { iff_9 };

        let iff_10 = if prev_close > open { diff_c_prev_o.max(diff_hl) } else { diff_hl };
        let iff_11 = if prev_close > open { diff_c_prev_l.max(diff_hc) } else { diff_ol.max(diff_hc) };
        let iff_12 = if prev_close > open { diff_c_prev_o.max(diff_hl) } else { diff_hl };
        let iff_13 = if prev_close > open { diff_c_prev_l.max(diff_hc) } else { diff_ol };
        let iff_14 = if prev_close < open { diff_ol.max(diff_hc) } else { diff_hl };
        let iff_15 = if prev_close > open { diff_c_prev_o.max(diff_hl) } else { iff_14 };
        let iff_16 = if diff_hc < diff_cl { iff_13 } else { iff_15 };
        let iff_17 = if diff_hc > diff_cl { iff_12 } else { iff_16 };
        let iff_18 = if close > open { iff_11 } else { iff_17 };
        let bear_power = if close < open { iff_10 } else { iff_18 };

        let total_power = (bull_power + bear_power) as f64;
        if total_power > 0.0 {
            let bull_vol = (bull_power as f64 / total_power) * volume;
            let bear_vol = (bear_power as f64 / total_power) * volume;
            bull_vol - bear_vol
        } else {
            0.0
        }
    }

    fn find_pivot_low(&self, points: &BTreeMap<u64, f64>, lb_l: usize, lb_r: usize) -> Vec<(u64, f64)> {
        let values: Vec<(&u64, &f64)> = points.iter().collect();
        let mut pivots = Vec::new();
        if values.len() < lb_l + lb_r + 1 { return pivots; }

        for i in lb_l..values.len() - lb_r {
            let current = *values[i].1;
            let mut is_pivot = true;
            // Check left
            for j in (i - lb_l)..i {
                if *values[j].1 <= current { is_pivot = false; break; }
            }
            if !is_pivot { continue; }
            // Check right
            for j in (i + 1)..=(i + lb_r) {
                if *values[j].1 <= current { is_pivot = false; break; }
            }
            if is_pivot { pivots.push((*values[i].0, current)); }
        }
        pivots
    }

    fn find_pivot_high(&self, points: &BTreeMap<u64, f64>, lb_l: usize, lb_r: usize) -> Vec<(u64, f64)> {
        let values: Vec<(&u64, &f64)> = points.iter().collect();
        let mut pivots = Vec::new();
        if values.len() < lb_l + lb_r + 1 { return pivots; }

        for i in lb_l..values.len() - lb_r {
            let current = *values[i].1;
            let mut is_pivot = true;
            // Check left
            for j in (i - lb_l)..i {
                if *values[j].1 >= current { is_pivot = false; break; }
            }
            if !is_pivot { continue; }
            // Check right
            for j in (i + 1)..=(i + lb_r) {
                if *values[j].1 >= current { is_pivot = false; break; }
            }
            if is_pivot { pivots.push((*values[i].0, current)); }
        }
        pivots
    }



    fn draw_divergence(&self, ctx: &mut UiContext, t1: u64, c1: f64, t2: u64, c2: f64, color: Color, label: &str) {
        let (earliest, latest) = ctx.viewport_range;
        let (y_min, y_max) = self.y_bounds_in_range(earliest, latest).unwrap_or((-1.0, 1.0));
        let value_range = (y_max - y_min).max(1e-12);
        let time_range = (latest as f64 - earliest as f64).max(1.0);
        let bounds = ctx.bounds;

        let get_x = |ts: u64| -> f32 {
            let normalized = (ts as f64 - earliest as f64) / time_range;
            let x = bounds.x + (normalized as f32 * bounds.width);
            x.clamp(-1e6, 1e6)
        };
        let get_y = |val: f64| -> f32 {
            let normalized = ((val - y_min) / value_range).clamp(-1e6, 1e6);
            let y = bounds.y + ((1.0 - normalized as f32) * bounds.height);
            y.clamp(-1e6, 1e6)
        };

        let x1 = get_x(t1);
        let y1 = get_y(c1);
        let x2 = get_x(t2);
        let y2 = get_y(c2);

        if x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite() {
            let path = Path::new(|builder| {
                builder.move_to(Point::new(x1, y1));
                builder.line_to(Point::new(x2, y2));
            });
            ctx.frame.stroke(&path, Stroke::default().with_color(color).with_width(2.0 / ctx.scaling));
            
            // Render Label
            let is_bullish = color.g > color.r; // Simple check for green vs red
            let label_y = if is_bullish { y2 + 10.0 } else { y2 - 20.0 };
            
            if label_y.is_finite() {
                ctx.frame.fill_text(iced::widget::canvas::Text {
                    content: label.to_string(),
                    position: Point::new(x2, label_y),
                    color,
                    size: iced::Pixels(11.0),
                    align_x: iced::Alignment::Center.into(),
                    ..Default::default()
                });
            }
        }
    }
}

impl Indicator for Cvd {
    fn new(config: IndicatorConfig) -> Self {
        let mut up_color = Color::from_rgb(0.1, 0.8, 0.4);
        let mut down_color = Color::from_rgb(0.9, 0.2, 0.3);
        let mut display_mode = DisplayMode::Line;
        let mut calculation_mode = CalculationMode::Ohlc;
        let mut use_gradient = true;
        let mut session_mode = SessionMode::Daily;
        let mut show_divergence = true;
        
        let mut ma_length = 20;
        let mut plot_ma = false;
        let mut lb_l = 1;
        let mut lb_r = 2;
        let mut range_upper = 60;
        let mut range_lower = 5;
        let mut plot_bull = true;
        let mut plot_hidden_bull = true;
        let mut plot_bear = true;
        let mut plot_hidden_bear = true;

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
                Setting::Int { name, value, .. } if name == "MA Length" => {
                    ma_length = *value as usize;
                }
                Setting::Bool { name, value, .. } if name == "Plot MA" => {
                    plot_ma = *value;
                }
                Setting::Int { name, value, .. } if name == "Pivot Lookback Left" => {
                    lb_l = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Pivot Lookback Right" => {
                    lb_r = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Max Lookback Range" => {
                    range_upper = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Min Lookback Range" => {
                    range_lower = *value as usize;
                }
                Setting::Bool { name, value, .. } if name == "Show Bullish" => {
                    plot_bull = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Hidden Bullish" => {
                    plot_hidden_bull = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Bearish" => {
                    plot_bear = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Hidden Bearish" => {
                    plot_hidden_bear = *value;
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
            Setting::Int {
                name: "MA Length".to_string(),
                value: ma_length as i32,
                min: 1,
                max: 500,
                step: 1,
                description: Some("SMA length for CVD".to_string()),
            },
            Setting::Bool {
                name: "Plot MA".to_string(),
                value: plot_ma,
                description: Some("Show CVD SMA line".to_string()),
            },
            Setting::Bool {
                name: "Show Divergence".to_string(),
                value: show_divergence,
                description: Some("Show price vs CVD divergence markers".to_string()),
            },
            Setting::Int {
                name: "Pivot Lookback Left".to_string(),
                value: lb_l as i32,
                min: 1,
                max: 50,
                step: 1,
                description: None,
            },
            Setting::Int {
                name: "Pivot Lookback Right".to_string(),
                value: lb_r as i32,
                min: 1,
                max: 50,
                step: 1,
                description: None,
            },
            Setting::Int {
                name: "Min Lookback Range".to_string(),
                value: range_lower as i32,
                min: 1,
                max: 200,
                step: 1,
                description: None,
            },
            Setting::Int {
                name: "Max Lookback Range".to_string(),
                value: range_upper as i32,
                min: 1,
                max: 500,
                step: 1,
                description: None,
            },
            Setting::Bool {
                name: "Show Bullish".to_string(),
                value: plot_bull,
                description: None,
            },
            Setting::Bool {
                name: "Show Hidden Bullish".to_string(),
                value: plot_hidden_bull,
                description: None,
            },
            Setting::Bool {
                name: "Show Bearish".to_string(),
                value: plot_bear,
                description: None,
            },
            Setting::Bool {
                name: "Show Hidden Bearish".to_string(),
                value: plot_hidden_bear,
                description: None,
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
            last_candle_time: None,
            last_candle_delta: 0.0,
            last_candle_ohlc: (0.0, 0.0, 0.0, 0.0),
            prev_candle_ohlc: (0.0, 0.0, 0.0, 0.0),
            price_points: BTreeMap::new(),
            cvd_ma_points: BTreeMap::new(),
            ma_length,
            plot_ma,
            lb_l,
            lb_r,
            range_upper,
            range_lower,
            plot_bull,
            plot_hidden_bull,
            plot_bear,
            plot_hidden_bear,
            session_mode,
            last_session_ts: 0,
            last_interval: 60000, // Default to 1m
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
            }
        }
        self.last_candle_time = Some(kline.time);

        self.cumulative_delta += delta;
        self.last_candle_delta = delta;
        self.last_candle_ohlc = (o, h, l, c);
        
        self.delta_points.insert(kline.time, self.cumulative_delta);
        self.price_points.insert(kline.time, kline.close.to_f32() as f64);

        // Update SMA
        if self.delta_points.len() >= self.ma_length {
            let values: Vec<f64> = self.delta_points.values().rev().take(self.ma_length).cloned().collect();
            let sum: f64 = values.iter().sum();
            self.cvd_ma_points.insert(kline.time, sum / self.ma_length as f64);
        }
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

            let interval = if self.last_interval > 0 { self.last_interval } else { 60000 };
            let candle_start = (tick.time / interval) * interval;

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
            let x = bounds.x + (normalized as f32 * bounds.width);
            x.clamp(-1e6, 1e6)
        };

        let get_y = |val: f64| -> f32 {
            let normalized = ((val - y_min) / value_range).clamp(-1e9, 1e9);
            let y = bounds.y + ((1.0 - normalized as f32) * bounds.height);
            y.clamp(-1e6, 1e6)
        };

        // 1. Draw CVD SMA if enabled
        if self.plot_ma {
            let ma_points: Vec<(u64, f64)> = self.cvd_ma_points.range(earliest..=latest)
                .map(|(&t, &v)| (t, v))
                .collect();
            
            if !ma_points.is_empty() {
                let path = Path::new(|builder| {
                    let mut first = true;
                    for &(t, v) in &ma_points {
                        let x = get_x(t);
                        let y = get_y(v);
                        if !x.is_finite() || !y.is_finite() { continue; }
                        let p = Point::new(x, y);
                        if first { builder.move_to(p); first = false; }
                        else { builder.line_to(p); }
                    }
                });
                ctx.frame.stroke(&path, Stroke::default()
                    .with_color(Color::WHITE)
                    .with_width(1.0 / scaling));
            }
        }

        let points: Vec<(u64, f64)> = self.delta_points.range(earliest..=latest)
            .map(|(&t, &v)| (t, v))
            .collect();

        if points.is_empty() { return; }

        // 2. Main CVD Line / Histogram
        if self.display_mode == DisplayMode::Line {
            let path = Path::new(|builder| {
                let mut first = true;
                for &(t, v) in &points {
                    let x = get_x(t);
                    let y = get_y(v);
                    if !x.is_finite() || !y.is_finite() { continue; }
                    let p = Point::new(x, y);
                    if first { builder.move_to(p); first = false; }
                    else { builder.line_to(p); }
                }
            });

            ctx.frame.stroke(&path, Stroke::default()
                .with_color(Color::from_rgb(1.0, 1.0, 0.0)) // Yellow CVD line
                .with_width(2.0 / scaling));
        } else {
            let cell_width = bounds.width / points.len().max(1) as f32;
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
                    ctx.frame.fill_rectangle(Point::new(x - cell_width * 0.4, top), Size::new(cell_width * 0.8, height), color);
                }
            }
        }

        // 3. Divergence Detection & Rendering
        if self.show_divergence && points.len() > self.lb_r + self.lb_l {
            let price_lows = self.find_pivot_low(&self.price_points, self.lb_l, self.lb_r);
            let price_highs = self.find_pivot_high(&self.price_points, self.lb_l, self.lb_r);

            // Bullish / Hidden Bullish
            if self.plot_bull || self.plot_hidden_bull {
                for i in 1..price_lows.len() {
                    let (t2, p2) = price_lows[i];
                    let (t1, p1) = price_lows[i-1];
                    
                    let bar_dist = if ctx.interval > 0 {
                        (t2.saturating_sub(t1) / ctx.interval) as usize
                    } else {
                        0
                    };

                    if bar_dist < self.range_lower || bar_dist > self.range_upper {
                        continue;
                    }
                    
                    if let (Some(c1), Some(c2)) = (self.delta_points.get(&t1), self.delta_points.get(&t2)) {
                        // Regular Bullish: Price Lower Low, Osc Higher Low
                        if self.plot_bull && p2 < p1 && *c2 > *c1 {
                             self.draw_divergence(ctx, t1, *c1, t2, *c2, Color::from_rgb(0.0, 1.0, 0.0), "Bull");
                        }
                        // Hidden Bullish: Price Higher Low, Osc Lower Low
                        if self.plot_hidden_bull && p2 > p1 && *c2 < *c1 {
                             self.draw_divergence(ctx, t1, *c1, t2, *c2, Color::from_rgb(0.0, 1.0, 0.0), "H Bull");
                        }
                    }
                }
            }

            // Bearish / Hidden Bearish
            if self.plot_bear || self.plot_hidden_bear {
                for i in 1..price_highs.len() {
                    let (t2, p2) = price_highs[i];
                    let (t1, p1) = price_highs[i-1];

                    let bar_dist = if ctx.interval > 0 {
                        (t2.saturating_sub(t1) / ctx.interval) as usize
                    } else {
                        0
                    };

                    if bar_dist < self.range_lower || bar_dist > self.range_upper {
                        continue;
                    }

                    if let (Some(c1), Some(c2)) = (self.delta_points.get(&t1), self.delta_points.get(&t2)) {
                        // Regular Bearish: Price Higher High, Osc Lower High
                        if self.plot_bear && p2 > p1 && *c2 < *c1 {
                             self.draw_divergence(ctx, t1, *c1, t2, *c2, Color::from_rgb(1.0, 0.0, 0.0), "Bear");
                        }
                        // Hidden Bearish: Price Lower High, Osc Higher High
                        if self.plot_hidden_bear && p2 < p1 && *c2 > *c1 {
                             self.draw_divergence(ctx, t1, *c1, t2, *c2, Color::from_rgb(1.0, 0.0, 0.0), "H Bear");
                        }
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
        self.delta_points.clear();
        self.price_points.clear();
        self.cvd_ma_points.clear();
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
                Setting::Int { name, value, .. } if name == "MA Length" => {
                    self.ma_length = *value as usize;
                }
                Setting::Bool { name, value, .. } if name == "Plot MA" => {
                    self.plot_ma = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Divergence" => {
                    self.show_divergence = *value;
                }
                Setting::Int { name, value, .. } if name == "Pivot Lookback Left" => {
                    self.lb_l = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Pivot Lookback Right" => {
                    self.lb_r = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Max Lookback Range" => {
                    self.range_upper = *value as usize;
                }
                Setting::Int { name, value, .. } if name == "Min Lookback Range" => {
                    self.range_lower = *value as usize;
                }
                Setting::Bool { name, value, .. } if name == "Show Bullish" => {
                    self.plot_bull = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Hidden Bullish" => {
                    self.plot_hidden_bull = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Bearish" => {
                    self.plot_bear = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Hidden Bearish" => {
                    self.plot_hidden_bear = *value;
                }
                Setting::Color { name, value, .. } if name == "Up Color" => {
                    self.up_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Down Color" => {
                    self.down_color = *value;
                }
                _ => {}
            }
        }
        if session_changed {
            self.reset();
        }
    }
}
