use crate::chart::comparison::ComparisonChart;
use crate::screen::dashboard::pane::{Event, Message as PaneMessage};
use crate::screen::dashboard::panel::timeandsales;
use crate::split_column;
use crate::widget::{classic_slider_row, labeled_slider, numeric_input_box};
use crate::{style, tooltip, widget::scrollable_content};

use data::chart::heatmap::HeatmapStudy;
use data::chart::kline::FootprintStudy;
use data::chart::{
    KlineChartKind,
    heatmap::{self, CoalesceKind},
    kline::ClusterKind,
};
use data::layout::pane::VisualConfig;
use data::panel::ladder;
use data::panel::timeandsales::{StackedBar, StackedBarRatio};
use data::util::format_with_commas;

use iced::widget::{checkbox, space};
use iced::{
    Alignment, Element, Length,
    widget::{
        button, column, container, pane_grid, pick_list, radio, row, slider, text,
        tooltip::Position as TooltipPosition,
    },
};
use std::time::Duration;



#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub trade_size_input: String,
    pub order_size_input: String,
    pub color_scale_input: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    TradeSizeInputChanged(String),
    OrderSizeInputChanged(String),
    ColorScaleInputChanged(String),
    ReloadData,
}

pub fn cfg_view_container<'a, T>(max_width: u32, content: T) -> Element<'a, PaneMessage>
where
    T: Into<Element<'a, PaneMessage>>,
{
    container(scrollable_content(content))
        .width(Length::Shrink)
        .padding(28)
        .max_width(max_width)
        .style(style::chart_modal)
        .into()
}

pub fn heatmap_cfg_view<'a>(
    cfg: heatmap::Config,
    pane: pane_grid::Pane,
    study_config: &'a study::Configurator<HeatmapStudy>,
    studies: &'a [HeatmapStudy],
    basis: data::chart::Basis,
    state: &'a State,
) -> Element<'a, PaneMessage> {
    let trade_size_slider = {
        let filter = cfg.trade_size_filter;
        let slider = labeled_slider(
            "Trade",
            0.0..=1_000_000_000.0,
            filter,
            move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        trade_size_filter: value,
                        ..cfg
                    }),
                    false,
                )
            },
            |value| format!(">${}", format_with_commas(*value)),
            Some(500.0),
        );

        let input = numeric_input_box(
            "Min Size",
            "0",
            &state.trade_size_input,
            state.trade_size_input.parse::<f64>().is_ok(),
            move |s| {
                PaneMessage::PaneEvent(
                    pane,
                    Event::SettingsInteraction(self::Message::TradeSizeInputChanged(s)),
                )
            },
            None,
        );

        column![slider, input].spacing(4)
    };

    let order_size_slider = {
        let filter = cfg.order_size_filter;
        let slider = labeled_slider(
            "Order",
            0.0..=1_000_000_000.0,
            filter,
            move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        order_size_filter: value,
                        ..cfg
                    }),
                    false,
                )
            },
            |value| format!(">${}", format_with_commas(*value)),
            Some(5000.0),
        );

        let input = numeric_input_box(
            "Min Size",
            "0",
            &state.order_size_input,
            state.order_size_input.parse::<f64>().is_ok(),
            move |s| {
                PaneMessage::PaneEvent(
                    pane,
                    Event::SettingsInteraction(self::Message::OrderSizeInputChanged(s)),
                )
            },
            None,
        );

        column![slider, input].spacing(4)
    };

    let circle_scaling_slider = cfg.trade_size_scale.map(|radius_scale| {
        classic_slider_row(
            text("Circle radius scaling"),
            slider(10..=200, radius_scale, move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        trade_size_scale: Some(value),
                        ..cfg
                    }),
                    false,
                )
            })
            .step(10)
            .into(),
            Some(text(format!("{}%", radius_scale)).size(13)),
        )
    });

    let coalescer_cfg: Option<Element<_>> = if let Some(coalescing) = cfg.coalescing {
        let threshold_pct = coalescing.threshold();

        let coalescer_kinds = {
            let average = radio(
                "Average",
                CoalesceKind::Average(threshold_pct),
                Some(coalescing),
                move |value| {
                    PaneMessage::VisualConfigChanged(
                        pane,
                        VisualConfig::Heatmap(heatmap::Config {
                            coalescing: Some(value),
                            ..cfg
                        }),
                        false,
                    )
                },
            )
            .spacing(4);

            let first = radio(
                "First",
                CoalesceKind::First(threshold_pct),
                Some(coalescing),
                move |value| {
                    PaneMessage::VisualConfigChanged(
                        pane,
                        VisualConfig::Heatmap(heatmap::Config {
                            coalescing: Some(value),
                            ..cfg
                        }),
                        false,
                    )
                },
            )
            .spacing(4);

            let max = radio(
                "Max",
                CoalesceKind::Max(threshold_pct),
                Some(coalescing),
                move |value| {
                    PaneMessage::VisualConfigChanged(
                        pane,
                        VisualConfig::Heatmap(heatmap::Config {
                            coalescing: Some(value),
                            ..cfg
                        }),
                        false,
                    )
                },
            )
            .spacing(4);

            row![
                text("Merge method: "),
                row![average, first, max].spacing(12)
            ]
            .spacing(12)
        };

        let threshold_slider = classic_slider_row(
            text("Size similarity"),
            slider(0.05..=0.8, threshold_pct, move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        coalescing: Some(coalescing.with_threshold(value)),
                        ..cfg
                    }),
                    false,
                )
            })
            .step(0.05)
            .into(),
            Some(text(format!("{:.0}%", threshold_pct * 100.0)).size(13)),
        );

        Some(
            container(column![coalescer_kinds, threshold_slider].spacing(8))
                .style(style::modal_container)
                .padding(8)
                .into(),
        )
    } else {
        None
    };

    let size_filters_column = column![
        text("Size filters").size(14),
        column![trade_size_slider, order_size_slider].spacing(8),
    ]
    .spacing(8);

    let noise_filters_column = {
        let merge_checkbox = checkbox(cfg.coalescing.is_some())
            .label("Merge orders if sizes are similar")
            .on_toggle(move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        coalescing: if value {
                            Some(CoalesceKind::Average(0.15))
                        } else {
                            None
                        },
                        ..cfg
                    }),
                    false,
                )
            });

        let mut col = column![text("Noise filters").size(14), merge_checkbox].spacing(8);
        if let Some(c) = coalescer_cfg {
            col = col.push(c);
        }
        col
    };

    let trade_viz_column = {
        let dyn_checkbox = checkbox(cfg.trade_size_scale.is_some())
            .label("Dynamic circle radius")
            .on_toggle(move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Heatmap(heatmap::Config {
                        trade_size_scale: if value { Some(100) } else { None },
                        ..cfg
                    }),
                    false,
                )
            });

        let mut col = column![text("Trade visualization").size(14), dyn_checkbox].spacing(8);
        if let Some(slider) = circle_scaling_slider {
            col = col.push(slider);
        }
        col
    };

    let study_cfg = study_config.view(studies, basis, state).map(move |msg| {
        PaneMessage::PaneEvent(
            pane,
            Event::StudyConfigurator(study::StudyMessage::Heatmap(msg)),
        )
    });

    let content = split_column![
        size_filters_column,
        noise_filters_column,
        trade_viz_column,
        column![text("Studies").size(14), study_cfg].spacing(8),
        row![
            space::horizontal(),
            sync_all_button(pane, VisualConfig::Heatmap(cfg))
        ]
        ; spacing = 12, align_x = Alignment::Start
    ];

    cfg_view_container(360, content)
}

pub fn timesales_cfg_view<'a>(
    cfg: timeandsales::Config,
    pane: pane_grid::Pane,
) -> Element<'a, PaneMessage> {
    let trade_size_column = {
        let filter = cfg.trade_size_filter;
        let slider = labeled_slider(
            "Trade",
            0.0..=1_000_000_000.0, // Uncapped: increased to 1B
            filter,
            move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::TimeAndSales(timeandsales::Config {
                        trade_size_filter: value,
                        ..cfg
                    }),
                    false,
                )
            },
            |value| format!(">${}", format_with_commas(*value)),
            Some(500.0),
        );

        column![text("Size filter").size(14), slider].spacing(8)
    };

    let retention_minutes = (cfg.trade_retention.as_secs_f32() / 60.0).max(1.0);
    let retention_slider = {
        let slider_ui = slider(1.0..=60.0, retention_minutes, move |new_minutes| {
            let mins = new_minutes.round().max(1.0) as u64;
            PaneMessage::VisualConfigChanged(
                pane,
                VisualConfig::TimeAndSales(timeandsales::Config {
                    trade_retention: Duration::from_secs(mins * 60),
                    ..cfg
                }),
                false,
            )
        })
        .step(1.0);

        classic_slider_row(
            text("Keep trades for"),
            slider_ui.into(),
            Some(text(format!("≈ {} min", retention_minutes.round() as u64)).size(13)),
        )
    };

    let history_column = column![
        row![
            text("History").size(14),
            tooltip(
                button("i").style(style::button::info),
                Some("Affects the stacked bar, colors and how much you can scroll down"),
                TooltipPosition::Top,
            )
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        retention_slider
    ]
    .spacing(8);

    let stacked_bar: Element<_> = {
        let is_shown = cfg.stacked_bar.is_some();

        let enable_checkbox = checkbox(is_shown).label("Show stacked bar").on_toggle({
            move |value| {
                let current_ratio = cfg.stacked_bar.map(|h| h.ratio()).unwrap_or_default();
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::TimeAndSales(timeandsales::Config {
                        stacked_bar: if value {
                            Some(StackedBar::Compact(current_ratio))
                        } else {
                            None
                        },
                        ..cfg
                    }),
                    false,
                )
            }
        });

        let controls: Option<Element<_>> = cfg.stacked_bar.map(|hist| {
            let ratio = hist.ratio();
            let is_compact = matches!(hist, StackedBar::Compact(_));

            let compact = radio("Compact", true, Some(is_compact), {
                move |_v| {
                    PaneMessage::VisualConfigChanged(
                        pane,
                        VisualConfig::TimeAndSales(timeandsales::Config {
                            stacked_bar: Some(StackedBar::Compact(ratio)),
                            ..cfg
                        }),
                        false,
                    )
                }
            })
            .spacing(4);

            let full = radio("Full", false, Some(is_compact), {
                move |_v| {
                    PaneMessage::VisualConfigChanged(
                        pane,
                        VisualConfig::TimeAndSales(timeandsales::Config {
                            stacked_bar: Some(StackedBar::Full(ratio)),
                            ..cfg
                        }),
                        false,
                    )
                }
            })
            .spacing(4);

            let metric_picklist = pick_list(StackedBarRatio::ALL, Some(ratio), move |new_ratio| {
                let new_hist = Some(match cfg.stacked_bar {
                    Some(StackedBar::Full(_)) => StackedBar::Full(new_ratio),
                    _ => StackedBar::Compact(new_ratio),
                });
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::TimeAndSales(timeandsales::Config {
                        stacked_bar: new_hist,
                        ..cfg
                    }),
                    false,
                )
            });

            column![
                iced::widget::rule::horizontal(1),
                text("Mode").size(12),
                row![compact, full].spacing(12),
                text("Metric").size(12),
                metric_picklist,
            ]
            .spacing(8)
            .into()
        });

        let mut inner = column![enable_checkbox]
            .width(Length::Fill)
            .padding(4)
            .spacing(8);

        if let Some(ctrls) = controls {
            inner = inner.push(ctrls);
        }

        container(inner)
            .style(style::modal_container)
            .padding(8)
            .into()
    };

    let content = split_column![
        trade_size_column,
        history_column,
        stacked_bar,
        row![space::horizontal(), sync_all_button(pane, VisualConfig::TimeAndSales(cfg))],
        ; spacing = 12, align_x = Alignment::Start
    ];

    cfg_view_container(320, content)
}

pub fn comparison_cfg_view<'a>(
    pane: pane_grid::Pane,
    chart: &'a ComparisonChart,
) -> Element<'a, PaneMessage> {
    let series = &chart.series;
    let series_editor = &chart.series_editor;

    let content = column![series_editor.view(series).map(move |msg| {
        PaneMessage::PaneEvent(
            pane,
            Event::ComparisonChartInteraction(crate::chart::comparison::Message::Editor(msg)),
        )
    })];

    cfg_view_container(320, content)
}

pub fn kline_cfg_view<'a>(
    study_config: &'a study::Configurator<FootprintStudy>,
    cfg: data::chart::kline::Config,
    kind: &'a KlineChartKind,
    pane: pane_grid::Pane,
    basis: data::chart::Basis,
    state: &'a State,
) -> Element<'a, PaneMessage> {
    let content = match kind {
        KlineChartKind::Candles => column![text(
            "This chart type doesn't have any configurations, WIP..."
        )],
        KlineChartKind::Footprint {
            clusters,
            scaling,
            studies,
        } => {
            let cluster_picklist =
                pick_list(ClusterKind::ALL, Some(clusters), move |new_cluster_kind| {
                    PaneMessage::PaneEvent(pane, Event::ClusterKindSelected(new_cluster_kind))
                });

            let scaling = {
                let picklist = pick_list(
                    data::chart::kline::ClusterScaling::ALL,
                    Some(scaling),
                    move |new_scaling| {
                        PaneMessage::PaneEvent(pane, Event::ClusterScalingSelected(new_scaling))
                    },
                );

                if let data::chart::kline::ClusterScaling::Hybrid { weight } = scaling {
                    let hybrid_slider = slider(0.0..=1.0, *weight, move |new_weight| {
                        PaneMessage::PaneEvent(
                            pane,
                            Event::ClusterScalingSelected(
                                data::chart::kline::ClusterScaling::Hybrid { weight: new_weight },
                            ),
                        )
                    })
                    .step(0.05);

                    column![
                        picklist,
                        hybrid_slider,
                        text("Blend visible-range and per-candle scaling"),
                    ]
                    .spacing(8)
                } else {
                    column![picklist].spacing(8)
                }
            };

            let study_cfg = study_config.view(studies, basis, state).map(move |msg| {
                if let study::Message::TopLevel(inner) = msg {
                    PaneMessage::PaneEvent(pane, Event::SettingsInteraction(inner))
                } else {
                    PaneMessage::PaneEvent(
                        pane,
                        Event::StudyConfigurator(study::StudyMessage::Footprint(msg)),
                    )
                }
            });

            split_column![
                column![text("Cluster type").size(14), cluster_picklist].spacing(8),
                column![text("Cluster scaling").size(14), scaling].spacing(8),
                column![text("Studies").size(14), study_cfg].spacing(8),
                row![
                    button("Reload Data")
                        .on_press(PaneMessage::PaneEvent(pane, Event::SettingsInteraction(Message::ReloadData)))
                        .style(|t, s| style::button::cancel(t, s, false)),
                    space::horizontal(),
                    sync_all_button(pane, VisualConfig::Kline(cfg))
                ],
                ; spacing = 12, align_x = Alignment::Start
            ]
        }
    };

    cfg_view_container(360, content)
}

pub fn ladder_cfg_view<'a>(cfg: ladder::Config, pane: pane_grid::Pane) -> Element<'a, PaneMessage> {
    let display_options = {
        let spread = checkbox(cfg.show_spread)
            .label("Show Spread")
            .on_toggle(move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Ladder(ladder::Config {
                        show_spread: value,
                        ..cfg
                    }),
                    false,
                )
            });

        let chase_tracker = checkbox(cfg.show_chase_tracker)
            .label("Show Chase Tracker")
            .on_toggle(move |value| {
                PaneMessage::VisualConfigChanged(
                    pane,
                    VisualConfig::Ladder(ladder::Config {
                        show_chase_tracker: value,
                        ..cfg
                    }),
                    false,
                )
            });

        column![
            text("Display Options").size(14),
            column![
                spread,
                row![
                    chase_tracker,
                    tooltip(
                        button("i").style(style::button::info),
                        Some("Highlights consecutive best-price moves and fades when momentum stalls.\nCalculated using raw ungrouped data."),
                        TooltipPosition::Top,
                    )
                ]
                .align_y(Alignment::Center)
                .spacing(4)
            ]
            .spacing(4)
        ]
        .spacing(8)
    };

    let retention_slider = {
        let retention_minutes = (cfg.trade_retention.as_secs_f32() / 60.0).max(1.0);

        let slider_ui = slider(1.0..=60.0, retention_minutes, move |new_minutes| {
            let mins = new_minutes.round().max(1.0) as u64;
            PaneMessage::VisualConfigChanged(
                pane,
                VisualConfig::Ladder(ladder::Config {
                    trade_retention: Duration::from_secs(mins * 60),
                    ..cfg
                }),
                false,
            )
        })
        .step(1.0);

        classic_slider_row(
            text("Keep trades for"),
            slider_ui.into(),
            Some(text(format!("≈ {} min", retention_minutes.round() as u64)).size(13)),
        )
    };

    let history_column = column![text("History").size(14), retention_slider].spacing(8);

    let content = split_column![
        display_options,
        history_column,
        row![
            space::horizontal(),
            sync_all_button(pane, VisualConfig::Ladder(cfg))
        ],
        ; spacing = 12, align_x = Alignment::Start
    ];

    cfg_view_container(320, content)
}

fn sync_all_button<'a>(pane: pane_grid::Pane, config: VisualConfig) -> Element<'a, PaneMessage> {
    tooltip(
        button("Sync all").on_press(PaneMessage::VisualConfigChanged(pane, config, true)),
        Some("Apply configuration to similar panes"),
        TooltipPosition::Top,
    )
}

pub mod study {
    use crate::{
        split_column,
        style::{self, Icon, icon_text},
    };
    use data::chart::heatmap::HeatmapStudy;
    use data::chart::kline::FootprintStudy;
    use iced::{
        Element, padding,
        widget::{button, checkbox, column, container, row, slider, space, text},
    };

    #[derive(Debug, Clone)]
    pub enum StudyMessage {
        Footprint(Message<FootprintStudy>),
        Heatmap(Message<HeatmapStudy>),
    }

    pub trait Study: Sized + Copy + 'static {
        fn is_same_type(&self, other: &Self) -> bool;
        fn all() -> Vec<Self>;
        fn view_config<'a>(
            &self,
            basis: data::chart::Basis,
            state: &'a crate::modal::pane::settings::State,
            on_change: impl Fn(Self) -> Message<Self> + Copy + 'a,
        ) -> Element<'a, Message<Self>>;
    }


    impl Study for FootprintStudy {
        fn is_same_type(&self, other: &Self) -> bool {
            std::mem::discriminant(self) == std::mem::discriminant(other)
        }

        fn all() -> Vec<Self> {
            FootprintStudy::ALL.to_vec()
        }

        fn view_config<'a>(
            &self,
            _basis: data::chart::Basis,
            state: &'a crate::modal::pane::settings::State,
            on_change: impl Fn(Self) -> Message<Self> + Copy + 'a,
        ) -> Element<'a, Message<Self>> {
            match *self {
                FootprintStudy::NPoC { lookback } => {
                    let slider_ui = slider(10.0..=400.0, lookback as f32, move |new_value| {
                        on_change(FootprintStudy::NPoC {
                            lookback: new_value as usize,
                        })
                    })
                    .step(10.0);

                    column![text(format!("Lookback: {lookback} datapoints")), slider_ui]
                        .padding(8)
                        .spacing(4)
                        .into()
                }
                FootprintStudy::Imbalance {
                    threshold,
                    color_scale,
                    ignore_zeros,
                    stack_count,
                } => {
                    let qty_threshold = {
                        let info_text = text(format!("Ask:Bid threshold: {threshold}%"));

                        let threshold_slider =
                            slider(100.0..=800.0, threshold as f32, move |new_value| {
                                on_change(FootprintStudy::Imbalance {
                                    threshold: new_value as usize,
                                    color_scale,
                                    ignore_zeros,
                                    stack_count,
                                })
                            })
                            .step(25.0);

                        column![info_text, threshold_slider,].padding(8).spacing(4)
                    };

                    let color_scaling = {
                        let color_scale_enabled = color_scale.is_some();
                        let color_scale_value = color_scale.unwrap_or(100);

                        let color_scale_checkbox = checkbox(color_scale_enabled)
                            .label("Dynamic color scaling")
                            .on_toggle(move |is_enabled| {
                                on_change(FootprintStudy::Imbalance {
                                    threshold,
                                    color_scale: if is_enabled {
                                        Some(color_scale_value)
                                    } else {
                                        None
                                    },
                                    ignore_zeros,
                                    stack_count,
                                })
                            });

                        if color_scale_enabled {
                            let slider = column![
                                text(format!("Opaque color at: {color_scale_value}x")),
                                slider(50.0..=1_000_000.0, color_scale_value as f32, move |new_value| {
                                    on_change(FootprintStudy::Imbalance {
                                        threshold,
                                        color_scale: Some(new_value as usize),
                                        ignore_zeros,
                                        stack_count,
                                    })
                                })
                                .step(50.0)
                            ]
                            .spacing(2);

                             let input = crate::widget::numeric_input_box(
                                "Scale",
                                "0",
                                &state.color_scale_input,
                                state.color_scale_input.parse::<f64>().is_ok(),
                                move |s| {
                                    Message::TopLevel(
                                       crate::modal::pane::settings::Message::ColorScaleInputChanged(s)
                                    )
                                },
                                None,
                            );

                            column![color_scale_checkbox, slider, input]
                                .padding(8)
                                .spacing(8)
                        } else {
                            column![color_scale_checkbox].padding(8)
                        }
                    };

                    let ignore_zeros_checkbox = {
                        let cbox = checkbox(ignore_zeros).label("Ignore zeros").on_toggle(
                            move |is_checked| {
                                on_change(FootprintStudy::Imbalance {
                                    threshold,
                                    color_scale,
                                    ignore_zeros: is_checked,
                                    stack_count,
                                })
                            },
                        );

                        column![cbox].padding(8).spacing(4)
                    };

                    let stack_count_slider = {
                        let info_text = text(format!("Stack count: {stack_count}"));
                        let slider_ui = slider(2.0..=10.0, stack_count as f32, move |new_value| {
                            on_change(FootprintStudy::Imbalance {
                                threshold,
                                color_scale,
                                ignore_zeros,
                                stack_count: new_value as usize,
                            })
                        })
                        .step(1.0);
                        column![info_text, slider_ui].padding(8).spacing(4)
                    };

                    split_column![qty_threshold, color_scaling, ignore_zeros_checkbox, stack_count_slider]
                        .padding(4)
                        .into()
                }
                FootprintStudy::SummaryTable {
                    show_delta,
                    show_max_min,
                    show_volume,
                } => {
                    let delta_checkbox = checkbox(show_delta).label("Show Delta").on_toggle(
                        move |v| {
                            on_change(FootprintStudy::SummaryTable {
                                show_delta: v,
                                show_max_min,
                                show_volume,
                            })
                        },
                    );

                    let max_min_checkbox = checkbox(show_max_min)
                        .label("Show Max/Min Delta")
                        .on_toggle(move |v| {
                            on_change(FootprintStudy::SummaryTable {
                                show_delta,
                                show_max_min: v,
                                show_volume,
                            })
                        });

                    let volume_checkbox = checkbox(show_volume)
                        .label("Show Volume")
                        .on_toggle(move |v| {
                            on_change(FootprintStudy::SummaryTable {
                                show_delta,
                                show_max_min,
                                show_volume: v,
                            })
                        });

                    column![delta_checkbox, max_min_checkbox, volume_checkbox]
                        .padding(8)
                        .spacing(8)
                        .into()
                }
                FootprintStudy::VolumeProfile {
                    show_numbers,
                    bar_width_pct,
                    show_delta_color,
                } => {
                    let numbers_checkbox = checkbox(show_numbers).label("Show values").on_toggle(
                        move |v| {
                            on_change(FootprintStudy::VolumeProfile {
                                show_numbers: v,
                                bar_width_pct,
                                show_delta_color,
                            })
                        },
                    );

                    let delta_checkbox = checkbox(show_delta_color).label("Delta coloring").on_toggle(
                        move |v| {
                            on_change(FootprintStudy::VolumeProfile {
                                show_numbers,
                                bar_width_pct,
                                show_delta_color: v,
                            })
                        },
                    );

                    let width_slider = {
                        let info_text = text(format!("Width: {}%", (bar_width_pct * 100.0) as u32));
                        let slider_ui = slider(0.1..=1.0, bar_width_pct, move |new_value| {
                            on_change(FootprintStudy::VolumeProfile {
                                show_numbers,
                                bar_width_pct: new_value,
                                show_delta_color,
                            })
                        })
                        .step(0.05);

                        column![info_text, slider_ui].spacing(4)
                    };

                    column![numbers_checkbox, delta_checkbox, width_slider]
                        .padding(8)
                        .spacing(8)
                        .into()
                }
            }
        }
    }

    impl Study for HeatmapStudy {
        fn is_same_type(&self, other: &Self) -> bool {
            std::mem::discriminant(self) == std::mem::discriminant(other)
        }

        fn all() -> Vec<Self> {
            HeatmapStudy::ALL.to_vec()
        }

        fn view_config<'a>(
            &self,
            basis: data::chart::Basis,
            _state: &'a crate::modal::pane::settings::State,
            _on_change: impl Fn(Self) -> Message<Self> + Copy + 'a,
        ) -> Element<'a, Message<Self>> {
            let _interval_ms = match basis {
                data::chart::Basis::Time(interval) => interval.to_milliseconds(),
                data::chart::Basis::Tick(_) => {
                    return iced::widget::center(text(
                        "Heatmap studies are not supported for tick-based charts",
                    ))
                    .into();
                }
            };

            match *self {}
        }
    }

    #[derive(Debug, Clone)]
    pub enum Message<S: Study> {
        CardToggled(S),
        StudyToggled(S, bool),
        StudyValueChanged(S),
        TopLevel(super::Message), // variant to bubble up
    }

    pub enum Action<S: Study> {
        ToggleStudy(S, bool),
        ConfigureStudy(S),
    }

    pub struct Configurator<S: Study> {
        expanded_card: Option<S>,
    }

    impl<S: Study> Default for Configurator<S> {
        fn default() -> Self {
            Self {
                expanded_card: None,
            }
        }
    }

    impl<S: Study + ToString> Configurator<S> {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn update(&mut self, message: Message<S>) -> Option<Action<S>> {
            match message {
                Message::CardToggled(study) => {
                    let should_collapse = self
                        .expanded_card
                        .as_ref()
                        .is_some_and(|expanded| expanded.is_same_type(&study));

                    if should_collapse {
                        self.expanded_card = None;
                    } else {
                        self.expanded_card = Some(study);
                    }
                    None
                }
                Message::StudyToggled(study, is_checked) => Some(Action::ToggleStudy(study, is_checked)),
                Message::StudyValueChanged(study) => Some(Action::ConfigureStudy(study)),
                Message::TopLevel(_) => None,
            }
        }

        pub fn view<'a>(
            &self,
            active_studies: &'a [S],
            basis: data::chart::Basis,
            state: &'a crate::modal::pane::settings::State,
        ) -> Element<'a, Message<S>> {
            let mut content = column![].spacing(4);

            for available_study in S::all() {
                content =
                    content.push(self.create_study_row(available_study, active_studies, basis, state));
            }

            content.into()
        }

        fn create_study_row<'a>(
            &self,
            study: S,
            active_studies: &'a [S],
            basis: data::chart::Basis,
            state: &'a crate::modal::pane::settings::State,
        ) -> Element<'a, Message<S>> {
            let (is_selected, study_config) = {
                let mut is_selected = false;
                let mut study_config = None;

                for s in active_studies {
                    if s.is_same_type(&study) {
                        is_selected = true;
                        study_config = Some(*s);
                        break;
                    }
                }
                (is_selected, study_config)
            };

            let checkbox = checkbox(is_selected)
                .label(study_config.map_or(study.to_string(), |s| s.to_string()))
                .on_toggle(move |checked| Message::StudyToggled(study, checked));

            let mut checkbox_row = row![checkbox, space::horizontal()]
                .height(36)
                .align_y(iced::Alignment::Center)
                .padding(padding::left(8).right(4))
                .spacing(4);

            let is_expanded = self
                .expanded_card
                .as_ref()
                .is_some_and(|expanded| expanded.is_same_type(&study));

            if is_selected {
                checkbox_row = checkbox_row.push(
                    button(icon_text(Icon::Cog, 12))
                        .on_press(Message::CardToggled(study))
                        .style(move |theme, status| {
                            style::button::transparent(theme, status, is_expanded)
                        }),
                );
            }

            let mut column = column![checkbox_row];

            if is_expanded && let Some(config) = study_config {
                column = column.push(config.view_config(basis, state, |s| Message::StudyValueChanged(s)));
            }

            container(column).style(style::modal_container).into()
        }
    }
}
