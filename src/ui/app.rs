//! Main application logic

use crate::db::{Snippet, SnippetManager};
use crate::sentinel::SentinelMessage;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use iced::{
    event::{self, Event},
    keyboard::{self, key::Named, Key},
    widget::{
        button, column, container, operation, row, scrollable, text, text_editor, text_input,
        tooltip, Column, Space,
    },
    window, Element, Length, Subscription, Task, Theme,
};
use std::sync::mpsc::Sender;

use super::constants::*;
use super::editor::EditorState;
use super::icons::load_icon;
use super::styles::{modern_button_style, subtle_button_style};

#[derive(Debug, Clone)]
pub enum UiExternalMessage {
    Show,
    Hide,
    Exit,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SnippetSelected(usize),
    SnippetDoubleClicked(usize),
    EditSnippet(usize),
    NewSnippet,
    DeleteSnippet(usize),
    SaveSnippet,
    CancelEdit,
    TriggerChanged(String),
    LabelChanged(String),
    BodyChanged(String),
    BodyEditorAction(text_editor::Action),
    KeyPressed(keyboard::Event),
    NavigateUp,
    NavigateDown,
    ActivateSelected,
    TabPressed,
    ShowHelp,
    CloseHelp,
    ShowSettings,
    CloseSettings,
    OpenLink(String),
    // Settings messages
    AddTriggerApp(String),
    RemoveTriggerApp(usize),
    AddBlockApp(String),
    RemoveBlockApp(usize),
    TriggerAppInputChanged(String),
    BlockAppInputChanged(String),
    ExportSnippets,
    ImportSnippets,
    External(UiExternalMessage),
    CheckExternalMessages,
    WindowOpened(window::Id),
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    List,
    Editor,
    Help,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
enum FocusedField {
    Search,
    List,
    EditorTrigger,
    EditorLabel,
    EditorBody,
    SettingsTriggerApp,
    SettingsBlockApp,
}

pub struct PexandApp {
    search_query: String,
    snippets: Vec<Snippet>,
    filtered_snippets: Vec<(usize, Snippet)>,
    selected_index: usize,
    view_mode: ViewMode,
    editor_state: Option<EditorState>,
    db_path: String,
    sentinel_tx: Option<Sender<SentinelMessage>>,
    matcher: SkimMatcherV2,
    focused_field: FocusedField,
    external_rx: crossbeam_channel::Receiver<UiExternalMessage>,
    window_minimized: bool,
    window_id: Option<window::Id>,
    // Settings state
    trigger_apps: Vec<String>,
    block_apps: Vec<String>,
    trigger_app_input: String,
    block_app_input: String,
}

impl PexandApp {
    pub fn new(
        sentinel_tx: Option<Sender<SentinelMessage>>,
        external_rx: crossbeam_channel::Receiver<UiExternalMessage>,
    ) -> Self {
        let db_path = get_db_path();
        let snippets = load_snippets(&db_path);
        let filtered_snippets = snippets
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.clone()))
            .collect();

        Self {
            search_query: String::new(),
            snippets,
            filtered_snippets,
            selected_index: 0,
            view_mode: ViewMode::List,
            editor_state: None,
            db_path,
            sentinel_tx,
            matcher: SkimMatcherV2::default(),
            focused_field: FocusedField::Search,
            external_rx,
            window_minimized: true,
            window_id: None,
            trigger_apps: Self::load_trigger_apps(),
            block_apps: Self::load_block_apps(),
            trigger_app_input: String::new(),
            block_app_input: String::new(),
        }
    }

    fn reload_snippets(&mut self) {
        self.snippets = load_snippets(&self.db_path);
        self.filter_snippets();
    }

    fn filter_snippets(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_snippets = self
                .snippets
                .iter()
                .enumerate()
                .map(|(i, s)| (i, s.clone()))
                .collect();
        } else {
            let mut scored: Vec<(usize, Snippet, i64)> = self
                .snippets
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let trigger_score = self
                        .matcher
                        .fuzzy_match(&s.trigger, &self.search_query)
                        .unwrap_or(0);
                    let body_score = self
                        .matcher
                        .fuzzy_match(&s.body, &self.search_query)
                        .unwrap_or(0);
                    let score = trigger_score.max(body_score);
                    if score > 0 {
                        Some((i, s.clone(), score))
                    } else {
                        None
                    }
                })
                .collect();

            scored.sort_by(|a, b| b.2.cmp(&a.2));
            self.filtered_snippets = scored.into_iter().map(|(i, s, _)| (i, s)).collect();
        }

        if self.selected_index >= self.filtered_snippets.len() && !self.filtered_snippets.is_empty()
        {
            self.selected_index = 0;
        }
    }

    fn start_edit(&mut self, index: usize) {
        if let Some((original_index, snippet)) = self.filtered_snippets.get(index) {
            self.editor_state = Some(EditorState::edit(
                snippet.trigger.clone(),
                snippet.label.clone().unwrap_or_default(),
                snippet.body.clone(),
                *original_index,
            ));
            self.view_mode = ViewMode::Editor;
            self.focused_field = FocusedField::EditorTrigger;
        }
    }

    fn start_new(&mut self) {
        self.editor_state = Some(EditorState::new());
        self.view_mode = ViewMode::Editor;
        self.focused_field = FocusedField::EditorTrigger;
    }

    fn save_current(&mut self) {
        if let Some(editor) = &self.editor_state {
            if !editor.is_valid() {
                return;
            }

            let conn = match rusqlite::Connection::open(&self.db_path) {
                Ok(c) => c,
                Err(_) => return,
            };

            let manager = SnippetManager::new(&conn);

            if let Some(original_index) = editor.editing_index {
                // Update existing
                if let Some(original_snippet) = self.snippets.get(original_index) {
                    let mut updated = original_snippet.clone();
                    updated.trigger = editor.trigger.clone();
                    updated.label = if editor.label.is_empty() {
                        None
                    } else {
                        Some(editor.label.clone())
                    };
                    updated.body = editor.body.clone();
                    updated.touch();
                    let _ = manager.update(&updated);
                }
            } else {
                // Create new
                let snippet = Snippet::with_label(
                    editor.trigger.clone(),
                    if editor.label.is_empty() {
                        None
                    } else {
                        Some(editor.label.clone())
                    },
                    editor.body.clone(),
                );
                let _ = manager.create(&snippet);
            }

            // Notify Sentinel to reload
            if let Some(tx) = &self.sentinel_tx {
                let _ = tx.send(SentinelMessage::ReloadTrie);
            }

            self.reload_snippets();
            self.view_mode = ViewMode::List;
            self.editor_state = None;
            self.focused_field = FocusedField::Search;
        }
    }

    fn delete_snippet(&mut self, index: usize) {
        if let Some((original_index, _)) = self.filtered_snippets.get(index) {
            if let Some(snippet) = self.snippets.get(*original_index) {
                let conn = match rusqlite::Connection::open(&self.db_path) {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let manager = SnippetManager::new(&conn);
                let _ = manager.delete(&snippet.trigger);

                // Notify Sentinel to reload
                if let Some(tx) = &self.sentinel_tx {
                    let _ = tx.send(SentinelMessage::ReloadTrie);
                }

                self.reload_snippets();
            }
        }
    }

    fn handle_tab_navigation(&mut self) -> Task<Message> {
        match self.view_mode {
            ViewMode::List => match self.focused_field {
                FocusedField::Search => {
                    if !self.filtered_snippets.is_empty() {
                        self.focused_field = FocusedField::List;
                    }
                    Task::none()
                }
                FocusedField::List => {
                    self.focused_field = FocusedField::Search;
                    Task::none()
                }
                _ => Task::none(),
            },
            ViewMode::Editor => match self.focused_field {
                FocusedField::EditorTrigger => {
                    self.focused_field = FocusedField::EditorLabel;
                    operation::focus(iced::widget::Id::from(LABEL_INPUT_ID))
                }
                FocusedField::EditorLabel => {
                    self.focused_field = FocusedField::EditorBody;
                    operation::focus(iced::widget::Id::from("body_editor"))
                }
                FocusedField::EditorBody => {
                    self.focused_field = FocusedField::EditorTrigger;
                    operation::focus(iced::widget::Id::from(TRIGGER_INPUT_ID))
                }
                _ => Task::none(),
            },
            ViewMode::Help => Task::none(),
            ViewMode::Settings => match self.focused_field {
                FocusedField::SettingsTriggerApp => {
                    self.focused_field = FocusedField::SettingsBlockApp;
                    operation::focus(iced::widget::Id::from(SETTINGS_BLOCK_APP_INPUT_ID))
                }
                FocusedField::SettingsBlockApp => {
                    self.focused_field = FocusedField::SettingsTriggerApp;
                    operation::focus(iced::widget::Id::from(SETTINGS_TRIGGER_APP_INPUT_ID))
                }
                _ => Task::none(),
            },
        }
    }
}

impl PexandApp {
    fn init(
        flags: (
            Option<Sender<SentinelMessage>>,
            crossbeam_channel::Receiver<UiExternalMessage>,
        ),
    ) -> (Self, Task<Message>) {
        let (sentinel_tx, external_rx) = flags;
        let app = Self::new(sentinel_tx, external_rx);
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.filter_snippets();
            }
            Message::SnippetSelected(index) => {
                self.selected_index = index;
            }
            Message::SnippetDoubleClicked(index) => {
                self.start_edit(index);
            }
            Message::EditSnippet(index) => {
                self.start_edit(index);
                return operation::focus(iced::widget::Id::from(TRIGGER_INPUT_ID));
            }
            Message::NewSnippet => {
                self.start_new();
                return operation::focus(iced::widget::Id::from(TRIGGER_INPUT_ID));
            }
            Message::DeleteSnippet(index) => {
                self.delete_snippet(index);
            }
            Message::SaveSnippet => {
                self.save_current();
            }
            Message::CancelEdit => {
                self.view_mode = ViewMode::List;
                self.editor_state = None;
                self.focused_field = FocusedField::Search;
            }
            Message::TriggerChanged(value) => {
                if let Some(editor) = &mut self.editor_state {
                    editor.trigger = value;
                }
            }
            Message::LabelChanged(value) => {
                if let Some(editor) = &mut self.editor_state {
                    editor.label = value;
                }
            }
            Message::BodyChanged(value) => {
                if let Some(editor) = &mut self.editor_state {
                    editor.body = value;
                }
            }
            Message::BodyEditorAction(action) => {
                if let Some(editor) = &mut self.editor_state {
                    editor.body_content.perform(action);
                    editor.body = editor.body_content.text();
                }
            }
            Message::External(external) => match external {
                UiExternalMessage::Show => {
                    eprintln!(
                        "[DEBUG] Show message received, window_id: {:?}",
                        self.window_id
                    );
                    self.window_minimized = false;
                    self.focused_field = FocusedField::Search;
                    // Show and focus the window, then focus the search input
                    return window::latest().and_then(|id| {
                        eprintln!("[DEBUG] Got latest window id: {:?}", id);
                        Task::batch([
                            window::set_mode(id, window::Mode::Windowed),
                            window::gain_focus(id),
                            operation::focus(iced::widget::Id::from(SEARCH_INPUT_ID)),
                        ])
                    });
                }
                UiExternalMessage::Hide => {
                    self.window_minimized = true;
                    // Hide the window
                    if let Some(id) = self.window_id {
                        return window::set_mode(id, window::Mode::Hidden);
                    }
                    return window::latest()
                        .and_then(|id| window::set_mode(id, window::Mode::Hidden));
                }
                UiExternalMessage::Exit => {
                    std::process::exit(0);
                }
            },
            Message::KeyPressed(kbd_event) => {
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = kbd_event {
                    match key {
                        Key::Named(Named::Escape) => match self.view_mode {
                            ViewMode::Editor => {
                                self.view_mode = ViewMode::List;
                                self.editor_state = None;
                                self.focused_field = FocusedField::Search;
                            }
                            ViewMode::Help => {
                                self.view_mode = ViewMode::List;
                            }
                            ViewMode::Settings => {
                                self.view_mode = ViewMode::List;
                            }
                            ViewMode::List => {
                                self.window_minimized = true;
                                // Hide the window
                                if let Some(id) = self.window_id {
                                    return window::set_mode(id, window::Mode::Hidden);
                                }
                                return window::latest()
                                    .and_then(|id| window::set_mode(id, window::Mode::Hidden));
                            }
                        },
                        Key::Named(Named::Enter) => match self.view_mode {
                            ViewMode::List if self.focused_field == FocusedField::List => {
                                if !self.filtered_snippets.is_empty()
                                    && self.selected_index < self.filtered_snippets.len()
                                {
                                    self.start_edit(self.selected_index);
                                }
                            }
                            _ => {}
                        },
                        Key::Named(Named::Tab) => {
                            return self.handle_tab_navigation();
                        }
                        Key::Named(Named::ArrowUp) => {
                            if self.view_mode == ViewMode::List
                                && self.focused_field == FocusedField::List
                            {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                }
                            }
                        }
                        Key::Named(Named::ArrowDown) => {
                            if self.view_mode == ViewMode::List
                                && self.focused_field == FocusedField::List
                            {
                                if self.selected_index
                                    < self.filtered_snippets.len().saturating_sub(1)
                                {
                                    self.selected_index += 1;
                                }
                            }
                        }
                        Key::Named(Named::Delete) => {
                            if self.view_mode == ViewMode::List
                                && self.focused_field == FocusedField::List
                            {
                                if !self.filtered_snippets.is_empty()
                                    && self.selected_index < self.filtered_snippets.len()
                                {
                                    self.delete_snippet(self.selected_index);
                                }
                            }
                        }
                        Key::Character(c) if c.as_str() == "n" || c.as_str() == "N" => {
                            if modifiers.control() && self.view_mode == ViewMode::List {
                                self.start_new();
                            }
                        }
                        Key::Character(c) if c.as_str() == "s" || c.as_str() == "S" => {
                            if modifiers.control() && self.view_mode == ViewMode::Editor {
                                self.save_current();
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::TabPressed => {
                return self.handle_tab_navigation();
            }
            Message::ShowHelp => {
                self.view_mode = ViewMode::Help;
            }
            Message::CloseHelp => {
                self.view_mode = ViewMode::List;
            }
            Message::ShowSettings => {
                self.view_mode = ViewMode::Settings;
                self.focused_field = FocusedField::SettingsTriggerApp;
                return operation::focus(iced::widget::Id::from(SETTINGS_TRIGGER_APP_INPUT_ID));
            }
            Message::CloseSettings => {
                self.view_mode = ViewMode::List;
            }
            Message::OpenLink(url) => {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn();
            }
            Message::NavigateUp => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Message::NavigateDown => {
                if self.selected_index < self.filtered_snippets.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            Message::ActivateSelected => {
                if !self.filtered_snippets.is_empty()
                    && self.selected_index < self.filtered_snippets.len()
                {
                    self.start_edit(self.selected_index);
                }
            }
            Message::CheckExternalMessages => {
                // Poll for external messages
                if let Ok(msg) = self.external_rx.try_recv() {
                    eprintln!("[DEBUG] Received external message: {:?}", msg);
                    return Task::done(Message::External(msg));
                }
            }
            Message::WindowOpened(id) => {
                // Store the window ID when window opens
                eprintln!("[DEBUG] Window opened with id: {:?}", id);
                self.window_id = Some(id);
            }
            Message::TriggerAppInputChanged(input) => {
                self.trigger_app_input = input;
            }
            Message::BlockAppInputChanged(input) => {
                self.block_app_input = input;
            }
            Message::AddTriggerApp(app_name) => {
                if !app_name.trim().is_empty() && !self.trigger_apps.contains(&app_name) {
                    self.trigger_apps.push(app_name);
                    Self::save_trigger_apps(&self.trigger_apps);
                    self.trigger_app_input.clear();
                }
            }
            Message::RemoveTriggerApp(index) => {
                if index < self.trigger_apps.len() {
                    self.trigger_apps.remove(index);
                    Self::save_trigger_apps(&self.trigger_apps);
                }
            }
            Message::AddBlockApp(app_name) => {
                if !app_name.trim().is_empty() && !self.block_apps.contains(&app_name) {
                    self.block_apps.push(app_name);
                    Self::save_block_apps(&self.block_apps);
                    self.block_app_input.clear();
                }
            }
            Message::RemoveBlockApp(index) => {
                if index < self.block_apps.len() {
                    self.block_apps.remove(index);
                    Self::save_block_apps(&self.block_apps);
                }
            }
            Message::ExportSnippets => {
                self.export_snippets();
            }
            Message::ImportSnippets => {
                self.import_snippets();
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.view_mode {
            ViewMode::List => self.view_list(),
            ViewMode::Editor => self.view_editor(),
            ViewMode::Help => self.view_help(),
            ViewMode::Settings => self.view_settings(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        String::from("pexand")
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = event::listen_with(|event, _status, _id| {
            if let Event::Keyboard(kbd_event) = event {
                Some(Message::KeyPressed(kbd_event))
            } else {
                None
            }
        });

        // Poll the external channel periodically
        let external_check = iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::CheckExternalMessages);

        // Listen for window open events to capture window ID
        let window_events = window::open_events().map(Message::WindowOpened);

        Subscription::batch([keyboard, external_check, window_events])
    }
}

impl PexandApp {
    fn view_list(&self) -> Element<'_, Message> {
        // Header with icon, title, help, and new button
        let icon_text = text(ICON_EXPAND)
            .font(ICON_FONT)
            .size(12.0)
            .color(COLOR_ACCENT);

        let icon_container = container(icon_text)
            .padding(4)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BUTTON_BG)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

        let title = text("pexand").font(UI_FONT).size(13).color(COLOR_TEXT);

        let settings_icon = text(ICON_SETTINGS)
            .font(ICON_FONT)
            .size(13)
            .color(COLOR_MUTED)
            .align_x(iced::alignment::Horizontal::Center);

        let settings_btn = tooltip(
            button(settings_icon)
                .on_press(Message::ShowSettings)
                .padding(6)
                .width(Length::Fixed(20.0))
                .style(subtle_button_style),
            "Settings",
            tooltip::Position::Bottom,
        );

        let help_icon = text(ICON_HELP)
            .font(ICON_FONT)
            .size(13)
            .color(COLOR_MUTED)
            .align_x(iced::alignment::Horizontal::Center);

        let help_btn = tooltip(
            button(help_icon)
                .on_press(Message::ShowHelp)
                .padding(6)
                .width(Length::Fixed(20.0))
                .style(subtle_button_style),
            "Help & Shortcuts",
            tooltip::Position::Bottom,
        );

        let new_icon = text(ICON_ADD)
            .font(ICON_FONT)
            .size(13)
            .color(COLOR_ACCENT)
            .align_x(iced::alignment::Horizontal::Center);

        let new_btn = tooltip(
            button(new_icon)
                .on_press(Message::NewSnippet)
                .padding(8)
                .width(Length::Fixed(36.0))
                .style(modern_button_style),
            "Create new snippet (Ctrl+N)",
            tooltip::Position::Bottom,
        );

        let header = row![
            icon_container,
            title,
            Space::new().width(Length::Fill),
            settings_btn,
            help_btn,
            new_btn
        ]
        .spacing(8)
        .padding(iced::Padding::new(8.0).left(12.0).right(12.0).bottom(6.0))
        .align_y(iced::Alignment::Center);

        // Search input with icon
        let search_icon = text(ICON_SEARCH)
            .font(ICON_FONT)
            .size(15)
            .color(COLOR_MUTED);

        let search_input = text_input("Search triggers or content...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding([10, 12])
            .size(14)
            .font(UI_FONT)
            .width(Length::Fill)
            .id(SEARCH_INPUT_ID);

        let search = container(
            row![search_icon, search_input]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        )
        .padding([6, 12])
        .width(Length::Fill);

        // Snippet list
        let mut list_items = Column::new().spacing(6).padding([0, 12]);

        if self.filtered_snippets.is_empty() {
            let empty_msg = container(
                text("No snippets found. Press Ctrl+N to create one.")
                    .font(UI_FONT)
                    .size(13)
                    .color(COLOR_MUTED),
            )
            .padding(24)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center);
            list_items = list_items.push(empty_msg);
        } else {
            for (idx, (_, snippet)) in self.filtered_snippets.iter().enumerate() {
                let is_selected =
                    idx == self.selected_index && self.focused_field == FocusedField::List;

                // Trigger with optional label
                let trigger_display = if let Some(label) = &snippet.label {
                    format!("{} - {}", snippet.trigger, label)
                } else {
                    snippet.trigger.clone()
                };

                let trigger_text = text(trigger_display)
                    .font(UI_FONT)
                    .size(14)
                    .color(COLOR_TEXT);

                // Body preview
                let first_line = snippet.body.lines().next().unwrap_or("");
                let body_preview: String = if first_line.len() > 60 {
                    format!("{}...", first_line.chars().take(60).collect::<String>())
                } else if snippet.body.lines().count() > 1 {
                    format!("{}...", first_line)
                } else {
                    first_line.to_string()
                };
                let body_text = text(body_preview).font(UI_FONT).size(12).color(COLOR_MUTED);

                let snippet_info = column![trigger_text, body_text]
                    .spacing(4)
                    .width(Length::Fill);

                let clickable_info = button(snippet_info)
                    .on_press(Message::SnippetDoubleClicked(idx))
                    .padding(0)
                    .style(button::text);

                // Action buttons
                let edit_icon = text(ICON_EDIT)
                    .font(ICON_FONT)
                    .size(14)
                    .color(COLOR_MUTED)
                    .align_x(iced::alignment::Horizontal::Center);

                let edit_btn = tooltip(
                    button(edit_icon)
                        .on_press(Message::EditSnippet(idx))
                        .padding(6)
                        .width(Length::Fixed(28.0))
                        .style(subtle_button_style),
                    "Edit snippet",
                    tooltip::Position::Top,
                );

                let delete_icon = text(ICON_DELETE)
                    .font(ICON_FONT)
                    .size(14)
                    .color(COLOR_MUTED)
                    .align_x(iced::alignment::Horizontal::Center);

                let delete_btn = tooltip(
                    button(delete_icon)
                        .on_press(Message::DeleteSnippet(idx))
                        .padding(6)
                        .width(Length::Fixed(28.0))
                        .style(subtle_button_style),
                    "Delete snippet",
                    tooltip::Position::Top,
                );

                let snippet_row = container(
                    row![clickable_info, edit_btn, delete_btn]
                        .spacing(8)
                        .padding([10, 12])
                        .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .style(move |_: &Theme| {
                    if is_selected {
                        container::Style {
                            background: Some(iced::Background::Color(COLOR_CARD_ACTIVE)),
                            border: iced::Border {
                                color: COLOR_ACCENT,
                                width: 2.0,
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        }
                    } else {
                        container::Style {
                            background: Some(iced::Background::Color(COLOR_CARD)),
                            border: iced::Border {
                                color: COLOR_BORDER,
                                width: 1.0,
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                });

                list_items = list_items.push(snippet_row);
            }
        }

        let list = scrollable(list_items)
            .height(Length::Fill)
            .width(Length::Fill);

        let hints = text("Tab: Navigate | Up/Down: Select | Enter: Edit | Del: Delete | Esc: Hide")
            .font(UI_FONT)
            .size(10)
            .color(COLOR_MUTED);

        let footer = container(hints)
            .padding(iced::Padding::new(6.0).left(12.0).right(12.0).bottom(8.0))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center);

        let content = column![header, search, list, footer].spacing(6);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(6)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BG)),
                ..Default::default()
            })
            .into()
    }

    fn view_editor(&self) -> Element<'_, Message> {
        if let Some(editor) = &self.editor_state {
            let title_text = if editor.editing_index.is_some() {
                "Edit Snippet"
            } else {
                "New Snippet"
            };

            let icon_text = text(ICON_EXPAND)
                .font(ICON_FONT)
                .size(12.0)
                .color(COLOR_ACCENT);

            let icon_container =
                container(icon_text)
                    .padding(4)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(COLOR_BUTTON_BG)),
                        border: iced::Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });

            let header = row![
                icon_container,
                text(title_text).font(UI_FONT).size(13).color(COLOR_TEXT)
            ]
            .spacing(8)
            .padding(iced::Padding::new(8.0).left(12.0).right(12.0).bottom(6.0))
            .align_y(iced::Alignment::Center);

            // Form fields
            let trigger_label = text("Trigger").font(UI_FONT).size(12).color(COLOR_MUTED);
            let trigger_input = text_input(";trigger", &editor.trigger)
                .on_input(Message::TriggerChanged)
                .padding([10, 12])
                .font(UI_FONT)
                .size(14)
                .id(TRIGGER_INPUT_ID);

            let label_label = text("Label (optional)")
                .font(UI_FONT)
                .size(12)
                .color(COLOR_MUTED);
            let label_input = text_input("Description", &editor.label)
                .on_input(Message::LabelChanged)
                .padding([10, 12])
                .font(UI_FONT)
                .size(14)
                .id(LABEL_INPUT_ID);

            let body_label = text("Expansion").font(UI_FONT).size(12).color(COLOR_MUTED);

            let body_editor = text_editor(&editor.body_content)
                .on_action(Message::BodyEditorAction)
                .font(UI_FONT)
                .height(Length::Fill)
                .id("body_editor");

            let body_input_container = container(body_editor)
                .height(Length::Fill)
                .width(Length::Fill)
                .style(|_: &Theme| container::Style {
                    background: Some(iced::Background::Color(COLOR_PANEL)),
                    border: iced::Border {
                        color: COLOR_BORDER,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });

            let form = container(
                column![
                    trigger_label,
                    trigger_input,
                    Space::new().height(Length::Fixed(12.0)),
                    label_label,
                    label_input,
                    Space::new().height(Length::Fixed(12.0)),
                    body_label,
                    body_input_container,
                ]
                .spacing(4)
                .height(Length::Fill),
            )
            .padding(12)
            .height(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_CARD)),
                border: iced::Border {
                    color: COLOR_BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

            // Action buttons
            let save_icon = text(ICON_SAVE).font(ICON_FONT).size(14).color(COLOR_ACCENT);

            let save_btn = tooltip(
                button(
                    row![save_icon, text("Save").font(UI_FONT).size(13)]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                )
                .on_press(Message::SaveSnippet)
                .padding([8, 14])
                .style(modern_button_style),
                "Save snippet (Ctrl+S)",
                tooltip::Position::Top,
            );

            let cancel_icon = text(ICON_CANCEL)
                .font(ICON_FONT)
                .size(14)
                .color(COLOR_MUTED);

            let cancel_btn = tooltip(
                button(
                    row![cancel_icon, text("Cancel").font(UI_FONT).size(13)]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                )
                .on_press(Message::CancelEdit)
                .padding([8, 14])
                .style(subtle_button_style),
                "Cancel editing (Esc)",
                tooltip::Position::Top,
            );

            let actions = row![save_btn, cancel_btn]
                .spacing(8)
                .padding([12, 0])
                .align_y(iced::Alignment::Center);

            let hints = text("Tab: Next Field | Ctrl+S: Save | Esc: Cancel")
                .font(UI_FONT)
                .size(10)
                .color(COLOR_MUTED);

            let content = column![
                header,
                Space::new().height(Length::Fixed(6.0)),
                form,
                Space::new().height(Length::Fixed(12.0)),
                actions,
                Space::new().height(Length::Fixed(6.0)),
                container(hints).padding([0, 12]),
            ]
            .spacing(4)
            .padding([0, 12]);

            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(6)
                .style(|_: &Theme| container::Style {
                    background: Some(iced::Background::Color(COLOR_BG)),
                    ..Default::default()
                })
                .into()
        } else {
            container(text("No editor state")).into()
        }
    }

    fn view_help(&self) -> Element<'_, Message> {
        let icon_text = text(ICON_EXPAND)
            .font(ICON_FONT)
            .size(32.0)
            .color(COLOR_ACCENT);

        let icon_container = container(icon_text)
            .padding(8)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BUTTON_BG)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

        let title = text("Help & Shortcuts")
            .font(UI_FONT)
            .size(18)
            .color(COLOR_TEXT);

        let header = column![icon_container, title]
            .spacing(12)
            .align_x(iced::Alignment::Center);

        let purpose_title = text("Purpose").font(UI_FONT).size(14).color(COLOR_ACCENT);

        let purpose_text = text("Pexand is a lightweight text expander that helps you save time by expanding short triggers into longer text snippets. Type a trigger like ;email and it instantly expands to your full email address.")
            .font(UI_FONT)
            .size(12)
            .color(COLOR_TEXT)
            .line_height(1.5);

        let shortcuts_title = text("Keyboard Shortcuts")
            .font(UI_FONT)
            .size(14)
            .color(COLOR_ACCENT);

        let shortcuts = column![
            shortcut_row("Ctrl+Alt+Shift+P", "Open Pexand window"),
            shortcut_row("Ctrl+N", "Create new snippet"),
            shortcut_row("Tab", "Navigate between fields"),
            shortcut_row("Up/Down", "Select snippet in list"),
            shortcut_row("Enter", "Edit selected snippet"),
            shortcut_row("Delete", "Delete selected snippet"),
            shortcut_row("Ctrl+S", "Save snippet (in editor)"),
            shortcut_row("Esc", "Close window or cancel"),
        ]
        .spacing(6);

        let learn_more_title = text("Learn More")
            .font(UI_FONT)
            .size(14)
            .color(COLOR_ACCENT);

        let website_url = "https://pexand.techformist.com";
        let learn_more_intro = text("Visit ").font(UI_FONT).size(12).color(COLOR_MUTED);

        let link_text = text(website_url).font(UI_FONT).size(12).color(COLOR_ACCENT);

        let link_btn = button(link_text)
            .on_press(Message::OpenLink(website_url.to_string()))
            .padding([2, 4])
            .style(button::text);

        let learn_more_outro = text(" for documentation, tutorials, and advanced features.")
            .font(UI_FONT)
            .size(12)
            .color(COLOR_MUTED);

        let learn_more_row =
            row![learn_more_intro, link_btn, learn_more_outro].align_y(iced::Alignment::Center);

        let close_btn = button(text("Close").font(UI_FONT).size(13).color(COLOR_ACCENT))
            .on_press(Message::CloseHelp)
            .padding([8, 20])
            .style(modern_button_style);

        let content = container(
            column![
                header,
                Space::new().height(Length::Fixed(20.0)),
                purpose_title,
                Space::new().height(Length::Fixed(8.0)),
                purpose_text,
                Space::new().height(Length::Fixed(20.0)),
                shortcuts_title,
                Space::new().height(Length::Fixed(8.0)),
                shortcuts,
                Space::new().height(Length::Fixed(20.0)),
                learn_more_title,
                Space::new().height(Length::Fixed(8.0)),
                learn_more_row,
                Space::new().height(Length::Fixed(24.0)),
                container(close_btn).align_x(iced::alignment::Horizontal::Center),
            ]
            .padding(20)
            .max_width(500),
        )
        .align_x(iced::alignment::Horizontal::Center)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(COLOR_CARD)),
            border: iced::Border {
                color: COLOR_BORDER,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

        container(scrollable(content).height(Length::Fill).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BG)),
                ..Default::default()
            })
            .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        // Header with icon similar to New Snippet page
        let header = row![
            text(ICON_SETTINGS)
                .font(ICON_FONT)
                .size(16.0)
                .color(COLOR_ACCENT),
            text("Settings").font(UI_FONT).size(13).color(COLOR_TEXT)
        ]
        .spacing(8)
        .padding(iced::Padding::new(8.0).left(12.0).right(12.0).bottom(6.0))
        .align_y(iced::Alignment::Center);

        // Application Filtering Section Header
        let filtering_header = text("Application Filtering")
            .font(UI_FONT)
            .size(14)
            .color(COLOR_TEXT);

        let filtering_desc = text("Control which applications trigger text expansion. If \"Only in these apps\" is empty, expansion works everywhere except blocked apps.")
            .font(UI_FONT)
            .size(11)
            .color(COLOR_MUTED)
            .line_height(1.4);

        // Trigger Apps (Whitelist)
        let trigger_apps_title = text("Only in these apps (whitelist)")
            .font(UI_FONT)
            .size(12)
            .color(COLOR_MUTED);

        let trigger_input = text_input("e.g., notepad.exe, chrome.exe", &self.trigger_app_input)
            .on_input(Message::TriggerAppInputChanged)
            .on_submit(Message::AddTriggerApp(self.trigger_app_input.clone()))
            .padding([10, 12])
            .font(UI_FONT)
            .size(14)
            .id(SETTINGS_TRIGGER_APP_INPUT_ID);

        let add_trigger_btn = button(text(ICON_ADD).font(ICON_FONT).size(12).color(COLOR_ACCENT))
            .on_press(Message::AddTriggerApp(self.trigger_app_input.clone()))
            .padding(8)
            .style(modern_button_style);

        let trigger_input_row = row![trigger_input, add_trigger_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let mut trigger_apps_list = Column::new().spacing(4);
        for (idx, app) in self.trigger_apps.iter().enumerate() {
            let app_row = row![
                text(app).font(UI_FONT).size(11).color(COLOR_TEXT),
                Space::new().width(Length::Fill),
                button(
                    text(ICON_DELETE)
                        .font(ICON_FONT)
                        .size(11)
                        .color(COLOR_MUTED)
                )
                .on_press(Message::RemoveTriggerApp(idx))
                .padding(4)
                .style(subtle_button_style)
            ]
            .spacing(8)
            .padding(6)
            .align_y(iced::Alignment::Center);

            let app_container =
                container(app_row)
                    .width(Length::Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(COLOR_PANEL)),
                        border: iced::Border {
                            color: COLOR_BORDER,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });

            trigger_apps_list = trigger_apps_list.push(app_container);
        }

        // Block Apps (Blacklist)
        let block_apps_title = text("Never in these apps (blacklist)")
            .font(UI_FONT)
            .size(12)
            .color(COLOR_MUTED);

        let block_input = text_input("e.g., cmd.exe, powershell.exe", &self.block_app_input)
            .on_input(Message::BlockAppInputChanged)
            .on_submit(Message::AddBlockApp(self.block_app_input.clone()))
            .padding([10, 12])
            .font(UI_FONT)
            .size(14)
            .id(SETTINGS_BLOCK_APP_INPUT_ID);

        let add_block_btn = button(text(ICON_ADD).font(ICON_FONT).size(12).color(COLOR_ACCENT))
            .on_press(Message::AddBlockApp(self.block_app_input.clone()))
            .padding(8)
            .style(modern_button_style);

        let block_input_row = row![block_input, add_block_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let mut block_apps_list = Column::new().spacing(4);
        for (idx, app) in self.block_apps.iter().enumerate() {
            let app_row = row![
                text(app).font(UI_FONT).size(11).color(COLOR_TEXT),
                Space::new().width(Length::Fill),
                button(
                    text(ICON_DELETE)
                        .font(ICON_FONT)
                        .size(11)
                        .color(COLOR_MUTED)
                )
                .on_press(Message::RemoveBlockApp(idx))
                .padding(4)
                .style(subtle_button_style)
            ]
            .spacing(8)
            .padding(6)
            .align_y(iced::Alignment::Center);

            let app_container =
                container(app_row)
                    .width(Length::Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(COLOR_PANEL)),
                        border: iced::Border {
                            color: COLOR_BORDER,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });

            block_apps_list = block_apps_list.push(app_container);
        }

        // Application Filtering section container with border
        let filtering_section = container(
            column![
                filtering_header,
                Space::new().height(Length::Fixed(8.0)),
                filtering_desc,
                Space::new().height(Length::Fixed(16.0)),
                trigger_apps_title,
                Space::new().height(Length::Fixed(6.0)),
                trigger_input_row,
                Space::new().height(Length::Fixed(8.0)),
                trigger_apps_list,
                Space::new().height(Length::Fixed(16.0)),
                block_apps_title,
                Space::new().height(Length::Fixed(6.0)),
                block_input_row,
                Space::new().height(Length::Fixed(8.0)),
                block_apps_list,
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(COLOR_CARD)),
            border: iced::Border {
                color: COLOR_BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        // Data Management Section Header
        let data_header = text("Data Management")
            .font(UI_FONT)
            .size(14)
            .color(COLOR_TEXT);

        // Data Management Section with border
        let export_btn = button(
            row![
                text(ICON_EXPORT)
                    .font(ICON_FONT)
                    .size(13)
                    .color(COLOR_ACCENT),
                text("Export Snippets")
                    .font(UI_FONT)
                    .size(12)
                    .color(COLOR_TEXT)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ExportSnippets)
        .padding([8, 16])
        .style(modern_button_style);

        let import_btn = button(
            row![
                text(ICON_IMPORT)
                    .font(ICON_FONT)
                    .size(13)
                    .color(COLOR_ACCENT),
                text("Import Snippets")
                    .font(UI_FONT)
                    .size(12)
                    .color(COLOR_TEXT)
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ImportSnippets)
        .padding([8, 16])
        .style(modern_button_style);

        let data_section = container(
            column![
                data_header,
                Space::new().height(Length::Fixed(12.0)),
                row![export_btn, import_btn].spacing(12)
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(COLOR_CARD)),
            border: iced::Border {
                color: COLOR_BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        // Close button - similar to new snippet page style
        let close_btn = button(
            row![
                text(ICON_CANCEL)
                    .font(ICON_FONT)
                    .size(14)
                    .color(COLOR_ACCENT),
                text("Close").font(UI_FONT).size(13).color(COLOR_TEXT)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::CloseSettings)
        .padding([8, 14])
        .style(modern_button_style);

        let content = column![
            header,
            Space::new().height(Length::Fixed(6.0)),
            filtering_section,
            Space::new().height(Length::Fixed(12.0)),
            data_section,
            Space::new().height(Length::Fixed(12.0)),
            row![close_btn].padding([0, 12]),
        ]
        .spacing(4)
        .padding([0, 12]);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(6)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BG)),
                ..Default::default()
            })
            .into()
    }

    // Load/Save settings helpers
    fn load_trigger_apps() -> Vec<String> {
        let path = Self::get_settings_path("trigger_apps.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_trigger_apps(apps: &[String]) {
        let path = Self::get_settings_path("trigger_apps.json");
        if let Ok(content) = serde_json::to_string_pretty(apps) {
            let _ = std::fs::write(&path, content);
        }
    }

    fn load_block_apps() -> Vec<String> {
        let path = Self::get_settings_path("block_apps.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_block_apps(apps: &[String]) {
        let path = Self::get_settings_path("block_apps.json");
        if let Ok(content) = serde_json::to_string_pretty(apps) {
            let _ = std::fs::write(&path, content);
        }
    }

    fn get_settings_path(filename: &str) -> std::path::PathBuf {
        use std::path::{Path, PathBuf};

        let portable_marker = Path::new("portable.txt");

        if portable_marker.exists() {
            PathBuf::from(filename)
        } else {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            let mut path = PathBuf::from(appdata);
            path.push("Pexand");
            std::fs::create_dir_all(&path).ok();
            path.push(filename);
            path
        }
    }

    fn export_snippets(&self) {
        use std::io::Write;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("pexand_export_{}.json", timestamp);

        // Try to use file dialog, fallback to desktop
        let export_path = match rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter("JSON", &["json"])
            .save_file()
        {
            Some(path) => path,
            None => {
                // Fallback to desktop
                let desktop = std::env::var("USERPROFILE")
                    .map(|p| std::path::PathBuf::from(p).join("Desktop"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                desktop.join(&filename)
            }
        };

        // Serialize snippets
        if let Ok(json) = serde_json::to_string_pretty(&self.snippets) {
            if let Ok(mut file) = std::fs::File::create(&export_path) {
                let _ = file.write_all(json.as_bytes());
                println!("Exported snippets to: {:?}", export_path);
            }
        }
    }

    fn import_snippets(&mut self) {
        // Open file dialog
        let import_path = match rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            Some(path) => path,
            None => return,
        };

        // Read and parse JSON
        if let Ok(content) = std::fs::read_to_string(&import_path) {
            if let Ok(imported_snippets) = serde_json::from_str::<Vec<Snippet>>(&content) {
                let conn = match rusqlite::Connection::open(&self.db_path) {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let manager = SnippetManager::new(&conn);

                // Import each snippet
                for snippet in imported_snippets {
                    // Check if trigger already exists
                    if manager.read(&snippet.trigger).ok().flatten().is_some() {
                        // Update existing
                        let _ = manager.update(&snippet);
                    } else {
                        // Create new
                        let _ = manager.create(&snippet);
                    }
                }

                // Notify Sentinel to reload
                if let Some(tx) = &self.sentinel_tx {
                    let _ = tx.send(SentinelMessage::ReloadTrie);
                }

                self.reload_snippets();
                println!("Imported snippets from: {:?}", import_path);
            }
        }
    }
}

fn shortcut_row<'a>(key: &'a str, description: &'a str) -> Element<'a, Message> {
    row![
        container(text(key).font(UI_FONT).size(11).color(COLOR_ACCENT))
            .padding([4, 10])
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(COLOR_BUTTON_BG)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
        Space::new().width(Length::Fixed(12.0)),
        text(description).font(UI_FONT).size(12).color(COLOR_TEXT),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

fn get_db_path() -> String {
    use std::path::{Path, PathBuf};

    let portable_marker = Path::new("portable.txt");

    if portable_marker.exists() {
        "pexand.db".to_string()
    } else {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(appdata);
        path.push("Pexand");
        path.push("pexand.db");
        path.to_string_lossy().to_string()
    }
}

fn load_snippets(db_path: &str) -> Vec<Snippet> {
    match rusqlite::Connection::open(db_path) {
        Ok(conn) => {
            let manager = SnippetManager::new(&conn);
            manager.list_all().unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

pub fn run_ui(
    sentinel_tx: Sender<SentinelMessage>,
    external_rx: crossbeam_channel::Receiver<UiExternalMessage>,
) -> iced::Result {
    let sentinel_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(sentinel_tx)));
    let external_rx = std::sync::Arc::new(external_rx);

    iced::application(
        move || {
            let tx = sentinel_tx.lock().unwrap().take();
            let rx = external_rx.clone();
            PexandApp::init((tx, (*rx).clone()))
        },
        PexandApp::update,
        PexandApp::view,
    )
    .subscription(PexandApp::subscription)
    .theme(PexandApp::theme)
    .title(PexandApp::title)
    .window(window::Settings {
        size: iced::Size::new(800.0, 600.0),
        position: window::Position::Centered,
        visible: false,
        icon: load_icon(),
        ..Default::default()
    })
    .run()
}
