//! UI rendering: 3x4 grid, profile panel, calibration, capture.

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, toggler,
};
use iced::{Alignment, Color, Element, Length, Padding};

use trinity_app::ipc::{ProfileDto, Status};

use rust_i18n::t;

use crate::icons::{self, MouseAction};
use crate::settings;
use crate::theme;
use crate::{App, Message, Route, WindowLayout};

pub fn render(app: &App) -> Element<'_, Message> {
    let body = match app.route {
        Route::Settings => settings_page(app),
        Route::Main => match app.connected {
            None | Some(false) => disconnected(app),
            Some(true) => match app.status.as_ref() {
                Some(status) if status.calibrating => calibration(app, status),
                Some(status) => main_view(app, status),
                None => column![
                    text(t!("status.connecting").to_string())
                        .size(18)
                        .color(theme::TEXT_DIM)
                ]
                .spacing(12)
                .into(),
            },
        },
    };

    let mut content = column![].spacing(14);
    content = content.push(scrollable(body).width(Length::Fill).height(Length::Fill));
    if app.route == Route::Main {
        content = content.push(footer());
    }

    let scene = container(content.max_width(1020.0).width(Length::Fill))
        .padding(iced::Padding {
            top: 14.0,
            bottom: 20.0,
            left: 20.0,
            right: 20.0,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill);

    let root = column![title_bar(app), scene]
        .width(Length::Fill)
        .height(Length::Fill);
    let mut layered = iced::widget::Stack::new().push(root);
    if let Some(notice) = app.notice.as_ref() {
        layered = layered.push(error_toast_layer(&notice.text));
    }
    container(layered)
        .style(theme::app_background_for(app.csd, app.maximized))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Floating error toast: anchored bottom-right, overlaid on the
/// content, does not intercept clicks (non-interactive container).
fn error_toast_layer(message: &str) -> Element<'_, Message> {
    container(
        container(
            row![
                text("⚠").size(13).color(theme::DANGER),
                text(message.to_owned())
                    .size(13)
                    .color(theme::TEXT)
                    .wrapping(iced::widget::text::Wrapping::Word),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .style(theme::error_toast)
        .padding(Padding::new(10.0).horizontal(14.0))
        .max_width(420.0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::End)
    .align_y(Alignment::End)
    .padding(16.0)
    .into()
}

fn title_bar(app: &App) -> Element<'_, Message> {
    let title = column![
        text("TRINITY MAPPER")
            .size(21)
            .font(semibold())
            .color(theme::TEXT),
        text(t!("titlebar.subtitle").to_string())
            .size(12)
            .color(theme::TEXT_DIM),
    ]
    .spacing(2);

    let mut pills = row![].spacing(8).align_y(Alignment::Center);
    if let Some(status) = app.status.as_ref() {
        pills = pills
            .push(pill(
                "●",
                if status.device_present {
                    theme::ACCENT
                } else {
                    theme::DANGER
                },
                if status.device_present {
                    t!("titlebar.device_present").to_string()
                } else {
                    t!("titlebar.device_absent").to_string()
                },
            ))
            .push(pill(
                "●",
                if status.calibrated {
                    theme::ACCENT
                } else {
                    theme::WARNING
                },
                if status.calibrated {
                    t!("titlebar.calibrated").to_string()
                } else {
                    t!("titlebar.calibration_needed").to_string()
                },
            ));
    }

    // Drag zone: title + pills + space up to the controls. Interactive
    // widgets (toggle, buttons) stay out of the zone. Outside Wayland
    // (system decorations), this is a plain header.
    let zone = row![title, pills]
        .spacing(16)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let draggable: Element<'_, Message> = if app.csd {
        mouse_area(zone)
            .on_press(Message::DragWindow)
            .on_double_click(Message::ToggleMaximize)
            .into()
    } else {
        zone.into()
    };

    let enabled = app.status.as_ref().is_some_and(|status| status.enabled);
    let switch = row![
        text(t!("titlebar.active").to_string())
            .size(13)
            .color(theme::TEXT_DIM),
        toggler(enabled)
            .on_toggle(Message::ToggleEnabled)
            .style(theme::toggle),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut bar = row![
        container(draggable).width(Length::Fill),
        switch,
        title_button("⚙", Message::ToggleSettings, false),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if app.csd {
        bar = bar.push(window_controls());
    }

    container(bar)
        .padding(iced::Padding {
            top: 10.0,
            bottom: 10.0,
            left: 16.0,
            right: 12.0,
        })
        .width(Length::Fill)
        .into()
}

fn window_controls() -> Element<'static, Message> {
    row![
        title_button("—", Message::MinimizeWindow, false),
        title_button("□", Message::ToggleMaximize, false),
        title_button("×", Message::CloseWindow, true),
    ]
    .spacing(2)
    .into()
}

fn title_button(glyph: &'static str, message: Message, danger: bool) -> Element<'static, Message> {
    button(text(glyph).size(14).font(semibold()))
        .on_press(message)
        .style(if danger {
            theme::title_close
        } else {
            theme::title_button
        })
        .padding(Padding::new(6.0).horizontal(12.0))
        .into()
}

fn semibold() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name(theme::FONT_NAME),
        weight: iced::font::Weight::Semibold,
        ..iced::Font::DEFAULT
    }
}

fn bold() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name(theme::FONT_NAME),
        weight: iced::font::Weight::Bold,
        ..iced::Font::DEFAULT
    }
}

fn pill(dot: &str, dot_color: Color, label: String) -> Element<'_, Message> {
    container(
        row![
            text(dot).size(11).color(dot_color),
            text(label).size(12).color(theme::TEXT_DIM),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(theme::pill)
    .padding(Padding::new(6.0).horizontal(12.0))
    .into()
}

fn footer() -> Element<'static, Message> {
    fn hint(icon: Element<'static, Message>, label: String) -> Element<'static, Message> {
        row![icon, text(label).size(11).color(theme::TEXT_DIM)]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }

    container(
        row![
            hint(
                icons::mouse_icon(MouseAction::Left, theme::ACCENT, 17.0),
                t!("footer.assign").to_string(),
            ),
            hint(
                icons::mouse_icon(MouseAction::Right, theme::DANGER, 17.0),
                t!("footer.clear").to_string(),
            ),
            hint(
                keycap(t!("keys.esc").to_string()),
                t!("footer.cancel").to_string(),
            ),
        ]
        .spacing(18)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}

/// Key rendered as a keycap (GNOME <kbd> style).
fn keycap(label: String) -> Element<'static, Message> {
    container(text(label).size(11).color(theme::TEXT))
        .padding(Padding::new(3.0).horizontal(7.0))
        .style(|_theme| iced::widget::container::Style {
            background: Some(theme::SURFACE_HI.into()),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

fn disconnected(app: &App) -> Element<'_, Message> {
    centered_card(
        column![
            text(t!("status.daemon_unreachable").to_string())
                .size(20)
                .font(semibold())
                .color(theme::TEXT),
            text(t!("status.socket", path = app.socket.display().to_string()).to_string())
                .size(12)
                .color(theme::TEXT_DIM),
            text(t!("status.daemon_hint").to_string())
                .size(14)
                .color(theme::TEXT_DIM),
            button(
                text(t!("status.retry").to_string())
                    .size(14)
                    .font(semibold())
                    .color(Color::BLACK),
            )
            .on_press(Message::Refresh)
            .style(theme::primary)
            .padding(Padding::new(10.0).horizontal(22.0)),
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
}

fn calibration<'a>(app: &'a App, status: &'a Status) -> Element<'a, Message> {
    let target = status.current_button.unwrap_or(0);
    let segments = (0..status.total_buttons)
        .map(|index| {
            let done = index < status.captured_count;
            let is_next = index == status.captured_count && target != 0;
            let width = if done || is_next { 22.0 } else { 14.0 };
            container(iced::widget::Space::new().width(Length::Fixed(width)))
                .height(Length::Fixed(6.0))
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(
                        if done || (is_next && app.pulse) {
                            theme::ACCENT
                        } else if is_next {
                            theme::ACCENT_DARK
                        } else {
                            theme::BORDER
                        }
                        .into(),
                    ),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..iced::Border::default()
                    },
                    ..iced::widget::container::Style::default()
                })
                .into()
        })
        .collect::<Vec<Element<'_, Message>>>();

    centered_card(
        column![
            text(t!("calibration.title").to_string())
                .size(13)
                .font(bold())
                .color(theme::ACCENT),
            text(if target == 0 {
                t!("calibration.finishing").to_string()
            } else {
                t!("calibration.prompt", button = target).to_string()
            })
            .size(34)
            .font(semibold())
            .color(theme::TEXT),
            row(segments).spacing(6),
            row![
                icons::mouse_icon(MouseAction::Left, theme::ACCENT, 15.0),
                icons::mouse_icon(MouseAction::Right, theme::DANGER, 15.0),
                text(
                    t!(
                        "calibration.progress",
                        captured = status.captured_count,
                        total = status.total_buttons
                    )
                    .to_string(),
                )
                .size(12)
                .color(theme::TEXT_DIM),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            button(
                text(t!("calibration.cancel").to_string())
                    .size(13)
                    .color(theme::DANGER),
            )
            .on_press(Message::CancelCalibration)
            .style(theme::danger)
            .padding(Padding::new(8.0).horizontal(18.0)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
}

fn main_view<'a>(app: &'a App, status: &'a Status) -> Element<'a, Message> {
    if let Some(button) = app.editing {
        return capture_overlay(app, button);
    }
    let grid = container(grid(app))
        .style(theme::card)
        .padding(22.0)
        .width(Length::Fill)
        .center_x(Length::Fill);
    let side = side_panel(app, status, app.window_width);
    match crate::layout_for(app.window_width) {
        WindowLayout::Wide => row![grid, side]
            .spacing(18)
            .align_y(Alignment::Start)
            .into(),
        WindowLayout::Narrow => column![grid, side].spacing(18).into(),
    }
}

fn capture_overlay(app: &App, button: u8) -> Element<'_, Message> {
    container(centered_card(
        column![
            text(t!("capture.title").to_string())
                .size(13)
                .font(bold())
                .color(theme::ACCENT),
            text(button.to_string())
                .size(56)
                .font(bold())
                .color(if app.pulse {
                    theme::ACCENT
                } else {
                    theme::TEXT
                }),
            text(t!("capture.prompt").to_string())
                .size(16)
                .color(theme::TEXT),
            row![
                keycap(t!("keys.ctrl").to_string()),
                keycap(t!("keys.shift").to_string()),
                keycap(t!("keys.alt").to_string()),
                keycap(t!("keys.logo").to_string()),
                text(t!("capture.modifiers").to_string())
                    .size(12)
                    .color(theme::TEXT_DIM),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text(t!("capture.hint").to_string())
                .size(12)
                .color(theme::TEXT_DIM),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    ))
    .style(theme::overlay)
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

fn centered_card<'a>(
    content: impl Into<iced::widget::Column<'a, Message>>,
) -> Element<'a, Message> {
    container(
        container(content.into().spacing(6))
            .style(theme::card)
            .padding(Padding::new(28.0).horizontal(44.0)),
    )
    .style(theme::overlay)
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

fn grid(app: &App) -> Element<'_, Message> {
    let rows = (1u8..=12)
        .step_by(3)
        .map(|first| {
            let cells = (first..first + 3).map(|number| grid_cell(app, number));
            row(cells).spacing(10).width(Length::Fill).into()
        })
        .collect::<Vec<Element<'_, Message>>>();
    column(rows)
        .spacing(10)
        .width(Length::Fill)
        .max_width(440.0)
        .into()
}

fn grid_cell<'a>(app: &'a App, number: u8) -> Element<'a, Message> {
    let status = app.status.as_ref();
    let calibrating_next = status
        .map(|status| status.calibrating && status.current_button == Some(number))
        .unwrap_or(false);
    let label = app.loaded_profile.as_ref().map_or_else(
        || "—".to_owned(),
        |profile| cell_label(app, profile, number),
    );
    let mapped = label != "—";

    let content = column![
        text(number.to_string()).size(10).color(if mapped {
            theme::ACCENT
        } else {
            theme::TEXT_DIM
        }),
        text(label.clone())
            .size(18)
            .font(if mapped {
                semibold()
            } else {
                iced::Font::DEFAULT
            })
            .color(if mapped { theme::TEXT } else { theme::TEXT_DIM }),
    ]
    .spacing(5)
    .align_x(Alignment::Center);

    let cell = button(content)
        .on_press(Message::EditButton(number))
        .padding(Padding::new(12.0))
        .width(Length::Fixed(118.0))
        .height(Length::Fixed(84.0))
        .style(theme::key_cell(theme::KeyCell {
            mapped,
            highlighted: app.editing == Some(number) || calibrating_next,
            dimmed: app.editing.is_some_and(|editing| editing != number),
            pulse: (app.editing == Some(number) || calibrating_next) && app.pulse,
        }));

    mouse_area(cell)
        .on_right_press(Message::ClearButton(number))
        .into()
}

fn cell_label(app: &App, profile: &ProfileDto, number: u8) -> String {
    profile
        .buttons
        .iter()
        .find(|mapping| mapping.button == number)
        .map(|mapping| {
            let modifiers: Vec<String> = mapping
                .modifiers
                .iter()
                .map(|name| {
                    trinity_core::key::KeyCode::from_name(name)
                        .map(|code| app.key_label(code))
                        .unwrap_or_else(|_| name.clone())
                })
                .collect();
            let key = trinity_core::key::KeyCode::from_name(&mapping.key)
                .map(|code| app.key_label(code))
                .unwrap_or_else(|_| mapping.key.clone());
            let mut parts = modifiers;
            parts.push(key);
            parts.join("+")
        })
        .unwrap_or_else(|| "—".to_owned())
}

fn side_panel<'a>(app: &'a App, status: &'a Status, window_width: f32) -> Element<'a, Message> {
    let panel_width = match crate::layout_for(window_width) {
        WindowLayout::Wide => Length::Fixed(300.0),
        WindowLayout::Narrow => Length::Fill,
    };
    let mut panel = column![].spacing(12).width(panel_width);

    if !status.calibrated {
        panel = panel.push(
            column![
                text(t!("panel.first_use_title").to_string())
                    .size(15)
                    .font(semibold())
                    .color(theme::TEXT),
                text(t!("panel.first_use_text").to_string())
                    .size(12)
                    .color(theme::TEXT_DIM),
                button(
                    text(t!("panel.start_calibration").to_string())
                        .size(13)
                        .font(semibold())
                        .color(Color::BLACK),
                )
                .on_press(Message::StartCalibration)
                .style(theme::primary)
                .padding(Padding::new(10.0).horizontal(16.0)),
            ]
            .spacing(8),
        );
    }

    let active = app
        .loaded_profile
        .as_ref()
        .map(|profile| profile.name.clone());
    let mut profiles = column![].spacing(6);
    for name in &app.profiles {
        let is_active = active.as_deref() == Some(name.as_str());
        let entry = button(
            row![
                text(if is_active { "●" } else { "○" })
                    .size(11)
                    .color(if is_active {
                        theme::ACCENT
                    } else {
                        theme::TEXT_DIM
                    }),
                text(name.clone()).size(13).color(if is_active {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectProfile(name.clone()))
        .padding(Padding::new(9.0).horizontal(12.0))
        .width(Length::Fill)
        .style(theme::profile_entry(is_active));
        profiles = profiles.push(entry);
    }

    let profile_list: Element<'_, Message> = if app.profiles.is_empty() {
        text(t!("panel.no_profiles").to_string())
            .size(12)
            .color(theme::TEXT_DIM)
            .into()
    } else {
        scrollable(profiles)
            .height(Length::Fixed(260.0))
            .width(Length::Fill)
            .into()
    };

    panel = panel.push(
        column![
            text(t!("panel.profiles").to_string())
                .size(15)
                .font(semibold())
                .color(theme::TEXT),
            profile_list,
            row![
                text_input(t!("panel.new_profile").as_ref(), &app.new_profile)
                    .on_input(Message::NewProfileName)
                    .on_submit(Message::CreateProfile)
                    .size(13)
                    .style(theme::input)
                    .width(Length::Fill),
                button(
                    text(t!("panel.create").to_string())
                        .size(12)
                        .color(Color::BLACK)
                )
                .on_press(Message::CreateProfile)
                .style(theme::primary)
                .padding(Padding::new(9.0).horizontal(14.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    );

    panel = panel.push(
        column![
            text(t!("panel.maintenance").to_string())
                .size(15)
                .font(semibold())
                .color(theme::TEXT),
            button(
                text(t!("panel.recalibrate").to_string())
                    .size(12)
                    .color(theme::TEXT),
            )
            .on_press(Message::StartCalibration)
            .style(theme::secondary)
            .padding(Padding::new(8.0).horizontal(14.0)),
        ]
        .spacing(8),
    );

    if let Some(error) = status.error.as_deref() {
        panel = panel.push(
            container(
                column![
                    text(t!("panel.daemon_error").to_string())
                        .size(11)
                        .color(theme::DANGER),
                    text(error).size(11).color(theme::TEXT_DIM),
                ]
                .spacing(4),
            )
            .style(theme::error_toast)
            .padding(10.0),
        );
    }

    container(panel).style(theme::card).padding(20.0).into()
}

fn settings_card<'a>(
    title: String,
    content: impl Into<iced::widget::Column<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(15).font(semibold()).color(theme::TEXT),
            content.into(),
        ]
        .spacing(10),
    )
    .style(theme::card)
    .padding(20.0)
    .width(Length::Fill)
    .into()
}

fn diag_row(label: String, value: String, danger: bool) -> Element<'static, Message> {
    row![
        text(label)
            .size(12)
            .color(theme::TEXT_DIM)
            .width(Length::Fixed(140.0)),
        text(value)
            .size(12)
            .color(if danger { theme::DANGER } else { theme::TEXT }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn settings_page(app: &App) -> Element<'_, Message> {
    let mut page = column![].spacing(16).width(Length::Fill);

    let mut languages = column![].spacing(6);
    let current = app.settings.effective_locale();
    for &locale in settings::LOCALES {
        let active = current == locale;
        languages = languages.push(
            button(
                row![
                    text(if active { "●" } else { "○" })
                        .size(11)
                        .color(if active {
                            theme::ACCENT
                        } else {
                            theme::TEXT_DIM
                        }),
                    text(settings::native_name(locale))
                        .size(13)
                        .color(if active { theme::TEXT } else { theme::TEXT_DIM }),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(Message::SelectLocale(locale.to_owned()))
            .padding(Padding::new(9.0).horizontal(12.0))
            .width(Length::Fill)
            .style(theme::profile_entry(active)),
        );
    }
    page = page.push(settings_card(
        t!("settings.language").to_string(),
        languages,
    ));

    let status = app.status.as_ref();
    let daemon_state = match app.connected {
        Some(true) => t!("settings.connected").to_string(),
        _ => t!("settings.disconnected").to_string(),
    };
    let device_state = match status.map(|status| status.device_present) {
        Some(true) => t!("settings.present").to_string(),
        Some(false) => t!("settings.absent").to_string(),
        None => "—".to_owned(),
    };
    let calibration_state = match status.map(|status| status.calibrated) {
        Some(true) => t!("settings.calibrated").to_string(),
        _ => t!("settings.required").to_string(),
    };
    let profile = status
        .and_then(|status| status.profile.clone())
        .unwrap_or_else(|| t!("settings.none").to_string());
    let (error, has_error) = match status.and_then(|status| status.error.clone()) {
        Some(error) => (error, true),
        None => (t!("settings.no_error").to_string(), false),
    };

    let diagnostics = column![
        diag_row(
            t!("settings.socket").to_string(),
            app.socket.display().to_string(),
            false,
        ),
        diag_row(t!("settings.daemon").to_string(), daemon_state, false),
        diag_row(t!("settings.device").to_string(), device_state, false),
        diag_row(
            t!("settings.calibration").to_string(),
            calibration_state,
            false,
        ),
        diag_row(t!("settings.active_profile").to_string(), profile, false),
        diag_row(t!("settings.last_error").to_string(), error, has_error),
    ]
    .spacing(8);
    page = page.push(settings_card(
        t!("settings.diagnostics").to_string(),
        diagnostics,
    ));

    let about = column![
        row![
            text("TRINITY MAPPER")
                .size(20)
                .font(bold())
                .color(theme::TEXT),
            text(env!("CARGO_PKG_VERSION").to_owned())
                .size(12)
                .color(theme::ACCENT),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        text(t!("settings.tagline").to_string())
            .size(12)
            .color(theme::TEXT_DIM),
        diag_row(
            t!("settings.license_label").to_string(),
            "GPL-3.0".to_owned(),
            false,
        ),
        row![
            text(t!("settings.project_label").to_string())
                .size(12)
                .color(theme::TEXT_DIM)
                .width(Length::Fixed(140.0)),
            text(settings::PROJECT_URL.to_owned())
                .size(12)
                .color(theme::TEXT),
            button(
                text(t!("settings.copy").to_string())
                    .size(11)
                    .color(theme::TEXT)
            )
            .on_press(Message::CopyProjectUrl)
            .style(theme::secondary)
            .padding(Padding::new(5.0).horizontal(10.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        diag_row(
            t!("settings.icons_label").to_string(),
            "Material Design Icons (Apache-2.0)".to_owned(),
            false,
        ),
        diag_row(
            t!("settings.font_label").to_string(),
            "Inter (SIL OFL 1.1)".to_owned(),
            false,
        ),
        text(t!("settings.translation_note").to_string())
            .size(11)
            .color(theme::TEXT_DIM),
    ]
    .spacing(8);
    page = page.push(settings_card(t!("settings.about").to_string(), about));

    page.into()
}
