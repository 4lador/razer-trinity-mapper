//! "Razer black & green" theme — palette and widget styles.

use iced::widget::{button, container, pick_list, scrollable};
use iced::{Border, Color};

pub const FONT_NAME: &str = "Inter";

pub const BG: Color = Color::from_rgb8(0x0F, 0x11, 0x15);
pub const SURFACE: Color = Color::from_rgb8(0x1A, 0x1D, 0x23);
pub const SURFACE_HI: Color = Color::from_rgb8(0x22, 0x26, 0x2E);
pub const BORDER: Color = Color::from_rgb8(0x2A, 0x2F, 0x3A);
pub const ACCENT: Color = Color::from_rgb8(0x44, 0xD6, 0x2C);
pub const ACCENT_DARK: Color = Color::from_rgb8(0x2A, 0x83, 0x1B);
pub const ACCENT_SOFT: Color = Color::from_rgba8(0x44, 0xD6, 0x2C, 0.14);
pub const TEXT: Color = Color::from_rgb8(0xE8, 0xEB, 0xF0);
pub const TEXT_DIM: Color = Color::from_rgb8(0x8B, 0x92, 0xA0);
pub const DANGER: Color = Color::from_rgb8(0xF3, 0x8B, 0xA8);
pub const DANGER_SOFT: Color = Color::from_rgba8(0xF3, 0x8B, 0xA8, 0.16);
pub const WARNING: Color = Color::from_rgb8(0xFA, 0xB3, 0x87);
pub const OVERLAY: Color = Color::from_rgba8(0x0A, 0x0C, 0x0F, 0.86);
/// Window outline hairline (libadwaita style): translucent white,
/// brighter than the surface but never read as a frame.
pub const WINDOW_OUTLINE: Color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.04);

fn solid(color: Color) -> button::Style {
    button::Style {
        background: Some(color.into()),
        text_color: TEXT,
        border: Border {
            radius: 12.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Window background: rounded rectangle with hairline in windowed CSD
/// mode, square when maximized, flat with system decorations (X11).
pub fn app_background_for(csd: bool, maximized: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |_theme| {
        let rounded = csd && !maximized;
        container::Style {
            background: Some(BG.into()),
            border: iced::Border {
                color: WINDOW_OUTLINE,
                width: if rounded { 1.0 } else { 0.0 },
                radius: if rounded { 12.0.into() } else { 0.0.into() },
            },
            ..container::Style::default()
        }
    }
}

pub fn card(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 16.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn pill(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE_HI.into()),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn overlay(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(OVERLAY.into()),
        ..container::Style::default()
    }
}

pub fn error_toast(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(DANGER_SOFT.into()),
        border: Border {
            color: DANGER,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn primary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => {
            let mut style = solid(Color::from_rgb8(0x58, 0xE8, 0x3F));
            style.text_color = Color::BLACK;
            style
        }
        button::Status::Pressed => {
            let mut style = solid(ACCENT_DARK);
            style.text_color = Color::BLACK;
            style
        }
        button::Status::Disabled => solid(SURFACE_HI),
        button::Status::Active => {
            let mut style = solid(ACCENT);
            style.text_color = Color::BLACK;
            style
        }
    }
}

pub fn secondary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(SURFACE_HI);
    match status {
        button::Status::Active => {
            style.border.color = BORDER;
        }
        button::Status::Hovered => {
            style.background = Some(SURFACE.into());
            style.border.color = ACCENT_DARK;
        }
        button::Status::Pressed => {
            style.border.color = ACCENT;
        }
        button::Status::Disabled => {}
    }
    style
}

pub fn danger(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(DANGER_SOFT);
    style.text_color = DANGER;
    if matches!(status, button::Status::Hovered) {
        style.border.color = DANGER;
    }
    style
}

/// Window control button (minimize/maximize/close): ghost style.
pub fn title_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.text_color = TEXT_DIM;
    style.border.radius = 8.0.into();
    match status {
        button::Status::Hovered => {
            style.background = Some(SURFACE_HI.into());
            style.text_color = TEXT;
        }
        button::Status::Pressed => {
            style.background = Some(SURFACE.into());
            style.text_color = TEXT;
        }
        _ => {}
    }
    style
}

/// Close button: danger highlight on hover.
pub fn title_close(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.text_color = TEXT_DIM;
    style.border.radius = 8.0.into();
    match status {
        button::Status::Hovered => {
            style.background = Some(DANGER.into());
            style.text_color = Color::BLACK;
        }
        button::Status::Pressed => {
            style.background = Some(DANGER.into());
            style.text_color = SURFACE;
        }
        _ => {}
    }
    style
}

/// Profile entry in the side panel.
pub fn profile_entry(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = solid(if is_active { SURFACE } else { SURFACE_HI });
        style.border.color = if is_active { ACCENT_DARK } else { BORDER };
        style.border.width = 1.0;
        if matches!(status, button::Status::Hovered) {
            style.border.color = ACCENT;
        }
        style
    }
}

/// Copy-to-clipboard button: ghost at rest, accent green on hover.
pub fn copy_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.border.radius = 6.0.into();
    if matches!(status, button::Status::Hovered) {
        style.background = Some(ACCENT_SOFT.into());
        style.border.color = ACCENT;
        style.border.width = 1.0;
    }
    style
}

/// Delete button: ghost at rest, danger red on hover.
pub fn delete_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.border.radius = 6.0.into();
    if matches!(status, button::Status::Hovered) {
        style.background = Some(DANGER_SOFT.into());
        style.border.color = DANGER;
        style.border.width = 1.0;
    }
    style
}

/// Ghost button: transparent, subtle surface on hover (for inline actions like profile rename).
pub fn ghost_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.text_color = TEXT;
    style.border.radius = 6.0.into();
    if matches!(status, button::Status::Hovered) {
        style.background = Some(SURFACE_HI.into());
    }
    style
}

/// Hyperlink-style button: accent color, underline on hover.
pub fn link_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = solid(Color::TRANSPARENT);
    style.text_color = ACCENT;
    style.border.radius = 4.0.into();
    if matches!(status, button::Status::Hovered) {
        style.border.color = ACCENT;
        style.border.width = 0.0;
        style.text_color = Color::from_rgb8(0x58, 0xE8, 0x3F);
    }
    style
}

/// Minimal scrollbar: invisible rail, thin translucent thumb.
/// Profile pick_list styled to match the theme.
pub fn picker(_theme: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list::Style {
        text_color: TEXT,
        placeholder_color: TEXT_DIM,
        handle_color: TEXT_DIM,
        background: SURFACE_HI.into(),
        border: iced::Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
    };
    if matches!(status, pick_list::Status::Hovered) {
        style.border.color = ACCENT_DARK;
    }
    if matches!(status, pick_list::Status::Opened { .. }) {
        style.border.color = ACCENT;
    }
    style
}

pub fn minimal_scrollbar(_theme: &iced::Theme, _status: scrollable::Status) -> scrollable::Style {
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.15).into(),
            border: Border::default(),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Color::TRANSPARENT.into(),
            border: Border::default(),
            shadow: iced::Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

/// Visual parameters of a grid key cell.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyCell {
    pub mapped: bool,
    pub highlighted: bool,
    pub dimmed: bool,
    pub pulse: bool,
    /// Suppresses hover styling — set when a popup overlay is open,
    /// preventing hover "bleed-through" from Stack layers.
    pub blocked: bool,
}

pub fn key_cell(cell: KeyCell) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = if cell.mapped {
            let mut style = solid(SURFACE);
            style.border.color = ACCENT_DARK;
            style.border.width = 1.0;
            style
        } else {
            let mut style = solid(SURFACE);
            style.border.color = BORDER;
            style.border.width = 1.0;
            style
        };
        if cell.highlighted || cell.pulse {
            style.background = Some(ACCENT_SOFT.into());
            style.border.color = ACCENT;
            style.border.width = if cell.pulse { 2.0 } else { 1.5 };
        }
        if cell.dimmed {
            style.background = Some(Color::from_rgba8(0x1A, 0x1D, 0x23, 0.45).into());
        }
        if !cell.blocked && matches!(status, button::Status::Hovered) {
            style.background = Some(ACCENT_SOFT.into());
            style.border.color = ACCENT;
            style.border.width = 1.5;
        }
        style
    }
}

pub fn toggle(
    _theme: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    let is_on = match status {
        iced::widget::toggler::Status::Active { is_toggled }
        | iced::widget::toggler::Status::Hovered { is_toggled }
        | iced::widget::toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    iced::widget::toggler::Style {
        background: (if is_on { ACCENT } else { SURFACE_HI }).into(),
        background_border_width: 1.0,
        background_border_color: if is_on { ACCENT } else { BORDER },
        foreground: (if is_on { Color::BLACK } else { TEXT_DIM }).into(),
        foreground_border_width: 0.0,
        foreground_border_color: Color::BLACK,
        text_color: Some(TEXT),
        border_radius: None,
        padding_ratio: 0.2,
    }
}

pub fn input(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let mut style = iced::widget::text_input::Style {
        background: SURFACE_HI.into(),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        icon: TEXT_DIM,
        placeholder: TEXT_DIM,
        value: TEXT,
        selection: ACCENT_DARK,
    };
    if matches!(status, iced::widget::text_input::Status::Focused { .. }) {
        style.border.color = ACCENT;
    }
    style
}
