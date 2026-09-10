//! UI rendering: 3x4 grid, profile panel, calibration, capture.

use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_input, toggler,
};
use iced::{Alignment, Color, Element, Length, Padding};

use trinity_app::ipc::{ProfileDto, Status};

use rust_i18n::t;

use crate::icons::{self, MouseAction};
use crate::settings;
use crate::theme;
use crate::{App, Message, Route};

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
    let body_area: Element<'_, Message> = if app.route == Route::Settings {
        scrollable(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::minimal_scrollbar)
            .into()
    } else {
        // Main view: centered without scrollable (no scrollbar offset).
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    };
    content = content.push(body_area);
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
    if app.manage_open {
        layered = layered.push(manage_profiles_popup(app));
    }
    container(layered)
        .style(theme::app_background_for(app.csd, app.maximized))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Floating error toast: anchored bottom-right, overlaid on the
/// content, does not intercept clicks (non-interactive container).
/// Delete-confirmation popup: dark overlay + centered card.
/// Manage-profiles popup: list with rename/delete + add input + close.
fn manage_profiles_popup(app: &App) -> Element<'_, Message> {
    let list = profile_list(app);
    let can_add = !app.manage_input.trim().is_empty();
    let add_row = row![
        text_input(t!("toolbar.add_placeholder").as_ref(), &app.manage_input)
            .on_input(Message::ManageProfileInput)
            .on_submit(Message::ManageAddProfile)
            .size(13)
            .style(theme::input)
            .width(Length::Fill),
        button(
            text(t!("toolbar.add").to_string())
                .size(12)
                .color(Color::BLACK),
        )
        .on_press_maybe(can_add.then_some(Message::ManageAddProfile))
        .style(theme::primary)
        .padding(Padding::new(7.0).horizontal(14.0)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let title_row = row![
        text(t!("toolbar.manage_title").to_string())
            .size(17)
            .font(semibold())
            .color(theme::TEXT)
            .width(Length::Fill),
        button(text("✕").size(14).color(theme::TEXT_DIM))
            .on_press(Message::HideManageProfiles)
            .style(theme::delete_button)
            .padding(6.0),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let separator = container(
        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(1.0)),
    )
    .style(|_t| iced::widget::container::Style {
        background: Some(theme::BORDER.into()),
        ..iced::widget::container::Style::default()
    });

    let card_content = if let Some(name) = app.confirm_delete.as_ref() {
        column![
            text(t!("panel.confirm_delete_title").to_string())
                .size(17)
                .font(semibold())
                .color(theme::TEXT),
            text(t!("panel.confirm_delete_body", name = name.to_string()).to_string(),)
                .size(13)
                .color(theme::TEXT_DIM),
            row![
                button(
                    text(t!("calibration.cancel").to_string())
                        .size(13)
                        .color(theme::TEXT),
                )
                .on_press(Message::CancelDeleteProfile)
                .style(theme::secondary)
                .padding(Padding::new(8.0).horizontal(16.0)),
                button(
                    text(t!("panel.delete").to_string())
                        .size(13)
                        .color(theme::DANGER),
                )
                .on_press(Message::ConfirmDeleteProfile)
                .style(theme::danger)
                .padding(Padding::new(8.0).horizontal(16.0)),
            ]
            .spacing(10),
        ]
        .spacing(14)
        .width(Length::Fill)
    } else {
        column![title_row, list, separator, add_row,]
            .spacing(14)
            .width(Length::Fill)
    };

    // Card wrapped in its own mouse_area (Noop) so clicks on the card
    // itself don't bubble up to the overlay and close the popup.
    let card = mouse_area(
        container(card_content)
            .style(theme::card)
            .padding(Padding::new(24.0).horizontal(28.0))
            .width(Length::Fixed(380.0)),
    )
    .on_press(Message::Noop);

    let overlay = container(card)
        .style(theme::overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill);

    mouse_area(overlay)
        .on_press(Message::HideManageProfiles)
        .into()
}

fn profile_list(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);

    // Only manageable profiles — "default" is not CRUD-able, it lives in the selectbox.
    let manageable: Vec<&String> = app
        .profiles
        .iter()
        .filter(|name| name.as_str() != "default")
        .collect();

    if manageable.is_empty() {
        return text(t!("panel.no_profiles").to_string())
            .size(12)
            .color(theme::TEXT_DIM)
            .into();
    }

    for name in manageable {
        let is_renaming = app.rename_from.as_deref() == Some(name.as_str());

        let name_content: Element<'_, Message> = if is_renaming {
            text_input("", &app.rename_input)
                .on_input(Message::RenameInput)
                .on_submit(Message::ConfirmRename)
                .size(13)
                .style(theme::input)
                .width(Length::Fill)
                .into()
        } else {
            button(
                text(name.clone())
                    .size(13)
                    .color(theme::TEXT)
                    .width(Length::Fill),
            )
            .on_press(Message::StartRename(name.clone()))
            .style(theme::ghost_button)
            .padding(Padding::new(2.0).horizontal(4.0))
            .width(Length::Fill)
            .into()
        };

        let actions: Element<'_, Message> = if is_renaming {
            row![
                button(text("✓").size(13).color(theme::ACCENT))
                    .on_press(Message::ConfirmRename)
                    .style(theme::copy_button)
                    .padding(6.0),
                button(text("✕").size(13).color(theme::DANGER))
                    .on_press(Message::CancelRenameAction)
                    .style(theme::delete_button)
                    .padding(6.0),
            ]
            .spacing(4)
            .into()
        } else {
            button(text("✕").size(13).color(theme::TEXT_DIM))
                .on_press(Message::RequestDeleteProfile(name.clone()))
                .style(theme::delete_button)
                .padding(6.0)
                .into()
        };

        let entry = container(
            row![name_content, actions]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .style(|_t| iced::widget::container::Style {
            background: Some(theme::SURFACE.into()),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .padding(Padding::new(4.0).horizontal(12.0))
        .height(Length::Fixed(38.0))
        .width(Length::Fill);

        list = list.push(entry);
    }

    list.into()
}

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

/// Responsive sizes derived from window width.
#[derive(Debug, Clone, Copy)]
pub struct ResponsiveSizes {
    pub grid_max: f32,
    pub cell_height: f32,
    pub grid_spacing: f32,
    pub toolbar_spacing: f32,
    pub cell_padding: f32,
}

/// Proportional layout: grid takes 60% of window width, clamped 420–700px.
/// Button height maintains a 0.6 aspect ratio; spacing scales with the grid.
pub fn responsive_sizes(window_width: f32) -> ResponsiveSizes {
    let grid_max = (window_width * 0.6).clamp(420.0, 700.0);
    let grid_spacing = (grid_max * 0.023).max(8.0);
    let cell_width = (grid_max - 2.0 * grid_spacing) / 3.0;
    let cell_height = cell_width * 0.6;
    let toolbar_spacing = (grid_max * 0.05).max(16.0);
    let cell_padding = (cell_width * 0.086).min(14.0);
    ResponsiveSizes {
        grid_max,
        cell_height,
        grid_spacing,
        toolbar_spacing,
        cell_padding,
    }
}

fn main_view<'a>(app: &'a App, status: &'a Status) -> Element<'a, Message> {
    if let Some(button) = app.editing {
        return capture_overlay(app, button);
    }
    if !status.calibrated {
        return column![
            text(t!("panel.first_use_text").to_string())
                .size(14)
                .color(theme::TEXT_DIM)
        ]
        .spacing(12)
        .into();
    }
    let sizes = responsive_sizes(app.window_width);
    let toolbar = profile_toolbar(app);
    let centered_grid = container(grid(app, sizes))
        .width(Length::Fill)
        .center_x(Length::Fill);
    let inner = container(
        column![toolbar, centered_grid]
            .spacing(sizes.toolbar_spacing)
            .width(Length::Fill)
            .align_x(Alignment::Center),
    )
    .max_width(sizes.grid_max)
    .width(Length::Fill);
    container(inner)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

/// Header bar: selectbox fills width, manage button anchored right.
fn profile_toolbar(app: &App) -> Element<'_, Message> {
    let selected = app
        .loaded_profile
        .as_ref()
        .map(|profile| profile.name.clone());

    let picker = pick_list(app.profiles.clone(), selected, Message::SelectProfile)
        .placeholder("—")
        .width(Length::Fill)
        .style(theme::picker);

    let manage = button(
        text(t!("toolbar.manage").to_string())
            .size(12)
            .color(theme::TEXT),
    )
    .on_press(Message::ShowManageProfiles)
    .style(theme::secondary)
    .padding(Padding::new(8.0).horizontal(14.0));

    row![picker, manage]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
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

fn grid(app: &App, sizes: ResponsiveSizes) -> Element<'_, Message> {
    let rows = (1u8..=12)
        .step_by(3)
        .map(|first| {
            let cells = (first..first + 3).map(|number| grid_cell(app, number, sizes));
            row(cells)
                .spacing(sizes.grid_spacing)
                .width(Length::Fill)
                .into()
        })
        .collect::<Vec<Element<'_, Message>>>();
    column(rows)
        .spacing(sizes.grid_spacing)
        .width(Length::Fill)
        .into()
}

fn grid_cell<'a>(app: &'a App, number: u8, sizes: ResponsiveSizes) -> Element<'a, Message> {
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
        .padding(Padding::new(sizes.cell_padding))
        .width(Length::Fill)
        .height(Length::Fixed(sizes.cell_height))
        .style(theme::key_cell(theme::KeyCell {
            mapped,
            highlighted: app.editing == Some(number) || calibrating_next,
            dimmed: app.editing.is_some_and(|editing| editing != number),
            pulse: (app.editing == Some(number) || calibrating_next) && app.pulse,
            blocked: app.manage_open || app.confirm_delete.is_some(),
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
    .padding(Padding::new(20.0).right(34.0))
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
        button(
            text(t!("settings.recalibrate").to_string())
                .size(12)
                .color(theme::TEXT),
        )
        .on_press(Message::StartCalibration)
        .style(theme::secondary)
        .padding(Padding::new(8.0).horizontal(14.0)),
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

    // Shortcuts section: one trinity-ctl command per profile
    let mut shortcuts = column![].spacing(8);
    for name in &app.profiles {
        let command = format!("trinity-ctl profile {name}");
        shortcuts = shortcuts.push(
            row![
                text(name.clone())
                    .size(12)
                    .color(theme::TEXT_DIM)
                    .width(Length::Fixed(100.0)),
                text(command.clone())
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(theme::TEXT)
                    .width(Length::Fill),
                button(icons::copy_icon(theme::TEXT_DIM, 13.0))
                    .on_press(Message::CopyShortcut(command))
                    .style(theme::copy_button)
                    .padding(5.0),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }
    page = page.push(settings_card(
        t!("settings.shortcuts").to_string(),
        shortcuts,
    ));

    // About last
    page = page.push(settings_card(t!("settings.about").to_string(), about));

    page.into()
}
