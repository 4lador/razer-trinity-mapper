//! SVG icons: embedded, tinted at runtime. Mouse pictograms from
//! Material Design (Pictogrammers, Apache-2.0); copy icon in the style
//! of Lucide (MIT, Isaac Hunt).

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
const COPY: &str = include_str!("../assets/icons/copy.svg");
const TRASH: &str = include_str!("../assets/icons/trash.svg");

fn hex(color: Color) -> String {
    let channel = |value: f32| (value * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

/// Tints an SVG by replacing the `OUTLINE` color placeholder.
fn tint_svg(raw: &str, color: Color) -> Vec<u8> {
    raw.replace("OUTLINE", &hex(color)).into_bytes()
}

/// Builds the tinted SVG for a mouse action.
pub fn tinted(action: MouseAction, color: Color) -> Vec<u8> {
    let raw = match action {
        MouseAction::Left => LEFT,
        MouseAction::Right => RIGHT,
    };
    tint_svg(raw, color)
}

/// Ready-to-use mouse icon widget.
pub fn mouse_icon(action: MouseAction, color: Color, size: f32) -> Element<'static, Message> {
    let handle = Handle::from_memory(tinted(action, color));
    svg::Svg::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// Ready-to-use copy icon widget (Lucide-style overlapping rectangles).
pub fn copy_icon(color: Color, size: f32) -> Element<'static, Message> {
    let handle = Handle::from_memory(tint_svg(COPY, color));
    svg::Svg::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// Ready-to-use trash icon widget (Lucide-style trash bin).
pub fn trash_icon(color: Color, size: f32) -> Element<'static, Message> {
    let handle = Handle::from_memory(tint_svg(TRASH, color));
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
