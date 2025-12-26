use std::collections::HashMap;

use data::layout::WindowSpec;
use iced::{Point, Size, Subscription, Task, window};

pub use iced::window::{Id, Position, Settings, close, open};
use iced_futures::MaybeSend;

/// ساختار نگهدارنده اطلاعات یک پنجره
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: Id,                 // شناسه منحصر به فرد پنجره
    pub position: Option<Point>, // موقعیت پنجره در صفحه
}

impl Window {
    /// ایجاد یک نمونه جدید از پنجره با شناسه مشخص
    pub fn new(id: Id) -> Self {
        Self { id, position: None }
    }
}

/// دریافت اندازه پیش‌فرض پنجره
pub fn default_size() -> Size {
    WindowSpec::default().size()
}

/// رویدادهای مربوط به پنجره
#[derive(Debug, Clone, Copy)]
pub enum Event {
    CloseRequested(window::Id), // درخواست بستن پنجره
}

/// گوش دادن به رویدادهای پنجره
pub fn events() -> Subscription<Event> {
    iced::event::listen_with(filtered_events)
}

/// فیلتر کردن رویدادهای خام سیستم و تبدیل به رویدادهای پنجره برنامه
fn filtered_events(
    event: iced::Event,
    _status: iced::event::Status,
    window: window::Id,
) -> Option<Event> {
    match &event {
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            Some(Event::CloseRequested(window))
        }
        _ => None,
    }
}

/// جمع‌آوری مشخصات (موقعیت و اندازه) تمام پنجره‌های باز
pub fn collect_window_specs<M, F>(window_ids: Vec<window::Id>, message: F) -> Task<M>
where
    F: Fn(HashMap<window::Id, WindowSpec>) -> M + Send + 'static,
    M: MaybeSend + 'static,
{
    // ایجاد لیستی از تسک‌ها برای دریافت مشخصات هر پنجره
    let window_spec_tasks = window_ids
        .into_iter()
        .map(|window_id| {
            // دریافت موقعیت پنجره
            let pos_task: Task<(Option<Point>, Option<Size>)> =
                iced::window::position(window_id).map(|pos| (pos, None));

            // دریافت اندازه پنجره
            let size_task: Task<(Option<Point>, Option<Size>)> =
                iced::window::size(window_id).map(|size| (None, Some(size)));

            // اجرای همزمان هر دو تسک و ترکیب نتایج
            Task::batch(vec![pos_task, size_task])
                .collect()
                .map(move |results| {
                    let position = results.iter().find_map(|(pos, _)| *pos);
                    let size = results
                        .iter()
                        .find_map(|(_, size)| *size)
                        .unwrap_or_else(|| Size::new(1024.0, 768.0));

                    (window_id, (position, size))
                })
        })
        .collect::<Vec<_>>();

    // اجرای تمام تسک‌های پنجره‌ها و تبدیل نتایج نهایی به مشخصات پنجره (WindowSpec)
    Task::batch(window_spec_tasks)
        .collect()
        .map(move |results| {
            let specs: HashMap<window::Id, WindowSpec> = results
                .into_iter()
                .filter_map(|(id, (pos, size))| {
                    pos.map(|position| (id, WindowSpec::from((&position, &size))))
                })
                .collect();

            message(specs)
        })
}

#[cfg(target_os = "linux")]
/// تنظیمات پنجره برای سیستم‌عامل لینوکس
pub fn settings() -> Settings {
    Settings {
        min_size: Some(Size::new(800.0, 600.0)),
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
/// تنظیمات پنجره برای سیستم‌عامل مک (macOS)
pub fn settings() -> Settings {
    use iced::window;

    Settings {
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        min_size: Some(Size::new(800.0, 600.0)),
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
/// تنظیمات پنجره برای سیستم‌عامل ویندوز
pub fn settings() -> Settings {
    Settings {
        min_size: Some(Size::new(800.0, 600.0)),
        ..Default::default()
    }
}
