use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Element, Length, Color, Alignment, 
};
use iced::widget::Id;

use crate::{
    logger::{LogEntry, LOG_BUFFER},
};



#[derive(Debug, Clone)]
pub enum Message {
    FilterChanged(String),
    Clear,
    Copy(String),
}

pub struct Console {
    logs: Vec<LogEntry>,
    filter: String,
    auto_scroll: bool,
    scroll_id: Id,
}

impl Console {
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            filter: String::new(),
            auto_scroll: true,
            scroll_id: Id::unique(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::FilterChanged(f) => self.filter = f,
            Message::Clear => {
                // We cannot clear the global buffer, but we can clear our local view?
                // Actually, let's just ignore for now or maybe clear global buffer?
                // Better to clear global but it's behind a lock.
                if let Ok(mut buffer) = LOG_BUFFER.write() {
                    buffer.clear();
                }
            }
            Message::Copy(_text) => {
                 // The string is passed by the view (constructed there) or constructed here
                 // But message needs to carry it for the main loop to handle
            }
        }
    }
    
    pub fn sync_logs(&mut self) {
        if let Ok(buffer) = LOG_BUFFER.read() {
            // Simple sync: just clone the whole buffer for now. 
            // Optimization: check if last entry timestamp/content matches to avoid clone
            // But VecDeque clone is fast enough for 1000 items.
            self.logs = buffer.iter().cloned().collect();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let filtered_logs: Vec<&LogEntry> = if self.filter.is_empty() {
             self.logs.iter().collect()
        } else {
             self.logs.iter().filter(|l| 
                 l.message.to_lowercase().contains(&self.filter.to_lowercase()) || 
                 l.level.to_string().to_lowercase().contains(&self.filter.to_lowercase())
             ).collect()
        };

        let content = column(
            filtered_logs.iter().map(|log| {
                let color = match log.level {
                    log::Level::Error => Color::from_rgb8(255, 0, 0), // Danger
                    log::Level::Warn => Color::from_rgb8(255, 165, 0), // Warning
                    log::Level::Info => Color::from_rgb8(0, 191, 255), // Info
                    log::Level::Debug => Color::from_rgb8(128, 128, 128), // Primary/Grey
                    log::Level::Trace => Color::from_rgb8(169, 169, 169), // Secondary
                };
                
                text(format!("{} [{}] {}", log.timestamp, log.level, log.message))
                    .size(12)
                    .color(color)
                    .font(iced::Font::MONOSPACE)
                    .into()
            })
        ).spacing(2);

        let scroll = scrollable(content)
            .id(self.scroll_id.clone())
            .height(Length::Fill)
            .width(Length::Fill);

        let controls = row![
            text("Console").size(14).font(iced::Font::MONOSPACE),
            text_input("Filter...", &self.filter)
                .on_input(Message::FilterChanged)
                .width(Length::Fixed(200.0))
                .padding(4),
            button("Clear")
                .on_press(Message::Clear)
                .style(button::text),
            button("Copy")
                .on_press(Message::Copy(self.logs.iter().map(|l| format!("{} [{}] {}", l.timestamp, l.level, l.message)).collect::<Vec<_>>().join("\n")))
                .style(button::text),
        ]
        .spacing(10)
        .padding(4)
        .align_y(Alignment::Center);

        column![
            controls,
            container(scroll)
                .style(iced::widget::container::bordered_box)
                .padding(4)
                .height(Length::Fill),
        ]
        .spacing(4)
        .into()
    }
}
