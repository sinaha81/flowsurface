use crate::screen::dashboard::pane::{Event, Message as PaneMessage};
use crate::style;
use iced::widget::{checkbox, column, container, pick_list, text};
use iced::{Element, Length};
use crate::indicators::Indicator;
use crate::indicators::Setting;
use data::config::theme::{from_hsva, to_hsva};

pub fn view<'a>(
    pane: iced::widget::pane_grid::Pane,
    indicator: &'a dyn Indicator,
    indicator_idx: usize,
) -> Element<'a, PaneMessage> {
    
    let settings_ui = indicator.settings().iter().enumerate().map(|(idx, setting)| {
        match setting {
            Setting::Bool { name, value, description } => {
                let cb = checkbox(*value)
                    .label(name.as_str())
                    .on_toggle(move |new_val| {
                        // We need a way to propagate this change back. 
                        // Since we have a mutable reference, we can theoretically change it here if we were in the update loop.
                        // But we are in the view loop. We cannot mutate here.
                        // The architectural pattern used in this app typically involves sending a message.
                        // However, `get_settings` returns `&mut Vec<Setting>`. 
                        // We can't use that in `view`.
                        
                        // Wait, the standard Iced pattern: view production produces Messages.
                        // We need a specific message to update a modular indicator setting.
                        // Let's go back and add `UpdateIndicatorSetting(usize, usize, SettingValue)` to Event.
                        // For now, let's placeholder.
                        
                        // Actually, looking at how `settings.rs` works, it seems they often use a temporary State or direct mutation via Message.
                        // We need a `IndicatorSettingChanged(usize, usize, Setting)` message.
                        // The index of the indicator, the index of the setting, and the new value.
                        
                        PaneMessage::PaneEvent(pane, Event::IndicatorSettingChanged(indicator_idx, idx, Setting::Bool { name: name.clone(), value: new_val, description: description.clone() }))
                    });
                
                if let Some(desc) = description {
                    column![cb, text(desc).size(12)].spacing(2).into()
                } else {
                    cb.into()
                }
            }
            Setting::Int { name, value, min, max, step, description } => {
                let s = crate::widget::labeled_slider(
                    name.as_str(),
                    *min..=*max,
                    *value,
                    move |new_val| {
                        PaneMessage::PaneEvent(pane, Event::IndicatorSettingChanged(indicator_idx, idx, Setting::Int { 
                            name: name.clone(), 
                            value: new_val, 
                            min: *min, max: *max, step: *step, 
                            description: description.clone() 
                        }))
                    },
                    |v| format!("{}", v),
                    Some(*step)
                );
                 
                if let Some(desc) = description {
                    column![s, text(desc).size(12)].spacing(2).into()
                } else {
                    s.into()
                }
            }
            Setting::Float { name, value, min, max, step, description } => {
                let s = crate::widget::labeled_slider(
                    name.as_str(),
                    *min..=*max,
                    *value,
                    move |new_val| {
                        PaneMessage::PaneEvent(pane, Event::IndicatorSettingChanged(indicator_idx, idx, Setting::Float { 
                            name: name.clone(), 
                            value: new_val, 
                            min: *min, max: *max, step: *step, 
                            description: description.clone() 
                        }))
                    },
                    |v| format!("{:.2}", v),
                    Some(*step)
                );
                 
                if let Some(desc) = description {
                    column![s, text(desc).size(12)].spacing(2).into()
                } else {
                    s.into()
                }
            }
             Setting::Enum { name, value, options, description } => {
                let pick = pick_list(options.clone(), Some(value.clone()), move |new_val| {
                    PaneMessage::PaneEvent(pane, Event::IndicatorSettingChanged(indicator_idx, idx, Setting::Enum { 
                        name: name.clone(), 
                        value: new_val, 
                        options: options.clone(),
                        description: description.clone() 
                    }))
                });
                
                if let Some(desc) = description {
                    column![text(name.as_str()), pick, text(desc).size(12)].spacing(4).into()
                } else {
                     column![
                        text(name.as_str()),
                        pick
                    ].spacing(4).into()
                }
            }
            Setting::Color { name, value, description } => {
                let hsva = to_hsva(*value);
                
                let picker = crate::widget::color_picker::color_picker(hsva, move |new_hsva| {
                    let new_color = from_hsva(new_hsva);
                    PaneMessage::PaneEvent(pane, Event::IndicatorSettingChanged(indicator_idx, idx, Setting::Color { 
                        name: name.clone(), 
                        value: new_color, 
                        description: description.clone() 
                    }))
                });

                if let Some(desc) = description {
                    column![text(name.as_str()), picker, text(desc).size(12)].spacing(4).into()
                } else {
                    column![text(name.as_str()), picker].spacing(4).into()
                }
            }
        }
    }).collect::<Vec<_>>();

    let content = column(settings_ui).spacing(12);

    container(crate::widget::scrollable_content(content))
        .width(Length::Shrink)
        .padding(28)
        .max_width(360)
        .style(style::chart_modal)
        .into()
}
