use super::Element;
use crate::{
    screen::ConfirmDialog,
    style::{self, Icon, icon_text, modal_container},
};
use iced::{
    Alignment::{self, Center},
    Color,
    Length::Fill,
    Theme, border, padding,
    widget::{button, column, container, row, scrollable, slider, space, text, tooltip::Position},
};

// ماژول‌های مربوط به ویجت‌های اختصاصی برنامه
pub mod chart;
pub mod color_picker;
pub mod column_drag;
pub mod decorate;
pub mod multi_split;
pub mod toast;

#[allow(dead_code)]
pub const DEFAULT_TOOLTIP_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// ایجاد یک تولتیپ (Tooltip) ساده برای یک عنصر
pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>, // عنصر اصلی
    tooltip: Option<&'a str>,                 // متن تولتیپ
    position: Position,                       // موقعیت نمایش
) -> Element<'a, Message> {
    tooltip_with_delay(content, tooltip, position, std::time::Duration::ZERO)
}

/// ایجاد یک تولتیپ با تاخیر زمانی در نمایش
pub fn tooltip_with_delay<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: Option<&'a str>,
    position: Position,
    delay: std::time::Duration,
) -> Element<'a, Message> {
    match tooltip {
        Some(tooltip) => iced::widget::tooltip(
            content,
            container(text(tooltip)).style(style::tooltip).padding(8),
            position,
        )
        .delay(delay)
        .into(),
        None => content.into(),
    }
}

/// ایجاد یک محتوای قابل اسکرول (عمودی)
pub fn scrollable_content<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    scrollable::Scrollable::with_direction(
        content,
        scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    )
    .into()
}

/// ایجاد کانتینر برای دیالوگ تایید (Confirm Dialog)
pub fn confirm_dialog_container<'a, Message: 'a + Clone>(
    confirm_dialog: ConfirmDialog<Message>,
    on_cancel: Message,
) -> Element<'a, Message> {
    let dialog = confirm_dialog.message;
    let on_confirm = *confirm_dialog.on_confirm;
    let on_confirm_msg = confirm_dialog.on_confirm_btn_text;

    container(
        column![
            text(dialog).size(14),
            row![
                button(text("Cancel"))
                    .style(|theme, status| style::button::transparent(theme, status, false))
                    .on_press(on_cancel),
                button(text(on_confirm_msg.unwrap_or("Confirm".to_string()))).on_press(on_confirm),
            ]
            .spacing(8),
        ]
        .align_x(Alignment::Center)
        .spacing(16),
    )
    .padding(24)
    .style(style::dashboard_modal)
    .into()
}

/// ایجاد یک ردیف شامل لیبل و اسلایدر کلاسیک
pub fn classic_slider_row<'a, Message>(
    label: iced::widget::Text<'a>,
    slider: Element<'a, Message>,
    placeholder: Option<iced::widget::Text<'a>>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let slider = if let Some(placeholder) = placeholder {
        column![slider, placeholder]
            .spacing(2)
            .align_x(Alignment::Center)
    } else {
        column![slider]
    };

    container(
        row![label, slider]
            .align_y(Alignment::Center)
            .spacing(8)
            .padding(8),
    )
    .style(style::modal_container)
    .into()
}

/// ایجاد یک دکمه به همراه تولتیپ
pub fn button_with_tooltip<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    message: M,
    tooltip_text: Option<&'a str>,
    tooltip_pos: crate::TooltipPosition,
    style_fn: impl Fn(&Theme, button::Status) -> button::Style + 'static,
) -> Element<'a, M> {
    let btn = button(content).style(style_fn).on_press(message);

    if let Some(text) = tooltip_text {
        tooltip(btn, Some(text), tooltip_pos)
    } else {
        btn.into()
    }
}

/// ایجاد یک ردیف با قابلیت جابجایی (Drag)
pub fn dragger_row<'a, Message>(
    content: Element<'a, Message>, // محتوای ردیف
    is_enabled: bool,              // آیا قابلیت جابجایی فعال است؟
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let content = if is_enabled {
        let icon = icon_text(Icon::DragHandle, 11);
        row![icon, content,]
            .align_y(Alignment::Center)
            .spacing(2)
            .into()
    } else {
        content
    };

    container(content)
        .padding(2)
        .style(style::dragger_row_container)
        .into()
}

/// ایجاد یک اسلایدر به همراه لیبل و مقدار فعلی
pub fn labeled_slider<'a, T, Message: Clone + 'static>(
    label: impl text::IntoFragment<'a>,           // لیبل اسلایدر
    range: std::ops::RangeInclusive<T>,           // محدوده مقادیر
    current: T,                                   // مقدار فعلی
    on_change: impl Fn(T) -> Message + 'a,        // تابع فراخوانی هنگام تغییر
    to_string: impl Fn(&T) -> String,             // تابع تبدیل مقدار به متن
    step: Option<T>,                              // گام‌های تغییر
) -> Element<'a, Message>
where
    T: 'static + Copy + PartialOrd + Into<f64> + From<u8> + num_traits::FromPrimitive,
{
    let mut slider = iced::widget::slider(range, current, on_change)
        .width(Fill)
        .height(24)
        .style(|theme: &Theme, status| {
            let palette = theme.extended_palette();

            slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        palette.background.strong.color.into(),
                        Color::TRANSPARENT.into(),
                    ),
                    width: 24.0,
                    border: border::rounded(2),
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Rectangle {
                        width: 2,
                        border_radius: 2.0.into(),
                    },
                    background: match status {
                        iced::widget::slider::Status::Active => {
                            palette.background.strong.color.into()
                        }
                        iced::widget::slider::Status::Hovered => palette.primary.base.color.into(),
                        iced::widget::slider::Status::Dragged => palette.primary.weak.color.into(),
                    },
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            }
        });

    if let Some(v) = step {
        slider = slider.step(v);
    }

    iced::widget::stack![
        container(slider).style(modal_container),
        row![text(label), space::horizontal(), text(to_string(&current))]
            .padding([0, 10])
            .height(Fill)
            .align_y(Center),
    ]
    .into()
}

/// ایجاد یک جعبه ورودی عددی به همراه لیبل
pub fn numeric_input_box<'a, F, Message: Clone + 'static>(
    label: &'a str,               // لیبل ورودی
    placeholder: &str,           // متن راهنما (Placeholder)
    raw_input_buf: &str,         // بافر ورودی خام
    is_input_valid: bool,        // آیا ورودی معتبر است؟
    on_input_changed: F,         // تابع فراخوانی هنگام تغییر ورودی
    on_submit_maybe: Option<Message>, // پیام ارسالی هنگام تایید (اختیاری)
) -> Element<'a, Message>
where
    F: Fn(String) -> Message + 'static,
{
    let text_input_widget = iced::widget::text_input(placeholder, raw_input_buf)
        .on_input(on_input_changed)
        .on_submit_maybe(on_submit_maybe)
        .align_x(iced::Alignment::Center)
        .style(move |theme, status| style::validated_text_input(theme, status, is_input_valid));

    row![text(label), text_input_widget]
        .padding(padding::right(20).left(20))
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
}

/// ایجاد دکمه مربوط به گروه‌بندی پیوندها (Link Group)
pub fn link_group_button<'a, Message, F>(
    id: iced::widget::pane_grid::Pane,
    link_group: Option<data::layout::pane::LinkGroup>,
    on_press: F,
) -> Element<'a, Message>
where
    Message: Clone + 'static,
    F: Fn(iced::widget::pane_grid::Pane) -> Message + 'static,
{
    let is_active = link_group.is_some();

    let icon = if let Some(group) = link_group {
        text(group.to_string())
            .font(style::AZERET_MONO)
            .align_x(Alignment::Start)
            .align_y(Alignment::Center)
    } else {
        text("-")
            .font(style::AZERET_MONO)
            .align_x(Alignment::Start)
            .align_y(Alignment::Center)
    };

    button(icon)
        .style(move |theme: &Theme, status| {
            style::button::bordered_toggle(theme, status, is_active)
        })
        .on_press(on_press(id))
        .width(28)
        .into()
}

#[macro_export]
/// ایجاد یک ستون که بین هر آیتم آن یک خط جداکننده افقی قرار می‌گیرد
///
/// # مثال
/// ```
/// split_column![
///     text("Item 1"),
///     text("Item 2"),
///     text("Item 3"),
/// ] ; spacing = 8, align_x = Alignment::Start
/// ```
///
macro_rules! split_column {
    () => {
        column![]
    };

    ($item:expr $(,)?) => {
        column![$item]
    };

    ($first:expr, $($rest:expr),+ $(,)?) => {{
        let mut col = column![$first];
        $(
            col = col.push(iced::widget::rule::horizontal(1.0).style($crate::style::split_ruler));
            col = col.push($rest);
        )+
        col
    }};

    ($($item:expr),* $(,)?; spacing = $spacing:expr) => {{
        $crate::split_column![$($item),*].spacing($spacing)
    }};

    ($($item:expr),* $(,)?; spacing = $spacing:expr, align_x = $align:expr) => {{
        $crate::split_column![$($item),*].spacing($spacing).align_x($align)
    }};
}
