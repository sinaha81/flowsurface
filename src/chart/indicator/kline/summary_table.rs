use crate::chart::{Message, ViewState};
use crate::chart::indicator::kline::KlineIndicatorImpl;
use iced::Element;

pub struct SummaryTableIndicator;

impl SummaryTableIndicator {
    pub fn new() -> Self {
        Self
    }
}

impl KlineIndicatorImpl for SummaryTableIndicator {
    fn clear_all_caches(&mut self) {}
    fn clear_crosshair_caches(&mut self) {}

    fn element<'a>(
        &'a self,
        _chart: &'a ViewState,
        _visible_range: std::ops::RangeInclusive<u64>,
    ) -> Element<'a, Message> {
        // SummaryTable is rendered directly in KlineChart's draw call,
        // but needs to exist as an indicator to be enabled/disabled.
        iced::widget::column![].into()
    }
}
