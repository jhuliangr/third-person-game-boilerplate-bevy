//! Reusable UI building blocks and the shared palette.

use bevy::prelude::*;

pub(crate) const TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
pub(crate) const MUTED_TEXT: Color = Color::srgb(0.7, 0.72, 0.78);
pub(crate) const OVERLAY: Color = Color::srgba(0.02, 0.03, 0.05, 0.65);
pub(crate) const PANEL: Color = Color::srgba(0.08, 0.09, 0.12, 0.92);
pub(crate) const BUTTON_IDLE: Color = Color::srgb(0.15, 0.17, 0.22);
pub(crate) const BUTTON_HOVERED: Color = Color::srgb(0.25, 0.28, 0.36);
pub(crate) const BUTTON_ACTIVE: Color = Color::srgb(0.85, 0.45, 0.12);

pub(crate) fn full_screen(background: Color) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(20),
            ..default()
        },
        BackgroundColor(background),
    )
}

pub(crate) fn label(text: &str, size: f32) -> impl Bundle {
    colored_label(text, size, TEXT)
}

pub(crate) fn colored_label(text: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// A clickable box with centered text. `marker` identifies what the button does.
pub(crate) fn button(text: &str, width: f32, height: f32, marker: impl Bundle) -> impl Bundle {
    (
        marker,
        Button,
        Node {
            width: px(width),
            height: px(height),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BorderColor::all(MUTED_TEXT),
        BackgroundColor(BUTTON_IDLE),
        children![label(text, 22.0)],
    )
}
