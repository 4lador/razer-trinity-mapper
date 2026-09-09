//! Mouse pictograms: embedded Material Design icons (Pictogrammers,
//! Apache-2.0), tinted at runtime — the clicked button reads through the
//! shape (cut out of the silhouette), a single color is enough.

use iced::widget::svg::{self, Handle};
use iced::{Color, Element, Length};

use crate::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAction {
    Left,
    Right,
}

const LEFT: &str = include_str!("../assets/icons/mouse-left.svg");
const RIGHT: &str = include_str!("../assets/icons/mouse-right.svg");

fn hex(color: Color) -> String {
    let channel = |value: f32| (value * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

/// Builds the tinted SVG for an action.
pub fn tinted(action: MouseAction, color: Color) -> Vec<u8> {
    let raw = match action {
        MouseAction::Left => LEFT,
        MouseAction::Right => RIGHT,
    };
    raw.replace("OUTLINE", &hex(color)).into_bytes()
}

/// Ready-to-use mouse icon widget.
pub fn mouse_icon(action: MouseAction, color: Color, size: f32) -> Element<'static, Message> {
    let handle = Handle::from_memory(tinted(action, color));
    svg::Svg::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_replaces_the_placeholder() {
        let content = String::from_utf8(tinted(MouseAction::Left, Color::WHITE)).unwrap();
        assert!(content.contains("#FFFFFF"), "icon not tinted: {content}");
        assert!(!content.contains("OUTLINE"));
    }

    #[test]
    fn variants_are_distinct_shapes() {
        let left = String::from_utf8(tinted(MouseAction::Left, Color::WHITE)).unwrap();
        let right = String::from_utf8(tinted(MouseAction::Right, Color::WHITE)).unwrap();
        assert_ne!(left, right);
    }
}
