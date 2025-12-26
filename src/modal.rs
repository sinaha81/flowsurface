// ماژول‌های مربوط به انواع مختلف مودال‌ها (پنجره‌های شناور)
pub mod audio;
pub mod layout_manager;
pub mod pane;
pub mod theme_editor;

use iced::widget::{center, container, mouse_area, opaque, stack};
use iced::{Alignment, Color, Element, Length, padding};
pub use layout_manager::LayoutManager;
pub use pane::indicators;
pub use pane::stream::{self, ModifierKind};
pub use theme_editor::ThemeEditor;

/// ایجاد یک مودال دیالوگ اصلی که کل صفحه را می‌پوشاند
pub fn main_dialog_modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,    // عنصر پایه (پس‌زمینه)
    content: impl Into<Element<'a, Message>>, // محتوای مودال
    on_blur: Message,                         // پیامی که هنگام کلیک روی پس‌زمینه ارسال می‌شود
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.8, // شفافیت پس‌زمینه تیره
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}

/// ایجاد یک مودال مخصوص داشبورد با قابلیت تنظیم تراز و فاصله
pub fn dashboard_modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,    // عنصر پایه
    content: impl Into<Element<'a, Message>>, // محتوای مودال
    on_blur: Message,                         // پیامی که هنگام کلیک روی پس‌زمینه ارسال می‌شود
    padding: padding::Padding,                // فاصله داخلی
    align_y: Alignment,                       // تراز عمودی
    align_x: Alignment,                       // تراز افقی
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        mouse_area(
            container(opaque(content))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding)
                .align_y(align_y)
                .align_x(align_x)
        )
        .on_press(on_blur)
    ]
    .into()
}
