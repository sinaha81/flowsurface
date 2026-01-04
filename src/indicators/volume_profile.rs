// Volume Profile Indicator
// Displays volume distribution across price levels for specified sessions

use super::{Indicator, IndicatorConfig, Setting, UiContext, SessionMode, ViewContext};
use exchange::util::{Price, PriceStep};
use exchange::{Trade, Kline};
use iced::{Color, Point};
use std::collections::BTreeMap;

pub struct VolumeProfile {
    name: String,
    settings: Vec<Setting>,
    
    // Data storage (timestamp -> SessionData)
    // We store multiple sessions so we can render historical ones if needed, 
    // but usually we just care about the current and previous.
    sessions: Vec<SessionData>,
    
    // Current session boundary
    last_session_boundary: u64,
    session_mode: SessionMode,
    
    // Configuration
    value_area_pct: f64,
    width_pct: f32,
    placement_right: bool,
    
    // Colors
    poc_color: Color,
    va_color: Color,
    bg_color: Color,
    
    tick_size: PriceStep,
}

struct SessionData {
    start_time: u64,
    end_time: u64,
    bins: BTreeMap<Price, f32>,
    total_volume: f32,
    poc: Option<Price>,
    vah: Option<Price>,
    val: Option<Price>,
}

impl SessionData {
    fn new(start_time: u64) -> Self {
        Self {
            start_time,
            end_time: start_time,
            bins: BTreeMap::new(),
            total_volume: 0.0,
            poc: None,
            vah: None,
            val: None,
        }
    }

    fn add_volume(&mut self, price: Price, volume: f32) {
        *self.bins.entry(price).or_insert(0.0) += volume;
        self.total_volume += volume;

    }

    fn calculate_metrics(&mut self, va_pct: f64) {
        if self.bins.is_empty() { return; }

        // POC
        let mut max_vol = 0.0;
        let mut poc = None;
        for (&price, &vol) in &self.bins {
            if vol > max_vol {
                max_vol = vol;
                poc = Some(price);
            }
        }
        self.poc = poc;

        // Value Area (VAH/VAL)
        if let Some(poc_price) = poc {
            let target_vol = self.total_volume * (va_pct as f32 / 100.0);
            let mut current_vol = *self.bins.get(&poc_price).unwrap_or(&0.0);
            
            let prices: Vec<Price> = self.bins.keys().cloned().collect();
            let p_idx = prices.iter().position(|&p| p == poc_price).unwrap();
            let mut l_ptr = p_idx;
            let mut r_ptr = p_idx;

            while current_vol < target_vol {
                let l_vol = if l_ptr > 0 { *self.bins.get(&prices[l_ptr - 1]).unwrap_or(&0.0) } else { 0.0 };
                let r_vol = if r_ptr < prices.len() - 1 { *self.bins.get(&prices[r_ptr + 1]).unwrap_or(&0.0) } else { 0.0 };

                if l_vol == 0.0 && r_vol == 0.0 { break; }

                if l_vol >= r_vol && l_ptr > 0 {
                    l_ptr -= 1;
                    current_vol += l_vol;
                } else if r_ptr < prices.len() - 1 {
                    r_ptr += 1;
                    current_vol += r_vol;
                } else if l_ptr > 0 {
                    l_ptr -= 1;
                    current_vol += l_vol;
                } else {
                    break;
                }
            }
            
            self.val = Some(prices[l_ptr]);
            self.vah = Some(prices[r_ptr]);
        }
    }
}

impl VolumeProfile {
    fn is_new_session(&self, timestamp: u64) -> bool {
        if self.session_mode == SessionMode::Chart {
            return false;
        }
        if self.last_session_boundary == 0 {
            return true;
        }
        self.session_mode.is_new_session(timestamp, self.last_session_boundary)
    }

    fn current_session_mut(&mut self, timestamp: u64) -> &mut SessionData {
        if self.sessions.is_empty() || self.is_new_session(timestamp) {
            self.last_session_boundary = timestamp;
            self.sessions.push(SessionData::new(timestamp));
            // Keep only last 50 sessions for performance
            if self.sessions.len() > 50 {
                self.sessions.remove(0);
            }
        }
        self.sessions.last_mut().unwrap()
    }
}

impl Indicator for VolumeProfile {
    fn new(config: IndicatorConfig) -> Self {
        let mut session_mode = SessionMode::Daily;
        let mut va_pct = 68.0;
        let mut width_pct = 30.0;
        let mut placement_right = true;
        let mut poc_color = Color::from_rgb(1.0, 0.0, 0.0);
        let mut va_color = Color::from_rgba(0.2, 0.2, 0.8, 0.5);
        let mut bg_color = Color::from_rgba(0.5, 0.5, 0.5, 0.2);

        for setting in &config.settings {
            match setting {
                Setting::Enum { name, value, .. } if name == "Session Mode" => {
                    session_mode = SessionMode::from_str(value);
                }
                Setting::Float { name, value, .. } if name == "Value Area %" => {
                    va_pct = *value;
                }
                Setting::Int { name, value, .. } if name == "Width %" => {
                    width_pct = *value as f32;
                }
                Setting::Bool { name, value, .. } if name == "Placement Right" => {
                    placement_right = *value;
                }
                Setting::Color { name, value, .. } if name == "POC Color" => {
                    poc_color = *value;
                }
                Setting::Color { name, value, .. } if name == "VA Color" => {
                    va_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Background Color" => {
                    bg_color = *value;
                }
                _ => {}
            }
        }

        let settings = vec![
            Setting::Enum {
                name: "Session Mode".to_string(),
                value: session_mode.as_str().to_string(),
                options: vec!["Total".to_string(), "Session".to_string(), "Daily".to_string(), "Weekly".to_string(), "Monthly".to_string(), "Yearly".to_string()],
                description: Some("Reset interval".to_string()),
            },
            Setting::Float {
                name: "Value Area %".to_string(),
                value: va_pct,
                min: 0.0, max: 100.0, step: 1.0,
                description: Some("Highlight inner volume percentage".to_string()),
            },
            Setting::Int {
                name: "Width %".to_string(),
                value: width_pct as i32,
                min: 10, max: 100, step: 5,
                description: Some("Histogram width".to_string()),
            },
            Setting::Bool {
                name: "Placement Right".to_string(),
                value: placement_right,
                description: Some("Draw histogram on the right of session".to_string()),
            },
            Setting::Color {
                name: "POC Color".to_string(),
                value: poc_color,
                description: None,
            },
            Setting::Color {
                name: "VA Color".to_string(),
                value: va_color,
                description: None,
            },
            Setting::Color {
                name: "Background Color".to_string(),
                value: bg_color,
                description: None,
            },
        ];

        Self {
            name: config.name,
            settings,
            sessions: Vec::new(),
            last_session_boundary: 0,
            session_mode,
            value_area_pct: va_pct,
            width_pct,
            placement_right,
            poc_color,
            va_color,
            bg_color,
            tick_size: PriceStep::from_f32(0.000001), // Default
        }
    }

    fn update_kline(&mut self, kline: &Kline) {
        let tick_size = self.tick_size;
        let va_pct = self.value_area_pct;
        let session = self.current_session_mut(kline.time);
        session.end_time = kline.time;
        
        let low_bin = kline.low.round_to_step(tick_size);
        let high_bin = kline.high.round_to_step(tick_size);
        
        let steps = Price::steps_between_inclusive(low_bin, high_bin, tick_size).unwrap_or(1);
        let vol_per_step = (kline.volume.0 + kline.volume.1) as f32 / steps as f32;
        
        let mut curr = low_bin;
        for _ in 0..steps {
            session.add_volume(curr, vol_per_step);
            curr = curr.add_steps(1, tick_size);
        }
        
        session.calculate_metrics(va_pct);
    }

    fn update_tick(&mut self, tick: &Trade) {
        let tick_size = self.tick_size;
        let va_pct = self.value_area_pct;
        let session = self.current_session_mut(tick.time);
        session.end_time = session.end_time.max(tick.time);
        
        let price_bin = tick.price.round_to_step(tick_size);
        session.add_volume(price_bin, tick.qty);
        
        session.calculate_metrics(va_pct);
    }

    fn render(&self, ctx: &mut UiContext) {
        if self.sessions.is_empty() { return; }

        let (earliest, latest) = ctx.viewport_range;
        
        // Find sessions that overlap with viewport
        for session in &self.sessions {
            if session.end_time < earliest || session.start_time > latest {
                continue;
            }

            // Calculate session X bounds
            let s_start_x = {
                let diff = session.start_time as f64 - ctx.latest_x as f64;
                (diff / ctx.interval as f64 * ctx.cell_width as f64) as f32
            };
            let s_end_x = {
                let diff = session.end_time as f64 - ctx.latest_x as f64;
                (diff / ctx.interval as f64 * ctx.cell_width as f64) as f32
            };
            
            let session_width = (s_end_x - s_start_x).abs().max(ctx.cell_width);
            let hist_max_width = session_width * (self.width_pct / 100.0);
            
            // Find max volume in session for scaling
            let mut max_vol = 0.0f32;
            for &vol in session.bins.values() {
                max_vol = max_vol.max(vol);
            }
            if max_vol <= 0.0 { continue; }

            let get_y = |price: Price| -> f32 {
                let diff = (price.to_f32() as f64 - ctx.base_price) / ctx.tick_size;
                -(diff as f32 * ctx.cell_height)
            };

            for (&price, &vol) in &session.bins {
                let y = get_y(price);
                
                let bin_width = (vol / max_vol) * hist_max_width;
                let bin_height = ctx.cell_height;
                // Ensure minimal height for visibility
                let bin_height = bin_height.max(1.0);  
                
                let rect_x = if self.placement_right {
                    s_end_x - bin_width
                } else {
                    s_start_x
                };

                let mut color = self.bg_color;
                if let (Some(val), Some(vah)) = (session.val, session.vah) {
                    if price >= val && price <= vah {
                        color = self.va_color;
                    }
                }
                if Some(price) == session.poc {
                    color = self.poc_color;
                }

                ctx.frame.fill_rectangle(
                    Point::new(rect_x, y - bin_height / 2.0),
                    iced::Size::new(bin_width, bin_height),
                    color
                );
            }
            
            // Render Lines (VAH, VAL, POC)
            let line_width = s_end_x - s_start_x;
            if line_width > 0.0 {
                // VAH
                 if let Some(vah) = session.vah {
                    let y = get_y(vah);
                    ctx.frame.fill_rectangle(
                        Point::new(s_start_x, y - 1.0),
                        iced::Size::new(line_width, 2.0),
                        Color::BLACK
                    );
                }
                // VAL
                if let Some(val) = session.val {
                    let y = get_y(val);
                    ctx.frame.fill_rectangle(
                        Point::new(s_start_x, y - 1.0),
                        iced::Size::new(line_width, 2.0),
                        Color::BLACK
                    );
                }
                // POC
                if let Some(poc) = session.poc {
                    let y = get_y(poc);
                    ctx.frame.fill_rectangle(
                        Point::new(s_start_x, y - 1.5),
                        iced::Size::new(line_width, 3.0),
                        Color::from_rgb(1.0, 0.65, 0.0) // Orange
                    );
                }
            }
        }
    }

    fn name(&self) -> &str { &self.name }
    fn get_settings(&mut self) -> &mut Vec<Setting> { &mut self.settings }
    fn settings(&self) -> &[Setting] { &self.settings }
    fn is_overlay(&self) -> bool { true }
    fn reset(&mut self) {
        self.sessions.clear();
        self.last_session_boundary = 0;
    }
    
    fn set_tick_size(&mut self, size: f64) {
        self.tick_size = PriceStep::from_f32(size as f32);
        self.reset();
    }

    fn element<'a>(&'a self, _ctx: &ViewContext) -> iced::Element<'a, crate::chart::Message> {
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
                Setting::Float { name, value, .. } if name == "Value Area %" => {
                    self.value_area_pct = *value;
                }
                Setting::Int { name, value, .. } if name == "Width %" => {
                    self.width_pct = *value as f32;
                }
                Setting::Bool { name, value, .. } if name == "Placement Right" => {
                    self.placement_right = *value;
                }
                Setting::Color { name, value, .. } if name == "POC Color" => {
                    self.poc_color = *value;
                }
                Setting::Color { name, value, .. } if name == "VA Color" => {
                    self.va_color = *value;
                }
                Setting::Color { name, value, .. } if name == "Background Color" => {
                    self.bg_color = *value;
                }
                _ => {}
            }
        }
        if session_changed {
            self.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange::util::Price;

    #[test]
    fn test_session_data_metrics() {
        let mut session = SessionData::new(1000);
        
        // Simulating volume at prices:
        // 100.0: 10
        // 101.0: 50 (POC)
        // 102.0: 20
        // 103.0: 5
        // Total: 85
        // VA (70%): 59.5
        
        session.add_volume(Price::from_f32(100.0), 10.0);
        session.add_volume(Price::from_f32(101.0), 50.0);
        session.add_volume(Price::from_f32(102.0), 20.0);
        session.add_volume(Price::from_f32(103.0), 5.0);
        
        // Calculate metrics
        session.calculate_metrics(70.0);
        
        // Assert POC
        assert_eq!(session.poc, Some(Price::from_f32(101.0)));
        
        // Assert Value Area
        // POC (50) is < 59.5. 
        // Next highest neighbors: 102 (20) vs 100 (10). Should pick 102.
        // Current vol: 50 + 20 = 70. > 59.5. Done.
        // Bounds should be [101, 102] OR [101, 102] depending on inclusion. 
        // Actually, logic starts at POC and expands.
        // L=P, R=P. 
        // Check L-1 (100, vol 10) vs R+1 (102, vol 20). 20 > 10.
        // Expand R to 102. Vol = 50 + 20 = 70.
        // 70 >= 59.5 target. Stop.
        // VAL = 101.0, VAH = 102.0.
        
        assert_eq!(session.val, Some(Price::from_f32(101.0)));
        assert_eq!(session.vah, Some(Price::from_f32(102.0)));
    }
}
