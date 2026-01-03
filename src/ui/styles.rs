//! Custom button styles for modern UI

use iced::widget::button;
use iced::Theme;

use super::constants::*;

/// Modern button style with subtle background
pub fn modern_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (COLOR_BUTTON_BG, COLOR_ACCENT),
        button::Status::Hovered => (COLOR_BUTTON_HOVER, COLOR_ACCENT_BRIGHT),
        button::Status::Pressed => (COLOR_BUTTON_ACTIVE, COLOR_ACCENT_BRIGHT),
        button::Status::Disabled => (COLOR_BUTTON_BG, COLOR_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        text_color,
        ..Default::default()
    }
}

/// Subtle button style with transparent background
pub fn subtle_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (iced::Color::TRANSPARENT, COLOR_MUTED),
        button::Status::Hovered => (COLOR_CARD_HOVER, COLOR_TEXT),
        button::Status::Pressed => (COLOR_CARD_ACTIVE, COLOR_TEXT),
        button::Status::Disabled => (iced::Color::TRANSPARENT, COLOR_MUTED),
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        text_color,
        ..Default::default()
    }
}
