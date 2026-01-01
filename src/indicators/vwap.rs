// VWAP (Volume Weighted Average Price) Indicator
// Supports anchored sessions with standard deviation bands

use super::{Indicator, IndicatorConfig, Setting, UiContext, SessionMode};
use exchange::{Trade, Kline};
use iced::widget::canvas::{Path, Stroke};
use iced::{Color, Point};
use std::collections::BTreeMap;

pub struct Vwap {
    name: String,
    settings: Vec<Setting>,
    
    // VWAP calculation state
    cumulative_pv: f64,
    cumulative_volume: f64,
    cumulative_pv_sq: f64,
    
    // Result values for current point
    vwap_value: f64,
    std_dev: f64,
    
    // Session management
    session_mode: SessionMode,
    last_session_boundary: u64,
    session_start_hour: i32,
    session_start_minute: i32,
    
    // Visualization
    line_color: Color,
    band_color: Color,
    band_alpha: f32, // Added customization
    show_bands: bool,
    fill_bands: bool,
    // Stats for the current (live) candle to avoid double counting
    last_candle_time: Option<u64>,
    last_candle_pv: f64,
    last_candle_vol: f64,
    last_candle_pv_sq: f64,
    std_multipliers: Vec<f64>,
    last_interval: u64,
    
    // Data storage (timestamp -> (vwap, std_dev))
    points: BTreeMap<u64, (f64, f64)>,
}

impl Vwap {
    fn is_new_session(&self, timestamp: u64) -> bool {
        if self.session_mode == SessionMode::Chart {
             return false;
        }

        if self.last_session_boundary == 0 {
            return true;
        }

        let offset_ms = (self.session_start_hour as i64 * 3600 + self.session_start_minute as i64 * 60) * 1000;
        let current_eff = timestamp as i64 - offset_ms;
        let last_eff = self.last_session_boundary as i64 - offset_ms;

        self.session_mode.is_new_session(current_eff as u64, last_eff as u64)
    }

    fn reset_stats(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_volume = 0.0;
        self.cumulative_pv_sq = 0.0;
        self.vwap_value = 0.0;
        self.std_dev = 0.0;
        self.last_candle_time = None;
        self.last_candle_pv = 0.0;
        self.last_candle_vol = 0.0;
        self.last_candle_pv_sq = 0.0;
    }

    fn push_point(&mut self, timestamp: u64, val: (f64, f64)) {
        self.points.insert(timestamp, val);
        
        if self.points.len() > 10000 {
            let keys: Vec<_> = self.points.keys().take(1000).cloned().collect();
            for k in keys { self.points.remove(&k); }
        }
    }
}

impl Indicator for Vwap {
    fn new(config: IndicatorConfig) -> Self {
        let mut session_mode = SessionMode::Daily;
        let mut show_bands = true;
        let mut fill_bands = true;
        let mut line_color = Color::from_rgb(1.0, 1.0, 0.0); // Bright Yellow
        let mut band_color = Color::from_rgb(1.0, 0.5, 0.0); // Orange
        let mut band_alpha = 0.15;

        let mut session_start_hour = 0;
        let mut session_start_minute = 0;

        for setting in &config.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    session_mode = SessionMode::from_str(value);
                }
                Setting::Int { name, value, .. } if name == "Start Hour" => {
                    session_start_hour = *value;
                }
                Setting::Int { name, value, .. } if name == "Start Minute" => {
                    session_start_minute = *value;
                }
                Setting::Bool { name, value, .. } if name == "Show Bands" => {
                    show_bands = *value;
                }
                Setting::Bool { name, value, .. } if name == "Fill Bands" => {
                    fill_bands = *value;
                }
                Setting::Color { name, value, .. } if name == "Line Color" => {
                    line_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Band Color" => {
                    band_color = *value;
                }
                Setting::Float { name, value, .. } if name == "Band Alpha" => {
                    band_alpha = *value as f32;
                }
                _ => {}
            }
        }

        let settings = vec![
            Setting::Enum {
                name: "Session Mode".to_string(),
                value: session_mode.as_str().to_string(),
                options: vec!["Total".to_string(), "Session".to_string(), "Daily".to_string(), "Weekly".to_string(), "Monthly".to_string(), "3 Month".to_string(), "6 Month".to_string(), "Yearly".to_string()],
                description: Some("Reset interval".to_string()),
            },
            Setting::Bool {
                name: "Show Bands".to_string(),
                value: show_bands,
                description: Some("Display deviation bands".to_string()),
            },
            Setting::Bool {
                name: "Fill Bands".to_string(),
                value: fill_bands,
                description: Some("Fill area between bands".to_string()),
            },
            Setting::Float {
                name: "Band Alpha".to_string(),
                value: band_alpha as f64,
                min: 0.0,
                max: 1.0,
                step: 0.05,
                description: Some("Transparency of bands".to_string()),
            },
            Setting::Int {
                name: "Start Hour".to_string(),
                value: session_start_hour,
                min: 0, max: 23, step: 1,
                description: Some("Session start hour (UTC)".to_string()),
            },
            Setting::Int {
                name: "Start Minute".to_string(),
                value: session_start_minute,
                min: 0, max: 59, step: 1,
                description: Some("Session start minute".to_string()),
            },
            Setting::Color {
                name: "Line Color".to_string(),
                value: line_color,
                description: None,
            },
            Setting::Color {
                name: "Band Color".to_string(),
                value: band_color,
                description: None,
            },
        ];

        Self {
            name: config.name,
            settings,
            cumulative_pv: 0.0,
            cumulative_volume: 0.0,
            cumulative_pv_sq: 0.0,
            vwap_value: 0.0,
            std_dev: 0.0,
            session_mode,
            last_session_boundary: 0,
            session_start_hour,
            session_start_minute,
            line_color,
            band_color,
            band_alpha,
            show_bands,
            fill_bands,
            std_multipliers: vec![1.0, 2.0, 3.0],
            points: BTreeMap::new(),
            last_candle_time: None,
            last_candle_pv: 0.0,
            last_candle_vol: 0.0,
            last_candle_pv_sq: 0.0,
            last_interval: 0,
        }
    }

    fn update_kline(&mut self, kline: &Kline) {
        if self.is_new_session(kline.time) {
            self.reset_stats();
            self.last_session_boundary = kline.time;
        }

        let typical_price = (kline.high.to_f32() + kline.low.to_f32() + kline.close.to_f32()) / 3.0;
        let volume = (kline.volume.0 + kline.volume.1) as f64;
        let pv = typical_price as f64 * volume;
        let pv_sq = (typical_price as f64).powi(2) * volume;

        // Handle partial kline updates for the same timestamp
        if let Some(last_time) = self.last_candle_time {
            if last_time == kline.time {
                self.cumulative_pv -= self.last_candle_pv;
                self.cumulative_volume -= self.last_candle_vol;
                self.cumulative_pv_sq -= self.last_candle_pv_sq;
            } else {
                if kline.time > last_time {
                    self.last_interval = kline.time - last_time;
                }
                self.last_candle_time = Some(kline.time);
            }
        } else {
            self.last_candle_time = Some(kline.time);
        }

        self.cumulative_pv += pv;
        self.cumulative_volume += volume;
        self.cumulative_pv_sq += pv_sq;
        
        self.last_candle_pv = pv;
        self.last_candle_vol = volume;
        self.last_candle_pv_sq = pv_sq;

        if self.cumulative_volume > 0.0 {
            self.vwap_value = self.cumulative_pv / self.cumulative_volume;
            let mean_pv2 = self.cumulative_pv_sq / self.cumulative_volume;
            let variance = mean_pv2 - self.vwap_value.powi(2);
            self.std_dev = variance.max(0.0).sqrt();
            
            self.push_point(kline.time, (self.vwap_value, self.std_dev));
        }
    }

    fn update_tick(&mut self, tick: &Trade) {
        if self.is_new_session(tick.time) {
            self.reset_stats();
            self.last_session_boundary = tick.time;
        }

        let price = tick.price.to_f32() as f64;
        let volume = tick.qty as f64;
        let pv = price * volume;
        let pv2 = price * price * volume;

        if pv.is_finite() && pv2.is_finite() {
            self.cumulative_pv += pv;
            self.cumulative_volume += volume;
            self.cumulative_pv_sq += pv2;

            let candle_start = if self.last_interval > 0 {
                (tick.time / self.last_interval) * self.last_interval
            } else {
                tick.time
            };

            if let Some(last_time) = self.last_candle_time {
                if candle_start == last_time {
                    self.last_candle_pv += pv;
                    self.last_candle_vol += volume;
                    self.last_candle_pv_sq += pv2;
                } else if candle_start > last_time {
                    self.last_candle_time = Some(candle_start);
                    self.last_candle_pv = pv;
                    self.last_candle_vol = volume;
                    self.last_candle_pv_sq = pv2;
                }
            } else {
                self.last_candle_time = Some(candle_start);
                self.last_candle_pv = pv;
                self.last_candle_vol = volume;
                self.last_candle_pv_sq = pv2;
            }

            if self.cumulative_volume > 0.0 {
                self.vwap_value = self.cumulative_pv / self.cumulative_volume;
                let mean_pv2 = self.cumulative_pv_sq / self.cumulative_volume;
                let variance = mean_pv2 - self.vwap_value.powi(2);
                self.std_dev = variance.max(0.0).sqrt();
                
                self.push_point(tick.time, (self.vwap_value, self.std_dev));
            }
        }
    }

    fn render(&self, ctx: &mut UiContext) {
        if self.points.is_empty() {
            return;
        }

        let (earliest, latest) = ctx.viewport_range;
        if latest <= earliest { return; }

        let scaling = ctx.scaling;
        
        let get_x = |ts: u64| -> f32 {
            let diff = ts as f64 - ctx.latest_x as f64;
            (diff / ctx.interval as f64 * ctx.cell_width as f64) as f32
        };

        let get_y = |price: f64| -> f32 {
            let diff = (price - ctx.base_price) / ctx.tick_size;
            -(diff as f32 * ctx.cell_height)
        };

        let visible_points: Vec<(u64, f64, f64)> = self.points.range(earliest..=latest)
            .map(|(&t, &(v, s))| (t, v, s))
            .collect();

        if visible_points.len() < 2 {
            return;
        }

        // Draw bands first (below the line)
        if self.show_bands {
            for (i, &multiplier) in self.std_multipliers.iter().enumerate().rev() {
                let current_alpha = (self.band_alpha * (1.0 - (i as f32 * 0.2))).max(0.01);
                let current_band_color = Color { a: current_alpha, ..self.band_color };
                
                // Draw filled area for the widest band
                if self.fill_bands && i == 2 && visible_points.len() >= 2 {
                     let fill_path = Path::new(|builder| {
                         let first = visible_points[0];
                         builder.move_to(Point::new(get_x(first.0), get_y(first.1 + first.2 * multiplier)));
                         for p in visible_points.iter().skip(1) {
                             builder.line_to(Point::new(get_x(p.0), get_y(p.1 + p.2 * multiplier)));
                         }
                         for p in visible_points.iter().rev() {
                             builder.line_to(Point::new(get_x(p.0), get_y(p.1 - p.2 * multiplier)));
                         }
                         builder.close();
                     });
                     ctx.frame.fill(&fill_path, Color { a: self.band_alpha * 0.4, ..self.band_color });
                }

                // Stroke upper band
                let u_path = Path::new(|builder| {
                    let first = visible_points[0];
                    builder.move_to(Point::new(get_x(first.0), get_y(first.1 + first.2 * multiplier)));
                    for p in visible_points.iter().skip(1) {
                        builder.line_to(Point::new(get_x(p.0), get_y(p.1 + p.2 * multiplier)));
                    }
                });
                ctx.frame.stroke(&u_path, Stroke::default().with_color(current_band_color).with_width(1.0 / scaling));

                // Stroke lower band
                let l_path = Path::new(|builder| {
                    let first = visible_points[0];
                    builder.move_to(Point::new(get_x(first.0), get_y(first.1 - first.2 * multiplier)));
                    for p in visible_points.iter().skip(1) {
                        builder.line_to(Point::new(get_x(p.0), get_y(p.1 - p.2 * multiplier)));
                    }
                });
                ctx.frame.stroke(&l_path, Stroke::default().with_color(current_band_color).with_width(1.0 / scaling));
            }
        }

        // Draw VWAP line
        let path = Path::new(|builder| {
            let first = visible_points[0];
            builder.move_to(Point::new(get_x(first.0), get_y(first.1)));
            for point in visible_points.iter().skip(1) {
                builder.line_to(Point::new(get_x(point.0), get_y(point.1)));
            }
        });

        ctx.frame.stroke(&path, Stroke::default()
            .with_color(self.line_color)
            .with_width(2.0 / scaling));
    }

    fn name(&self) -> &str { &self.name }
    fn get_settings(&mut self) -> &mut Vec<Setting> { &mut self.settings }
    fn settings(&self) -> &[Setting] { &self.settings }
    fn is_overlay(&self) -> bool { true }

    fn reset(&mut self) {
        self.reset_stats();
        self.points.clear();
    }

    fn element<'a>(&'a self, _ctx: &super::ViewContext) -> iced::Element<'a, crate::chart::Message> {
        iced::widget::column![].into()
    }

    fn sync_settings(&mut self) {
        let mut session_changed = false;
        for setting in &self.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    let new_mode = SessionMode::from_str(value);
                    if self.session_mode != new_mode {
                        self.session_mode = new_mode;
                        session_changed = true;
                    }
                }
                Setting::Int { name, value, .. } if name == "Start Hour" => {
                    if self.session_start_hour != *value {
                        self.session_start_hour = *value;
                        session_changed = true;
                    }
                }
                Setting::Int { name, value, .. } if name == "Start Minute" => {
                    if self.session_start_minute != *value {
                        self.session_start_minute = *value;
                        session_changed = true;
                    }
                }
                Setting::Bool { name, value, .. } if name == "Show Bands" => {
                    self.show_bands = *value;
                }
                Setting::Bool { name, value, .. } if name == "Fill Bands" => {
                    self.fill_bands = *value;
                }
                Setting::Color { name, value, .. } if name == "Line Color" => {
                    self.line_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Band Color" => {
                    self.band_color = *value;
                }
                Setting::Float { name, value, .. } if name == "Band Alpha" => {
                    self.band_alpha = *value as f32;
                }
                _ => {}
            }
        }
        if session_changed {
            self.reset();
        }
    }
}
