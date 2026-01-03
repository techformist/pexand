//! UI constants and theme colors

use iced::Font;

// Input field IDs
pub const SEARCH_INPUT_ID: &str = "search_input";
pub const TRIGGER_INPUT_ID: &str = "trigger_input";
pub const LABEL_INPUT_ID: &str = "label_input";
pub const SETTINGS_BLOCK_APP_INPUT_ID: &str = "settings_block_app_input";

// Fonts
pub const UI_FONT: Font = Font::with_name("Segoe UI Variable Display");
pub const ICON_FONT: Font = Font::with_name("Segoe Fluent Icons");

// Icon constants using Unicode from Segoe Fluent Icons
pub const ICON_SEARCH: &str = "\u{E721}"; // Search
pub const ICON_ADD: &str = "\u{E710}"; // Add
pub const ICON_EDIT: &str = "\u{E70F}"; // Edit
pub const ICON_DELETE: &str = "\u{E74D}"; // Delete
pub const ICON_SAVE: &str = "\u{E74E}"; // Save
pub const ICON_CANCEL: &str = "\u{E711}"; // Cancel
pub const ICON_HELP: &str = "\u{E897}"; // Help
pub const ICON_SETTINGS: &str = "\u{E713}"; // Settings
pub const ICON_EXPAND: &str = "\u{E740}"; // FullScreen/Expand
pub const ICON_IMPORT: &str = "\u{E8B5}"; // CloudDownload/Import
pub const ICON_EXPORT: &str = "\u{E898}"; // Upload/Export

// Modern startup palette - deep purples with electric accents
pub const COLOR_BG: iced::Color = iced::Color::from_rgb(0.07, 0.07, 0.11);
pub const COLOR_PANEL: iced::Color = iced::Color::from_rgb(0.10, 0.10, 0.15);
pub const COLOR_CARD: iced::Color = iced::Color::from_rgb(0.12, 0.12, 0.18);
pub const COLOR_CARD_HOVER: iced::Color = iced::Color::from_rgb(0.14, 0.14, 0.22);
pub const COLOR_CARD_ACTIVE: iced::Color = iced::Color::from_rgb(0.16, 0.16, 0.26);
pub const COLOR_BORDER: iced::Color = iced::Color::from_rgb(0.20, 0.20, 0.30);
pub const COLOR_MUTED: iced::Color = iced::Color::from_rgb(0.55, 0.55, 0.65);
pub const COLOR_TEXT: iced::Color = iced::Color::from_rgb(0.92, 0.92, 0.96);
pub const COLOR_ACCENT: iced::Color = iced::Color::from_rgb(0.40, 0.45, 0.95);
pub const COLOR_ACCENT_BRIGHT: iced::Color = iced::Color::from_rgb(0.50, 0.55, 1.0);
pub const COLOR_BUTTON_BG: iced::Color = iced::Color::from_rgba(0.40, 0.45, 0.95, 0.12);
pub const COLOR_BUTTON_HOVER: iced::Color = iced::Color::from_rgba(0.40, 0.45, 0.95, 0.20);
pub const COLOR_BUTTON_ACTIVE: iced::Color = iced::Color::from_rgba(0.40, 0.45, 0.95, 0.30);
