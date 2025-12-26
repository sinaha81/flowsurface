// غیرفعال کردن کنسول در ویندوز برای نسخه‌های ریلیز
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// تعریف ماژول‌های مختلف پروژه
mod audio;   // مدیریت صدا
mod chart;   // مدیریت نمودارها
mod layout;  // مدیریت چیدمان صفحات
mod logger;  // سیستم ثبت وقایع (Log)
mod modal;   // مدیریت پنجره‌های مودال (پاپ‌آپ)
mod screen;  // مدیریت صفحات مختلف برنامه
mod style;   // تعریف استایل‌ها و تم‌های ظاهری
mod widget;  // ویجت‌های سفارشی رابط کاربری
mod window;  // مدیریت پنجره‌های سیستم‌عامل

use data::config::theme::default_theme;
use data::{layout::WindowSpec, sidebar};
use layout::{LayoutId, configuration};
use modal::{LayoutManager, ThemeEditor, audio::AudioStream};
use modal::{dashboard_modal, main_dialog_modal};
use screen::dashboard::{self, Dashboard};
use widget::{
    confirm_dialog_container,
    toast::{self, Toast},
    tooltip,
};

use iced::{
    Alignment, Element, Subscription, Task, keyboard, padding,
    widget::{
        button, column, container, pane_grid, pick_list, row, rule, scrollable, text,
        tooltip::Position as TooltipPosition,
    },
};
use std::{borrow::Cow, collections::HashMap, vec};

/// نقطه شروع برنامه
fn main() {
    // راه‌اندازی سیستم لاگ
    logger::setup(cfg!(debug_assertions)).expect("Failed to initialize logger");

    // اجرای ترد برای پاکسازی داده‌های قدیمی بازار در پس‌زمینه
    std::thread::spawn(data::cleanup_old_market_data);

    // اجرای برنامه اصلی با استفاده از کتابخانه Iced
    let _ = iced::daemon(Flowsurface::new, Flowsurface::update, Flowsurface::view)
        .settings(iced::Settings {
            antialiasing: true, // فعال‌سازی لبه‌های نرم
            fonts: vec![
                Cow::Borrowed(style::AZERET_MONO_BYTES),
                Cow::Borrowed(style::ICONS_BYTES),
            ],
            default_text_size: iced::Pixels(12.0),
            ..Default::default()
        })
        .title(Flowsurface::title)
        .theme(Flowsurface::theme)
        .scale_factor(Flowsurface::scale_factor)
        .subscription(Flowsurface::subscription)
        .run();
}

/// ساختار اصلی برنامه Flowsurface که وضعیت کل برنامه را نگه می‌دارد
struct Flowsurface {
    main_window: window::Window,          // پنجره اصلی برنامه
    sidebar: dashboard::Sidebar,          // نوار کناری (سایدبار)
    layout_manager: LayoutManager,        // مدیریت چیدمان‌ها
    theme_editor: ThemeEditor,            // ویرایشگر تم
    audio_stream: AudioStream,            // مدیریت پخش صدا
    confirm_dialog: Option<screen::ConfirmDialog<Message>>, // دیالوگ تایید عملیات
    volume_size_unit: exchange::SizeUnit, // واحد نمایش حجم معاملات
    ui_scale_factor: data::ScaleFactor,   // ضریب بزرگنمایی رابط کاربری
    timezone: data::UserTimezone,         // منطقه زمانی کاربر
    theme: data::Theme,                   // تم فعلی برنامه
    notifications: Vec<Toast>,            // لیست اعلان‌ها (Toasts)
}

/// پیام‌های مختلف که در برنامه جابجا می‌شوند و باعث تغییر وضعیت می‌شوند
#[derive(Debug, Clone)]
enum Message {
    Sidebar(dashboard::sidebar::Message), // پیام‌های مربوط به سایدبار
    MarketWsEvent(exchange::Event),       // رویدادهای دریافت شده از وب‌ساکت بازار
    Dashboard {
        /// اگر `None` باشد، از چیدمان فعال استفاده می‌شود
        layout_id: Option<uuid::Uuid>,
        event: dashboard::Message,
    },
    Tick(std::time::Instant),             // پیام تیک زمان (برای به‌روزرسانی‌های دوره‌ای)
    WindowEvent(window::Event),           // رویدادهای مربوط به پنجره‌ها (مانند بستن)
    ExitRequested(HashMap<window::Id, WindowSpec>),    // درخواست خروج از برنامه
    RestartRequested(HashMap<window::Id, WindowSpec>), // درخواست راه‌اندازی مجدد
    GoBack,                               // بازگشت به مرحله قبل (مثلاً با دکمه Esc)
    DataFolderRequested,                  // درخواست باز کردن پوشه داده‌ها
    ThemeSelected(data::Theme),           // انتخاب تم جدید
    ScaleFactorChanged(data::ScaleFactor),// تغییر ضریب بزرگنمایی
    SetTimezone(data::UserTimezone),      // تنظیم منطقه زمانی
    ToggleTradeFetch(bool),               // فعال/غیرفعال کردن دریافت معاملات
    ApplyVolumeSizeUnit(exchange::SizeUnit), // اعمال واحد حجم جدید
    RemoveNotification(usize),            // حذف یک اعلان
    // نمایش یا پنهان کردن دیالوگ تایید
    ToggleDialogModal(Option<screen::ConfirmDialog<Message>>),
    ThemeEditor(modal::theme_editor::Message), // پیام‌های ویرایشگر تم
    Layouts(modal::layout_manager::Message),   // پیام‌های مدیریت چیدمان
    AudioStream(modal::audio::Message),        // پیام‌های مربوط به صدا
}

impl Flowsurface {
    /// ایجاد یک نمونه جدید از برنامه و بارگذاری وضعیت ذخیره شده
    fn new() -> (Self, Task<Message>) {
        // بارگذاری وضعیت ذخیره شده از دیسک
        let saved_state = layout::load_saved_state();

        // تنظیمات پنجره اصلی
        let (main_window_id, open_main_window) = {
            let (position, size) = saved_state.window();
            let config = window::Settings {
                size,
                position,
                exit_on_close_request: false,
                ..window::settings()
            };
            window::open(config)
        };

        // ایجاد سایدبار
        let (sidebar, launch_sidebar) = dashboard::Sidebar::new(&saved_state);

        // مقداردهی اولیه وضعیت برنامه
        let mut state = Self {
            main_window: window::Window::new(main_window_id),
            layout_manager: saved_state.layout_manager,
            theme_editor: ThemeEditor::new(saved_state.custom_theme),
            audio_stream: AudioStream::new(saved_state.audio_cfg),
            sidebar,
            confirm_dialog: None,
            timezone: saved_state.timezone,
            ui_scale_factor: saved_state.scale_factor,
            volume_size_unit: saved_state.volume_size_unit,
            theme: saved_state.theme,
            notifications: vec![],
        };

        // تعیین چیدمان فعال
        let active_layout_id = state.layout_manager.active_layout_id().unwrap_or(
            &state
                .layout_manager
                .layouts
                .first()
                .expect("No layouts available")
                .id,
        );
        let load_layout = state.load_layout(active_layout_id.unique, main_window_id);

        (
            state,
            open_main_window
                .discard()
                .chain(load_layout)
                .chain(launch_sidebar.map(Message::Sidebar)),
        )
    }

    /// به‌روزرسانی وضعیت برنامه بر اساس پیام‌های دریافتی
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // رویدادهای وب‌ساکت بازار (قیمت‌ها، معاملات و غیره)
            Message::MarketWsEvent(event) => {
                let main_window_id = self.main_window.id;
                let dashboard = self.active_dashboard_mut();

                match event {
                    exchange::Event::Connected(exchange) => {
                        log::info!("a stream connected to {exchange} WS");
                    }
                    exchange::Event::Disconnected(exchange, reason) => {
                        log::info!("a stream disconnected from {exchange} WS: {reason:?}");
                    }
                    exchange::Event::DepthReceived(
                        stream,
                        depth_update_t,
                        depth,
                        trades_buffer,
                    ) => {
                        // به‌روزرسانی عمق بازار و معاملات
                        let task = dashboard
                            .update_depth_and_trades(
                                &stream,
                                depth_update_t,
                                &depth,
                                &trades_buffer,
                                main_window_id,
                            )
                            .map(move |msg| Message::Dashboard {
                                layout_id: None,
                                event: msg,
                            });

                        // پخش صدا در صورت نیاز
                        if let Err(err) = self.audio_stream.try_play_sound(&stream, &trades_buffer)
                        {
                            log::error!("Failed to play sound: {err}");
                        }

                        return task;
                    }
                    exchange::Event::KlineReceived(stream, kline) => {
                        // به‌روزرسانی کندل‌ها
                        return dashboard
                            .update_latest_klines(&stream, &kline, main_window_id)
                            .map(move |msg| Message::Dashboard {
                                layout_id: None,
                                event: msg,
                            });
                    }
                }
            }
            // تیک زمان برای به‌روزرسانی‌های دوره‌ای
            Message::Tick(now) => {
                let main_window_id = self.main_window.id;

                return self
                    .active_dashboard_mut()
                    .tick(now, main_window_id)
                    .map(move |msg| Message::Dashboard {
                        layout_id: None,
                        event: msg,
                    });
            }
            // رویدادهای پنجره (مانند درخواست بستن)
            Message::WindowEvent(event) => match event {
                window::Event::CloseRequested(window) => {
                    let main_window = self.main_window.id;
                    let dashboard = self.active_dashboard_mut();

                    if window != main_window {
                        dashboard.popout.remove(&window);
                        return window::close(window);
                    }

                    let mut active_windows = dashboard
                        .popout
                        .keys()
                        .copied()
                        .collect::<Vec<window::Id>>();
                    active_windows.push(main_window);

                    return window::collect_window_specs(active_windows, Message::ExitRequested);
                }
            },
            // خروج از برنامه و ذخیره وضعیت
            Message::ExitRequested(windows) => {
                self.save_state_to_disk(&windows);
                return iced::exit();
            }
            // راه‌اندازی مجدد برنامه
            Message::RestartRequested(windows) => {
                self.save_state_to_disk(&windows);
                return self.restart();
            }
            // بازگشت به عقب (دکمه Esc)
            Message::GoBack => {
                let main_window = self.main_window.id;

                if self.confirm_dialog.is_some() {
                    self.confirm_dialog = None;
                } else if self.sidebar.active_menu().is_some() {
                    self.sidebar.set_menu(None);
                } else {
                    let dashboard = self.active_dashboard_mut();

                    if dashboard.go_back(main_window) {
                        return Task::none();
                    } else if dashboard.focus.is_some() {
                        dashboard.focus = None;
                    } else {
                        self.sidebar.hide_tickers_table();
                    }
                }
            }
            // تغییر تم برنامه
            Message::ThemeSelected(theme) => {
                self.theme = theme.clone();
            }
            // رویدادهای مربوط به داشبورد (نمودارها و پنل‌ها)
            Message::Dashboard {
                layout_id: id,
                event: msg,
            } => {
                let Some(active_layout) = self.layout_manager.active_layout_id() else {
                    log::error!("No active layout to handle dashboard message");
                    return Task::none();
                };

                let main_window = self.main_window;
                let layout_id = id.unwrap_or(active_layout.unique);

                if let Some(dashboard) = self.layout_manager.mut_dashboard(layout_id) {
                    let (main_task, event) = dashboard.update(msg, &main_window, &layout_id);

                    let additional_task = match event {
                        // توزیع داده‌های دریافت شده بین پنل‌ها
                        Some(dashboard::Event::DistributeFetchedData {
                            layout_id,
                            pane_id,
                            data,
                            stream,
                        }) => dashboard
                            .distribute_fetched_data(main_window.id, pane_id, data, stream)
                            .map(move |msg| Message::Dashboard {
                                layout_id: Some(layout_id),
                                event: msg,
                            }),
                        // نمایش اعلان جدید
                        Some(dashboard::Event::Notification(toast)) => {
                            self.notifications.push(toast);
                            Task::none()
                        }
                        // حل کردن استریم‌های ذخیره شده
                        Some(dashboard::Event::ResolveStreams { pane_id, streams }) => {
                            let tickers_info = self.sidebar.tickers_info();

                            let resolved_streams =
                                streams.into_iter().try_fold(vec![], |mut acc, persist| {
                                    let resolver = |t: &exchange::Ticker| {
                                        tickers_info.get(t).and_then(|opt| *opt)
                                    };

                                    match persist.into_stream_kind(resolver) {
                                        Ok(stream) => {
                                            acc.push(stream);
                                            Ok(acc)
                                        }
                                        Err(err) => Err(format!(
                                            "Failed to resolve persisted stream: {}",
                                            err
                                        )),
                                    }
                                });

                            match resolved_streams {
                                Ok(resolved) => {
                                    if resolved.is_empty() {
                                        Task::none()
                                    } else {
                                        dashboard
                                            .resolve_streams(main_window.id, pane_id, resolved)
                                            .map(move |msg| Message::Dashboard {
                                                layout_id: None,
                                                event: msg,
                                            })
                                    }
                                }
                                Err(err) => {
                                    log::warn!("{err}",);
                                    Task::none()
                                }
                            }
                        }
                        None => Task::none(),
                    };

                    return main_task
                        .map(move |msg| Message::Dashboard {
                            layout_id: Some(layout_id),
                            event: msg,
                        })
                        .chain(additional_task);
                }
            }
            // حذف یک اعلان از لیست
            Message::RemoveNotification(index) => {
                if index < self.notifications.len() {
                    self.notifications.remove(index);
                }
            }
            // تنظیم منطقه زمانی
            Message::SetTimezone(tz) => {
                self.timezone = tz;
            }
            // تغییر ضریب بزرگنمایی رابط کاربری
            Message::ScaleFactorChanged(value) => {
                self.ui_scale_factor = value;
            }
            // فعال/غیرفعال کردن دریافت معاملات
            Message::ToggleTradeFetch(checked) => {
                self.layout_manager
                    .iter_dashboards_mut()
                    .for_each(|dashboard| {
                        dashboard.toggle_trade_fetch(checked, &self.main_window);
                    });

                if checked {
                    self.confirm_dialog = None;
                }
            }
            // نمایش/پنهان کردن دیالوگ تایید
            Message::ToggleDialogModal(dialog) => {
                self.confirm_dialog = dialog;
            }
            // مدیریت چیدمان‌ها (تغییر، کپی و غیره)
            Message::Layouts(message) => {
                let action = self.layout_manager.update(message);

                match action {
                    // انتخاب یک چیدمان جدید
                    Some(modal::layout_manager::Action::Select(layout)) => {
                        let active_popout_keys = self
                            .active_dashboard()
                            .popout
                            .keys()
                            .copied()
                            .collect::<Vec<_>>();

                        let window_tasks = Task::batch(
                            active_popout_keys
                                .iter()
                                .map(|&popout_id| window::close::<window::Id>(popout_id))
                                .collect::<Vec<_>>(),
                        )
                        .discard();

                        let old_layout_id = self
                            .layout_manager
                            .active_layout_id()
                            .as_ref()
                            .map(|layout| layout.unique);

                        return window::collect_window_specs(
                            active_popout_keys,
                            dashboard::Message::SavePopoutSpecs,
                        )
                        .map(move |msg| Message::Dashboard {
                            layout_id: old_layout_id,
                            event: msg,
                        })
                        .chain(window_tasks)
                        .chain(self.load_layout(layout, self.main_window.id));
                    }
                    // کپی کردن یک چیدمان
                    Some(modal::layout_manager::Action::Clone(id)) => {
                        let manager = &mut self.layout_manager;

                        let source_data = manager.get(id).map(|layout| {
                            (
                                layout.id.name.clone(),
                                layout.id.unique,
                                data::Dashboard::from(&layout.dashboard),
                            )
                        });

                        if let Some((name, old_id, ser_dashboard)) = source_data {
                            let new_uid = uuid::Uuid::new_v4();
                            let new_layout = LayoutId {
                                unique: new_uid,
                                name: manager.ensure_unique_name(&name, new_uid),
                            };

                            let mut popout_windows = Vec::new();

                            for (pane, window_spec) in &ser_dashboard.popout {
                                let configuration = configuration(pane.clone());
                                popout_windows.push((configuration, *window_spec));
                            }

                            let dashboard = Dashboard::from_config(
                                configuration(ser_dashboard.pane.clone()),
                                popout_windows,
                                old_id,
                            );

                            manager.insert_layout(new_layout.clone(), dashboard);
                        }
                    }
                    None => {}
                }
            }
            // به‌روزرسانی تنظیمات صدا
            Message::AudioStream(message) => self.audio_stream.update(message),
            // باز کردن پوشه داده‌ها
            Message::DataFolderRequested => {
                if let Err(err) = data::open_data_folder() {
                    self.notifications
                        .push(Toast::error(format!("Failed to open data folder: {err}")));
                }
            }
            // پیام‌های مربوط به ویرایشگر تم
            Message::ThemeEditor(msg) => {
                let action = self.theme_editor.update(msg, &self.theme.clone().into());

                match action {
                    Some(modal::theme_editor::Action::Exit) => {
                        self.sidebar.set_menu(Some(sidebar::Menu::Settings));
                    }
                    Some(modal::theme_editor::Action::UpdateTheme(theme)) => {
                        self.theme = data::Theme(theme);

                        let main_window = self.main_window.id;

                        self.active_dashboard_mut()
                            .invalidate_all_panes(main_window);
                    }
                    None => {}
                }
            }
            // پیام‌های مربوط به سایدبار
            Message::Sidebar(message) => {
                let (task, action) = self.sidebar.update(message);

                match action {
                    // انتخاب یک نماد (Ticker) از لیست
                    Some(dashboard::sidebar::Action::TickerSelected(ticker_info, content)) => {
                        let main_window_id = self.main_window.id;

                        let task = {
                            if let Some(kind) = content {
                                self.active_dashboard_mut().init_focused_pane(
                                    main_window_id,
                                    ticker_info,
                                    kind,
                                )
                            } else {
                                self.active_dashboard_mut()
                                    .switch_tickers_in_group(main_window_id, ticker_info)
                            }
                        };

                        return task.map(move |msg| Message::Dashboard {
                            layout_id: None,
                            event: msg,
                        });
                    }
                    // بروز خطا در سایدبار
                    Some(dashboard::sidebar::Action::ErrorOccurred(err)) => {
                        self.notifications.push(Toast::error(err.to_string()));
                    }
                    None => {}
                }

                return task.map(Message::Sidebar);
            }
            // اعمال واحد حجم جدید و راه‌اندازی مجدد برای اعمال تغییرات
            Message::ApplyVolumeSizeUnit(pref) => {
                self.volume_size_unit = pref;
                self.confirm_dialog = None;

                let mut active_windows: Vec<window::Id> =
                    self.active_dashboard().popout.keys().copied().collect();
                active_windows.push(self.main_window.id);

                return window::collect_window_specs(active_windows, Message::RestartRequested);
            }
        }
        Task::none()
    }

    /// رندر کردن رابط کاربری برنامه
    fn view(&self, id: window::Id) -> Element<'_, Message> {
        let dashboard = self.active_dashboard();
        let sidebar_pos = self.sidebar.position();

        let tickers_table = &self.sidebar.tickers_table;

        // بررسی اینکه آیا پنجره فعلی، پنجره اصلی است یا یک پنجره پاپ‌اوت
        let content = if id == self.main_window.id {
            // نمای سایدبار
            let sidebar_view = self
                .sidebar
                .view(self.audio_stream.volume())
                .map(Message::Sidebar);

            // نمای داشبورد اصلی
            let dashboard_view = dashboard
                .view(&self.main_window, tickers_table, self.timezone)
                .map(move |msg| Message::Dashboard {
                    layout_id: None,
                    event: msg,
                });

            // عنوان هدر (مخصوص مک‌او‌اس)
            let header_title = {
                #[cfg(target_os = "macos")]
                {
                    iced::widget::center(
                        text("FLOWSURFACE")
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .size(16)
                            .style(style::title_text),
                    )
                    .height(20)
                    .align_y(Alignment::Center)
                    .padding(padding::top(4))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    column![]
                }
            };

            // چیدمان کلی صفحه اصلی (سایدبار + داشبورد)
            let base = column![
                header_title,
                match sidebar_pos {
                    sidebar::Position::Left => row![sidebar_view, dashboard_view,],
                    sidebar::Position::Right => row![dashboard_view, sidebar_view],
                }
                .spacing(4)
                .padding(8),
            ];

            // اگر منویی در سایدبار باز است، آن را به صورت مودال نمایش بده
            if let Some(menu) = self.sidebar.active_menu() {
                self.view_with_modal(base.into(), dashboard, menu)
            } else {
                base.into()
            }
        } else {
            // نمای پنجره‌های پاپ‌اوت (جدا شده)
            container(
                dashboard
                    .view_window(id, &self.main_window, tickers_table, self.timezone)
                    .map(move |msg| Message::Dashboard {
                        layout_id: None,
                        event: msg,
                    }),
            )
            .padding(padding::top(style::TITLE_PADDING_TOP))
            .into()
        };

        // مدیریت نمایش اعلان‌ها روی محتوا
        toast::Manager::new(
            content,
            &self.notifications,
            match sidebar_pos {
                sidebar::Position::Left => Alignment::Start,
                sidebar::Position::Right => Alignment::End,
            },
            Message::RemoveNotification,
        )
        .into()
    }

    /// تعیین تم برنامه برای هر پنجره
    fn theme(&self, _window: window::Id) -> iced_core::Theme {
        self.theme.clone().into()
    }

    /// تعیین عنوان پنجره
    fn title(&self, _window: window::Id) -> String {
        if let Some(id) = self.layout_manager.active_layout_id() {
            format!("Flowsurface [{}]", id.name)
        } else {
            "Flowsurface".to_string()
        }
    }

    /// تعیین ضریب بزرگنمایی برای هر پنجره
    fn scale_factor(&self, _window: window::Id) -> f32 {
        self.ui_scale_factor.into()
    }

    /// مدیریت اشتراک‌ها (Subscriptions) برای رویدادهای خارجی
    fn subscription(&self) -> Subscription<Message> {
        // رویدادهای پنجره
        let window_events = window::events().map(Message::WindowEvent);
        // رویدادهای سایدبار
        let sidebar = self.sidebar.subscription().map(Message::Sidebar);

        // استریم‌های داده‌های بازار
        let exchange_streams = self
            .active_dashboard()
            .market_subscriptions()
            .map(Message::MarketWsEvent);

        // تیک زمان هر ۱۰۰ میلی‌ثانیه
        let tick = iced::time::every(std::time::Duration::from_millis(100)).map(Message::Tick);

        // گوش دادن به کلیدهای کیبورد (مثلاً Esc برای بازگشت)
        let hotkeys = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, .. } = event else {
                return None;
            };
            match key {
                keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::GoBack),
                _ => None,
            }
        });

        Subscription::batch(vec![
            exchange_streams,
            sidebar,
            window_events,
            tick,
            hotkeys,
        ])
    }

    /// دریافت مرجع به داشبورد فعال
    fn active_dashboard(&self) -> &Dashboard {
        let active_layout = self
            .layout_manager
            .active_layout_id()
            .expect("No active layout");
        self.layout_manager
            .get(active_layout.unique)
            .map(|layout| &layout.dashboard)
            .expect("No active dashboard")
    }

    /// دریافت مرجع قابل تغییر به داشبورد فعال
    fn active_dashboard_mut(&mut self) -> &mut Dashboard {
        let active_layout = self
            .layout_manager
            .active_layout_id()
            .expect("No active layout");
        self.layout_manager
            .get_mut(active_layout.unique)
            .map(|layout| &mut layout.dashboard)
            .expect("No active dashboard")
    }

    /// بارگذاری یک چیدمان خاص
    fn load_layout(&mut self, layout_uid: uuid::Uuid, main_window: window::Id) -> Task<Message> {
        match self.layout_manager.set_active_layout(layout_uid) {
            Ok(layout) => {
                layout
                    .dashboard
                    .load_layout(main_window)
                    .map(move |msg| Message::Dashboard {
                        layout_id: Some(layout_uid),
                        event: msg,
                    })
            }
            Err(err) => {
                log::error!("Failed to set active layout: {}", err);
                Task::none()
            }
        }
    }

    /// نمایش محتوا به همراه یک پنجره مودال (تنظیمات، چیدمان، صدا و غیره)
    fn view_with_modal<'a>(
        &'a self,
        base: Element<'a, Message>,
        dashboard: &'a Dashboard,
        menu: sidebar::Menu,
    ) -> Element<'a, Message> {
        let sidebar_pos = self.sidebar.position();

        match menu {
            // منوی تنظیمات
            sidebar::Menu::Settings => {
                let settings_modal = {
                    // لیست انتخاب تم
                    let theme_picklist = {
                        let mut themes: Vec<iced::Theme> = iced_core::Theme::ALL.to_vec();

                        let default_theme = iced_core::Theme::Custom(default_theme().into());
                        themes.push(default_theme);

                        if let Some(custom_theme) = &self.theme_editor.custom_theme {
                            themes.push(custom_theme.clone());
                        }

                        pick_list(themes, Some(self.theme.0.clone()), |theme| {
                            Message::ThemeSelected(data::Theme(theme))
                        })
                    };

                    // دکمه ویرایشگر تم
                    let toggle_theme_editor = button(text("Theme editor")).on_press(
                        Message::Sidebar(dashboard::sidebar::Message::ToggleSidebarMenu(Some(
                            sidebar::Menu::ThemeEditor,
                        ))),
                    );

                    // لیست انتخاب منطقه زمانی
                    let timezone_picklist = pick_list(
                        [data::UserTimezone::Utc, data::UserTimezone::Local],
                        Some(self.timezone),
                        Message::SetTimezone,
                    );

                    // چک‌باکس نمایش حجم بر اساس ارز کوت (مثلاً USD)
                    let size_in_quote_currency_checkbox = {
                        let is_active = match self.volume_size_unit {
                            exchange::SizeUnit::Quote => true,
                            exchange::SizeUnit::Base => false,
                        };

                        let checkbox = iced::widget::checkbox(is_active)
                            .label("Size in quote currency")
                            .on_toggle(|checked| {
                                let on_dialog_confirm = Message::ApplyVolumeSizeUnit(if checked {
                                    exchange::SizeUnit::Quote
                                } else {
                                    exchange::SizeUnit::Base
                                });

                                let confirm_dialog = screen::ConfirmDialog::new(
                                    "Changing size display currency requires application restart"
                                        .to_string(),
                                    Box::new(on_dialog_confirm.clone()),
                                )
                                .with_confirm_btn_text("Restart now".to_string());

                                Message::ToggleDialogModal(Some(confirm_dialog))
                            });

                        tooltip(
                            checkbox,
                            Some(
                                "Display sizes/volumes in quote currency (USD)\nHas no effect on inverse perps or open interest",
                            ),
                            TooltipPosition::Top,
                        )
                    };

                    // انتخاب موقعیت سایدبار (چپ یا راست)
                    let sidebar_pos = pick_list(
                        [sidebar::Position::Left, sidebar::Position::Right],
                        Some(sidebar_pos),
                        |pos| {
                            Message::Sidebar(dashboard::sidebar::Message::SetSidebarPosition(pos))
                        },
                    );

                    // تنظیم ضریب بزرگنمایی رابط کاربری
                    let scale_factor = {
                        let current_value: f32 = self.ui_scale_factor.into();

                        let decrease_btn = if current_value > data::config::MIN_SCALE {
                            button(text("-"))
                                .on_press(Message::ScaleFactorChanged((current_value - 0.1).into()))
                        } else {
                            button(text("-"))
                        };

                        let increase_btn = if current_value < data::config::MAX_SCALE {
                            button(text("+"))
                                .on_press(Message::ScaleFactorChanged((current_value + 0.1).into()))
                        } else {
                            button(text("+"))
                        };

                        container(
                            row![
                                decrease_btn,
                                text(format!("{:.0}%", current_value * 100.0)).size(14),
                                increase_btn,
                            ]
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .padding(4),
                        )
                        .style(style::modal_container)
                    };

                    // چک‌باکس دریافت معاملات (تجربی)
                    let trade_fetch_checkbox = {
                        let is_active = exchange::fetcher::is_trade_fetch_enabled();

                        let checkbox = iced::widget::checkbox(is_active)
                            .label("Fetch trades (Binance)")
                            .on_toggle(|checked| {
                                if checked {
                                    let confirm_dialog = screen::ConfirmDialog::new(
                                        "This might be unreliable and take some time to complete. Proceed?"
                                            .to_string(),
                                        Box::new(Message::ToggleTradeFetch(true)),
                                    );
                                    Message::ToggleDialogModal(Some(confirm_dialog))
                                } else {
                                    Message::ToggleTradeFetch(false)
                                }
                            });

                        tooltip(
                            checkbox,
                            Some("Try to fetch trades for footprint charts"),
                            TooltipPosition::Top,
                        )
                    };

                    // دکمه باز کردن پوشه داده‌ها
                    let open_data_folder = {
                        let button =
                            button(text("Open data folder")).on_press(Message::DataFolderRequested);

                        tooltip(
                            button,
                            Some("Open the folder where the data & config is stored"),
                            TooltipPosition::Top,
                        )
                    };

                    // چیدمان ستونی محتوای تنظیمات
                    let column_content = split_column![
                        column![open_data_folder,].spacing(8),
                        column![text("Sidebar position").size(14), sidebar_pos,].spacing(12),
                        column![text("Time zone").size(14), timezone_picklist,].spacing(12),
                        column![text("Market data").size(14), size_in_quote_currency_checkbox,].spacing(12),
                        column![text("Theme").size(14), theme_picklist,].spacing(12),
                        column![text("Interface scale").size(14), scale_factor,].spacing(12),
                        column![
                            text("Experimental").size(14),
                            column![trade_fetch_checkbox, toggle_theme_editor,].spacing(8),
                        ]
                        .spacing(12),
                        ; spacing = 16, align_x = Alignment::Start
                    ];

                    let content = scrollable::Scrollable::with_direction(
                        column_content,
                        scrollable::Direction::Vertical(
                            scrollable::Scrollbar::new().width(8).scroller_width(6),
                        ),
                    );

                    container(content)
                        .align_x(Alignment::Start)
                        .max_width(240)
                        .padding(24)
                        .style(style::dashboard_modal)
                };

                let (align_x, padding) = match sidebar_pos {
                    sidebar::Position::Left => (Alignment::Start, padding::left(44).bottom(4)),
                    sidebar::Position::Right => (Alignment::End, padding::right(44).bottom(4)),
                };

                let base_content = dashboard_modal(
                    base,
                    settings_modal,
                    Message::Sidebar(dashboard::sidebar::Message::ToggleSidebarMenu(None)),
                    padding,
                    Alignment::End,
                    align_x,
                );

                // اگر دیالوگ تایید باز است، آن را روی تنظیمات نشان بده
                if let Some(dialog) = &self.confirm_dialog {
                    let dialog_content =
                        confirm_dialog_container(dialog.clone(), Message::ToggleDialogModal(None));

                    main_dialog_modal(
                        base_content,
                        dialog_content,
                        Message::ToggleDialogModal(None),
                    )
                } else {
                    base_content
                }
            }
            // منوی مدیریت چیدمان
            sidebar::Menu::Layout => {
                let main_window = self.main_window.id;

                // مدیریت پنل انتخاب شده
                let manage_pane = if let Some((window_id, pane_id)) = dashboard.focus {
                    let selected_pane_str =
                        if let Some(state) = dashboard.get_pane(main_window, window_id, pane_id) {
                            let link_group_name: String =
                                state.link_group.as_ref().map_or_else(String::new, |g| {
                                    " - Group ".to_string() + &g.to_string()
                                });

                            state.content.to_string() + &link_group_name
                        } else {
                            "".to_string()
                        };

                    let is_main_window = window_id == main_window;

                    // دکمه ریست کردن پنل
                    let reset_pane_button = {
                        let btn = button(text("Reset").align_x(Alignment::Center))
                            .width(iced::Length::Fill);
                        if is_main_window {
                            let dashboard_msg = Message::Dashboard {
                                layout_id: None,
                                event: dashboard::Message::Pane(
                                    main_window,
                                    dashboard::pane::Message::ReplacePane(pane_id),
                                ),
                            };

                            btn.on_press(dashboard_msg)
                        } else {
                            btn
                        }
                    };
                    // دکمه تقسیم کردن پنل
                    let split_pane_button = {
                        let btn = button(text("Split").align_x(Alignment::Center))
                            .width(iced::Length::Fill);
                        if is_main_window {
                            let dashboard_msg = Message::Dashboard {
                                layout_id: None,
                                event: dashboard::Message::Pane(
                                    main_window,
                                    dashboard::pane::Message::SplitPane(
                                        pane_grid::Axis::Horizontal,
                                        pane_id,
                                    ),
                                ),
                            };
                            btn.on_press(dashboard_msg)
                        } else {
                            btn
                        }
                    };

                    column![
                        text(selected_pane_str),
                        row![
                            tooltip(
                                reset_pane_button,
                                if is_main_window {
                                    Some("Reset selected pane")
                                } else {
                                    None
                                },
                                TooltipPosition::Top,
                            ),
                            tooltip(
                                split_pane_button,
                                if is_main_window {
                                    Some("Split selected pane horizontally")
                                } else {
                                    None
                                },
                                TooltipPosition::Top,
                            ),
                        ]
                        .spacing(8)
                    ]
                    .spacing(8)
                } else {
                    column![text("No pane selected"),].spacing(8)
                };

                // محتوای مودال مدیریت چیدمان
                let manage_layout_modal = {
                    let col = column![
                        manage_pane,
                        rule::horizontal(1.0).style(style::split_ruler),
                        self.layout_manager.view().map(Message::Layouts)
                    ];

                    container(col.align_x(Alignment::Center).spacing(20))
                        .width(260)
                        .padding(24)
                        .style(style::dashboard_modal)
                };

                let (align_x, padding) = match sidebar_pos {
                    sidebar::Position::Left => (Alignment::Start, padding::left(44).top(40)),
                    sidebar::Position::Right => (Alignment::End, padding::right(44).top(40)),
                };

                dashboard_modal(
                    base,
                    manage_layout_modal,
                    Message::Sidebar(dashboard::sidebar::Message::ToggleSidebarMenu(None)),
                    padding,
                    Alignment::Start,
                    align_x,
                )
            }
            // منوی تنظیمات صدا
            sidebar::Menu::Audio => {
                let (align_x, padding) = match sidebar_pos {
                    sidebar::Position::Left => (Alignment::Start, padding::left(44).top(76)),
                    sidebar::Position::Right => (Alignment::End, padding::right(44).top(76)),
                };

                let depth_streams_list = dashboard.streams.depth_streams(None);

                dashboard_modal(
                    base,
                    self.audio_stream
                        .view(depth_streams_list)
                        .map(Message::AudioStream),
                    Message::Sidebar(dashboard::sidebar::Message::ToggleSidebarMenu(None)),
                    padding,
                    Alignment::Start,
                    align_x,
                )
            }
            // منوی ویرایشگر تم
            sidebar::Menu::ThemeEditor => {
                let (align_x, padding) = match sidebar_pos {
                    sidebar::Position::Left => (Alignment::Start, padding::left(44).bottom(4)),
                    sidebar::Position::Right => (Alignment::End, padding::right(44).bottom(4)),
                };

                dashboard_modal(
                    base,
                    self.theme_editor
                        .view(&self.theme.0)
                        .map(Message::ThemeEditor),
                    Message::Sidebar(dashboard::sidebar::Message::ToggleSidebarMenu(None)),
                    padding,
                    Alignment::End,
                    align_x,
                )
            }
        }
    }

    /// ذخیره وضعیت فعلی برنامه روی دیسک
    fn save_state_to_disk(&mut self, windows: &HashMap<window::Id, WindowSpec>) {
        // به‌روزرسانی مشخصات پنجره‌های پاپ‌اوت
        self.active_dashboard_mut()
            .popout
            .iter_mut()
            .for_each(|(id, (_, window_spec))| {
                if let Some(new_window_spec) = windows.get(id) {
                    *window_spec = *new_window_spec;
                }
            });

        // همگام‌سازی تنظیمات جدول نمادها در سایدبار
        self.sidebar.sync_tickers_table_settings();

        // سریال‌سازی چیدمان‌ها برای ذخیره
        let mut ser_layouts = vec![];
        for layout in &self.layout_manager.layouts {
            if let Some(layout) = self.layout_manager.get(layout.id.unique) {
                let serialized_dashboard = data::Dashboard::from(&layout.dashboard);
                ser_layouts.push(data::Layout {
                    name: layout.id.name.clone(),
                    dashboard: serialized_dashboard,
                });
            }
        }

        let layouts = data::Layouts {
            layouts: ser_layouts,
            active_layout: self
                .layout_manager
                .active_layout_id()
                .map(|layout| layout.name.to_string())
                .clone(),
        };

        // دریافت مشخصات پنجره اصلی
        let main_window_spec = windows
            .iter()
            .find(|(id, _)| **id == self.main_window.id)
            .map(|(_, spec)| *spec);

        // ذخیره تنظیمات صدا
        let audio_cfg = data::AudioStream::from(&self.audio_stream);

        // ایجاد ساختار نهایی وضعیت برای ذخیره در فایل JSON
        let state = data::State::from_parts(
            layouts,
            self.theme.clone(),
            self.theme_editor.custom_theme.clone().map(data::Theme),
            main_window_spec,
            self.timezone,
            self.sidebar.state.clone(),
            self.ui_scale_factor,
            audio_cfg,
            self.volume_size_unit,
        );

        // نوشتن وضعیت در فایل
        match serde_json::to_string(&state) {
            Ok(layout_str) => {
                let file_name = data::SAVED_STATE_PATH;
                if let Err(e) = data::write_json_to_file(&layout_str, file_name) {
                    log::error!("Failed to write layout state to file: {}", e);
                } else {
                    log::info!("Persisted state to {file_name}");
                }
            }
            Err(e) => log::error!("Failed to serialize layout: {}", e),
        }
    }

    /// راه‌اندازی مجدد برنامه (بستن تمام پنجره‌ها و ایجاد نمونه جدید)
    fn restart(&mut self) -> Task<Message> {
        let mut windows_to_close: Vec<window::Id> =
            self.active_dashboard().popout.keys().copied().collect();
        windows_to_close.push(self.main_window.id);

        let close_windows = Task::batch(
            windows_to_close
                .into_iter()
                .map(window::close)
                .collect::<Vec<_>>(),
        );

        let (new_state, init_task) = Flowsurface::new();
        *self = new_state;

        close_windows.chain(init_task)
    }
}
