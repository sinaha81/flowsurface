use super::{Message, Action, Panel};
use crate::style;
use exchange::{Trade, Kline, TickerInfo};
use exchange::util::{Price, PriceStep};

use iced::widget::canvas::{self, Text};
use iced::{Alignment, Event, Point, Rectangle, Renderer, Theme, mouse, Color};

use std::collections::BTreeMap;
use std::time::Instant;

pub use data::panel::volume_profile::Config;

#[derive(Debug, Clone, Default)]
pub struct SessionProfile {
    pub bins: BTreeMap<Price, f32>,
    pub total_volume: f32,
    pub poc: Option<Price>,
    pub vah: Option<Price>,
    pub val: Option<Price>,
    pub max_vol: f32,
    pub dirty: bool,
}

impl SessionProfile {
    pub fn update_with_kline(&mut self, kline: &Kline, tick_size: PriceStep) {
        let low_bin = kline.low.round_to_step(tick_size);
        let high_bin = kline.high.round_to_step(tick_size);
        let steps = Price::steps_between_inclusive(low_bin, high_bin, tick_size).unwrap_or(1);
        let vol_per_step = (kline.volume.0 + kline.volume.1) / steps as f32;
        
        let mut curr = low_bin;
        for _ in 0..steps {
            *self.bins.entry(curr).or_insert(0.0) += vol_per_step;
            self.total_volume += vol_per_step;
            curr = curr.add_steps(1, tick_size);
        }
        self.dirty = true;
    }

    pub fn update_with_trade(&mut self, trade: &Trade, tick_size: PriceStep) {
        let price_bin = trade.price.round_to_step(tick_size);
        *self.bins.entry(price_bin).or_insert(0.0) += trade.qty;
        self.total_volume += trade.qty;
        self.dirty = true;
    }

    pub fn calculate_metrics(&mut self, value_area_pct: f32) {
        if !self.dirty || self.bins.is_empty() { return; }

        let mut max_vol = 0.0;
        let mut poc = None;
        for (&price, &vol) in &self.bins {
            if vol > max_vol {
                max_vol = vol;
                poc = Some(price);
            }
        }
        self.poc = poc;
        self.max_vol = max_vol;

        if let Some(poc_price) = poc {
            let target_vol = self.total_volume * (value_area_pct / 100.0);
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
                } else {
                    break;
                }
            }
            
            self.val = Some(prices[l_ptr]);
            self.vah = Some(prices[r_ptr]);
        }
        self.dirty = false;
    }
}

pub struct VolumeProfile {
    ticker_info: TickerInfo,
    pub config: Config,
    cache: canvas::Cache,
    last_tick: Instant,
    tick_size: PriceStep,
    scroll_px: f32,
    
    sessions: BTreeMap<u64, SessionProfile>,
}

impl super::Panel for VolumeProfile {
    fn scroll(&mut self, delta: f32) {
        self.scroll_px += delta;
        self.invalidate(Some(Instant::now()));
    }

    fn reset_scroll(&mut self) {
        self.scroll_px = 0.0;
        self.invalidate(Some(Instant::now()));
    }

    fn invalidate(&mut self, now: Option<Instant>) -> Option<Action> {
        self.cache.clear();
        if let Some(now) = now {
            self.last_tick = now;
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

impl VolumeProfile {
    pub fn new(config: Option<Config>, ticker_info: TickerInfo, tick_size: f32) -> Self {
        Self {
            ticker_info,
            config: config.unwrap_or_else(|| Config {
                row_height: 14.0, // Default to a more reasonable value
                ..Default::default()
            }),
            cache: canvas::Cache::default(),
            last_tick: Instant::now(),
            tick_size: PriceStep::from_f32(tick_size),
            scroll_px: 0.0,
            sessions: BTreeMap::new(),
        }
    }

    pub fn update_data(&mut self, trades: &[Trade], klines: &[Kline]) {
        let timeframe = self.config.period.to_timeframe();
        
        // If Period is VisibleRange, we clear and rebuild every time we get history
        // Actually, for simplicity, we just clear everything if we get multiple klines
        if klines.len() > 10 {
            self.sessions.clear();
        }

        for kline in klines {
            let session_start = timeframe.map(|tf| tf.start_of_interval(kline.time)).unwrap_or(0);
            let session = self.sessions.entry(session_start).or_default();
            session.update_with_kline(kline, self.tick_size);
        }

        for trade in trades {
            let session_start = timeframe.map(|tf| tf.start_of_interval(trade.time)).unwrap_or(0);
            let session = self.sessions.entry(session_start).or_default();
            session.update_with_trade(trade, self.tick_size);
        }

        for session in self.sessions.values_mut() {
            session.calculate_metrics(self.config.value_area_pct);
        }

        self.invalidate(Some(Instant::now()));
    }

    pub fn last_update(&self) -> Instant {
        self.last_tick
    }
}

impl canvas::Program<Message> for VolumeProfile {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let scroll_amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -(*y) * 20.0,
                    mouse::ScrollDelta::Pixels { y, .. } => -*y,
                };
                Some(canvas::Action::publish(Message::Scrolled(scroll_amount)).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                Some(canvas::Action::publish(Message::ResetScroll).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.extended_palette();
        let bg_color = Color { a: 0.1, ..palette.background.weak.text };
        let va_color = Color { a: 0.3, ..palette.primary.base.color };
        let poc_color = Color::from_rgb(1.0, 0.65, 0.0);
        let text_color = palette.background.base.text;

        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            if self.sessions.is_empty() { return; }

            // Take latest N sessions that fit in width
            let num_sessions = self.sessions.len();
            let session_width = (bounds.width / (num_sessions as f32).max(1.0)).min(bounds.width * 0.5);
            let row_height = self.config.row_height;
            let mid_y = bounds.height / 2.0 + self.scroll_px;

            // Anchor on the latest session's POC
            let latest_session = self.sessions.values().next_back().unwrap();
            let anchor_price = latest_session.poc.unwrap_or(latest_session.bins.keys().next().cloned().unwrap_or(Price { units: 0 }));

            for (i, (&_ts, session)) in self.sessions.iter().rev().enumerate() {
                let x_offset = bounds.width - (i as f32 + 1.0) * session_width;
                if x_offset + session_width < 0.0 { break; }

                for (&price, &vol) in &session.bins {
                    let diff_units = price.units - anchor_price.units;
                    let diff_steps = diff_units / self.tick_size.units;
                    let y = mid_y - (diff_steps as f32 * row_height);

                    if y < -row_height || y > bounds.height + row_height {
                        continue;
                    }

                    let bar_width = (vol / session.max_vol) * session_width * 0.9;
                    let mut color = bg_color;

                    if let (Some(val), Some(vah)) = (session.val, session.vah) {
                        if price >= val && price <= vah {
                            color = va_color;
                        }
                    }
                    if Some(price) == session.poc {
                        color = poc_color;
                    }

                    frame.fill_rectangle(
                        Point::new(x_offset, y - row_height / 2.0 + 1.0),
                        iced::Size::new(bar_width, row_height - 2.0),
                        color
                    );

                    // Price text (only for the latest/rightmost session)
                    if i == 0 {
                        let price_str = price.to_string(self.ticker_info.min_ticksize);
                        frame.fill_text(Text {
                            content: price_str,
                            position: Point::new(bounds.width - 5.0, y),
                            color: text_color,
                            size: 11.0.into(),
                            font: style::AZERET_MONO,
                            align_x: Alignment::End.into(),
                            align_y: Alignment::Center.into(),
                            ..Default::default()
                        });
                    }
                }
            }
        });

        vec![geometry]
    }
}
