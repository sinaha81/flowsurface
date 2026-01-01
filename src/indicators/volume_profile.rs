// Volume Profile Indicator
// Displays volume distribution across price levels with POC, VAH, and VAL

use super::{Indicator, IndicatorConfig, Setting, UiContext};
use exchange::{Trade, Kline};
use iced::widget::canvas::Path;
use iced::Color;
use std::collections::BTreeMap;

pub struct SessionProfile {
    pub price_volumes: BTreeMap<i64, (f64, f64)>,
    pub poc: Option<f64>,
    pub vah: Option<f64>,
    pub val: Option<f64>,
    pub max_vol: f64,
}

impl SessionProfile {
    pub fn new() -> Self {
        Self {
            price_volumes: BTreeMap::new(),
            poc: None, vah: None, val: None,
            max_vol: 0.0,
        }
    }
}

pub struct VolumeProfile {
    name: String,
    settings: Vec<Setting>,
    
    // Multi-session profile data
    sessions: BTreeMap<u64, SessionProfile>,
    current_session_start: u64,
    session_start_hour: i32,
    session_start_minute: i32,
    
    // Configuration
    session_mode: super::SessionMode,
    tick_size: f64,
    value_area_percent: f64,
    
    // Visualization
    profile_color: Color,
    poc_color: Color,
    va_color: Color,
    show_poc: bool,
    show_va: bool,
    side: VolumeProfileSide,
    width_percent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeProfileSide {
    Left,
    Right,
}

impl VolumeProfile {
    fn price_to_tick(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    fn is_new_session(&self, timestamp: u64) -> bool {
        if self.session_mode == super::SessionMode::Chart {
            return false;
        }
        if self.current_session_start == 0 {
            return true;
        }

        let offset_ms = (self.session_start_hour as i64 * 3600 + self.session_start_minute as i64 * 60) * 1000;
        let current_eff = timestamp as i64 - offset_ms;
        let last_eff = self.current_session_start as i64 - offset_ms;

        self.session_mode.is_new_session(current_eff as u64, last_eff as u64)
    }

    fn tick_to_price(&self, tick: i64) -> f64 {
        tick as f64 * self.tick_size
    }


    fn calculate_session_levels(profile: &mut SessionProfile, tick_size: f64, va_percent: f64) {
        if profile.price_volumes.is_empty() {
            profile.poc = None; profile.vah = None; profile.val = None;
            return;
        }

        let tick_to_price = |tick: i64| tick as f64 * tick_size;

        // 1. Find POC
        let (poc_tick, _) = profile.price_volumes.iter()
            .max_by(|(_, v1), (_, v2)| (v1.0 + v1.1).partial_cmp(&(v2.0 + v2.1)).unwrap())
            .unwrap();
        
        profile.poc = Some(tick_to_price(*poc_tick));

        // 2. 68% Value Area expansion
        let total_volume: f64 = profile.price_volumes.values().map(|(b, s)| b + s).sum();
        let target_volume = total_volume * va_percent;

        let poc_vol = profile.price_volumes.get(poc_tick).map(|(b, s)| b + s).unwrap_or(0.0);
        let mut va_volume = poc_vol;
        let mut upper_tick = *poc_tick;
        let mut lower_tick = *poc_tick;

        while va_volume < target_volume {
            let upper_next_vol = profile.price_volumes.get(&(upper_tick + 1)).map(|(b, s)| b + s).unwrap_or(0.0);
            let lower_next_vol = profile.price_volumes.get(&(lower_tick - 1)).map(|(b, s)| b + s).unwrap_or(0.0);

            if upper_next_vol == 0.0 && lower_next_vol == 0.0 { break; }

            if upper_next_vol >= lower_next_vol {
                va_volume += upper_next_vol;
                upper_tick += 1;
            } else {
                va_volume += lower_next_vol;
                lower_tick -= 1;
            }
        }

        profile.vah = Some(tick_to_price(upper_tick));
        profile.val = Some(tick_to_price(lower_tick));
        
        profile.max_vol = profile.price_volumes.values().map(|(b, s)| b + s).fold(0.0, f64::max);

        if profile.vah.map_or(false, |v| !v.is_finite()) { profile.vah = None; }
        if profile.val.map_or(false, |v| !v.is_finite()) { profile.val = None; }
    }

    fn price_to_y(&self, price: f64, bounds: iced::Rectangle, price_range: (f64, f64)) -> f32 {
        if !price.is_finite() {
            return bounds.y + bounds.height / 2.0;
        }
        let (highest, lowest) = price_range;
        let range = highest - lowest;
        if range.abs() < 1e-8 || !range.is_finite() { return bounds.y + bounds.height / 2.0; }
        let normalized = (price - lowest) / range;
        let y = bounds.y + (1.0 - normalized as f32) * bounds.height;
        if y.is_finite() { y } else { bounds.y + bounds.height / 2.0 }
    }
}

impl Indicator for VolumeProfile {
    fn new(config: IndicatorConfig) -> Self {
        let mut session_mode = super::SessionMode::Daily;
        let mut va_percent = 0.68;
        let mut profile_color = Color::from_rgba(0.4, 0.4, 0.9, 0.4);
        let mut poc_color = Color::from_rgb(1.0, 0.6, 0.0);
        let mut va_color = Color::from_rgba(0.8, 0.8, 0.0, 0.2);
        let mut side = VolumeProfileSide::Right;
        let mut width_percent = 30.0;

        let mut session_start_hour = 0;
        let mut session_start_minute = 0;

        for setting in &config.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    session_mode = super::SessionMode::from_str(value);
                }
                Setting::Int { name, value, .. } if name == "Start Hour" => {
                    session_start_hour = *value;
                }
                Setting::Int { name, value, .. } if name == "Start Minute" => {
                    session_start_minute = *value;
                }
                Setting::Enum { name, value, .. } if name == "Align" => {
                    side = if value == "Left" { VolumeProfileSide::Left } else { VolumeProfileSide::Right };
                }
                Setting::Float { name, value, .. } if name == "Value Area %" => {
                    va_percent = *value / 100.0;
                }
                Setting::Float { name, value, .. } if name == "Width %" => {
                    width_percent = *value as f32;
                }
                Setting::Color { name, value, .. } if name == "Profile Color" => {
                    profile_color = *value;
                }
                Setting::Color { name, value, .. } if name == "POC Color" => {
                    poc_color = *value;
                }
                Setting::Color { name, value, .. } if name == "VA Color" => {
                    va_color = *value;
                }
                _ => {}
            }
        }

        let settings = vec![
            Setting::Enum {
                name: "Session Mode".to_string(),
                value: session_mode.as_str().to_string(),
                options: vec![
                    "Total".to_string(),
                    "Daily".to_string(), 
                    "Weekly".to_string(), 
                    "Monthly".to_string(),
                    "Yearly".to_string(),
                ],
                description: None,
            },
            Setting::Enum {
                name: "Align".to_string(),
                value: if side == VolumeProfileSide::Left { "Left".to_string() } else { "Right".to_string() },
                options: vec!["Left".to_string(), "Right".to_string()],
                description: None,
            },
            Setting::Float {
                name: "Width %".to_string(),
                value: width_percent as f64,
                min: 5.0, max: 100.0, step: 5.0,
                description: None,
            },
            Setting::Float {
                name: "Value Area %".to_string(),
                value: va_percent * 100.0,
                min: 0.0, max: 100.0, step: 1.0,
                description: Some("Typically 68% for value area".to_string()),
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
                name: "Profile Color".to_string(),
                value: profile_color,
                description: None,
            },
            Setting::Color {
                name: "VA Color".to_string(),
                value: va_color,
                description: None,
            },
            Setting::Color {
                name: "POC Color".to_string(),
                value: poc_color,
                description: None,
            },
        ];

        Self {
            name: config.name,
            settings,
            sessions: BTreeMap::new(),
            current_session_start: 0,
            session_start_hour,
            session_start_minute,
            session_mode,
            tick_size: 1.0, 
            value_area_percent: va_percent,
            profile_color,
            poc_color,
            va_color,
            show_poc: true,
            show_va: true,
            side,
            width_percent,
        }
    }

    fn update_kline(&mut self, kline: &Kline) {
        let ts = kline.time;
        if self.is_new_session(ts) {
            self.current_session_start = ts;
        }
        let buy_vol = kline.volume.0 as f64;
        let sell_vol = kline.volume.1 as f64;
        let avg_price = (kline.high.to_f32() + kline.low.to_f32() + kline.close.to_f32()) as f64 / 3.0;
        let tick = self.price_to_tick(avg_price);

        // Ensure session exists
        let profile = self.sessions.entry(self.current_session_start).or_insert_with(|| SessionProfile::new());

        if !buy_vol.is_finite() || !sell_vol.is_finite() || !avg_price.is_finite() {
            return;
        }
        
        let entry = profile.price_volumes.entry(tick).or_insert((0.0, 0.0));
        entry.0 += buy_vol;
        entry.1 += sell_vol;

        Self::calculate_session_levels(profile, self.tick_size, self.value_area_percent);
    }

    fn update_tick(&mut self, tick: &Trade) {
        let ts = tick.time;
        if self.is_new_session(ts) {
            self.current_session_start = ts;
        }

        let tick_idx = self.price_to_tick(tick.price.to_f32() as f64);
        let qty = tick.qty as f64;
        let is_sell = tick.is_sell;

        let profile = self.sessions.entry(self.current_session_start).or_insert_with(|| SessionProfile::new());

        let entry = profile.price_volumes.entry(tick_idx).or_insert((0.0, 0.0));
        if !is_sell { entry.0 += qty; } else { entry.1 += qty; }

        Self::calculate_session_levels(profile, self.tick_size, self.value_area_percent);
    }

    fn render(&self, ctx: &mut UiContext) {
        if self.sessions.is_empty() { return; }

        let bounds = ctx.bounds;
        let p_range = ctx.price_range;
        let frame = &mut ctx.frame;
        let (earliest, latest) = ctx.viewport_range;

        // Iterate through visible sessions
        for (&start_ts, profile) in self.sessions.range(..=latest) {
            if profile.price_volumes.is_empty() { continue; }
            
            // Determine session end for bounds (next session start or latest visible)
            let next_ts = self.sessions.range((start_ts + 1)..).next()
                .map(|(ts, _)| *ts)
                .unwrap_or(latest)
                .min(latest);
            
            if next_ts < earliest { continue; }

            let session_start_x = ctx.latest_x as f32 - ((ctx.viewport_range.1.saturating_sub(start_ts)) as f32 / ctx.interval as f32) * ctx.cell_width;
            let session_end_x = ctx.latest_x as f32 - ((ctx.viewport_range.1.saturating_sub(next_ts)) as f32 / ctx.interval as f32) * ctx.cell_width;
            
            let session_width = (session_end_x - session_start_x).abs();
            if session_width < 1.0 { continue; }

            let max_bar_width = session_width * (self.width_percent / 100.0);
            let (highest, lowest) = p_range;

            for (&tick, &(b, s)) in &profile.price_volumes {
                let price = self.tick_to_price(tick);
                if price < lowest || price > highest { continue; }

                let total = b + s;
                let bar_width = (total / profile.max_vol) as f32 * max_bar_width;
                let y = self.price_to_y(price, bounds, p_range);
                
                let is_in_va = if let (Some(vah), Some(val)) = (profile.vah, profile.val) {
                    price <= vah && price >= val
                } else {
                    false
                };

                let bar_color = if is_in_va { self.profile_color } else { self.profile_color.scale_alpha(0.4) };

                let x = if self.side == VolumeProfileSide::Left {
                    session_start_x
                } else {
                    session_end_x - bar_width
                };

                frame.fill_rectangle(
                    iced::Point::new(x, y - 1.0),
                    iced::Size::new(bar_width, 2.0_f32.max(ctx.cell_height / 2.0)),
                    bar_color
                );
            }

            // POC Line per session
            if self.show_poc && let Some(poc) = profile.poc {
                let y = self.price_to_y(poc, bounds, p_range);
                let path = Path::line(iced::Point::new(session_start_x, y), iced::Point::new(session_end_x, y));
                frame.stroke(&path, iced::widget::canvas::Stroke::default().with_color(self.poc_color).with_width(1.0));
            }

            // VA Lines per session
            if self.show_va && let (Some(vah), Some(val)) = (profile.vah, profile.val) {
                let y_vah = self.price_to_y(vah, bounds, p_range);
                let y_val = self.price_to_y(val, bounds, p_range);
                let path_h = Path::line(iced::Point::new(session_start_x, y_vah), iced::Point::new(session_end_x, y_vah));
                let path_l = Path::line(iced::Point::new(session_start_x, y_val), iced::Point::new(session_end_x, y_val));
                frame.stroke(&path_h, iced::widget::canvas::Stroke::default().with_color(self.va_color).with_width(1.0));
                frame.stroke(&path_l, iced::widget::canvas::Stroke::default().with_color(self.va_color).with_width(1.0));
            }
        }
    }

    fn name(&self) -> &str { &self.name }
    fn get_settings(&mut self) -> &mut Vec<Setting> { &mut self.settings }
    fn settings(&self) -> &[Setting] { &self.settings }
    fn is_overlay(&self) -> bool { true }
    fn y_bounds(&self) -> Option<(f64, f64)> {
        if self.sessions.is_empty() { return Some((-1.0, 1.0)); }
        
        let mut min_p = f64::MAX;
        let mut max_p = f64::MIN;
        let mut found = false;

        for profile in self.sessions.values() {
            for &tick in profile.price_volumes.keys() {
                let p = self.tick_to_price(tick);
                if p.is_finite() {
                    if p < min_p { min_p = p; }
                    if p > max_p { max_p = p; }
                    found = true;
                }
            }
        }

        if !found {
            return Some((-1.0, 1.0));
        }

        let range = (max_p - min_p).abs();
        if range < 1e-8 {
            return Some((min_p - 1.0, max_p + 1.0));
        }

        Some((min_p, max_p))
    }
    fn reset(&mut self) {
        self.sessions.clear();
        self.current_session_start = 0;
    }

    fn sync_settings(&mut self) {
        let mut session_changed = false;
        let mut needs_recalc = false;
        for setting in &self.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    let new_mode = super::SessionMode::from_str(value);
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
                Setting::Enum { name, value, .. } if name == "Align" => {
                    self.side = if value == "Left" { VolumeProfileSide::Left } else { VolumeProfileSide::Right };
                }
                Setting::Float { name, value, .. } if name == "Width %" => {
                    self.width_percent = *value as f32;
                }
                Setting::Float { name, value, .. } if name == "Value Area %" => {
                    self.value_area_percent = *value / 100.0;
                    needs_recalc = true;
                }
                Setting::Color { name, value, .. } if name == "Profile Color" => {
                    self.profile_color = *value;
                }
                Setting::Color { name, value, .. } if name == "VA Color" => {
                    self.va_color = *value;
                }
                Setting::Color { name, value, .. } if name == "POC Color" => {
                    self.poc_color = *value;
                }
                _ => {}
            }
        }
        if session_changed {
            self.reset();
        } else if needs_recalc {
            for profile in self.sessions.values_mut() {
                Self::calculate_session_levels(profile, self.tick_size, self.value_area_percent);
            }
        }
    }

    fn set_tick_size(&mut self, size: f64) {
        if (self.tick_size - size).abs() < f64::EPSILON {
            return;
        }

        let old_tick_size = self.tick_size;
        self.tick_size = size;

        if self.sessions.is_empty() {
            return;
        }

        // Migrate existing volume data to new tick size buckets for ALL sessions
        for profile in self.sessions.values_mut() {
            let mut new_volumes: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
            
            for (tick, (buy, sell)) in profile.price_volumes.iter() {
                let price = *tick as f64 * old_tick_size;
                let new_tick = (price / size).round() as i64;
                
                let entry = new_volumes.entry(new_tick).or_insert((0.0, 0.0));
                entry.0 += buy;
                entry.1 += sell;
            }

            profile.price_volumes = new_volumes;
            Self::calculate_session_levels(profile, self.tick_size, self.value_area_percent);
        }
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
}
