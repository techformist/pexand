//! Editor state and operations

use iced::widget::text_editor;

/// State for the snippet editor
#[derive(Debug)]
pub struct EditorState {
    pub trigger: String,
    pub label: String,
    pub body: String,
    pub body_content: text_editor::Content,
    pub editing_index: Option<usize>,
}

impl EditorState {
    /// Create a new editor state for a new snippet
    pub fn new() -> Self {
        Self {
            trigger: String::new(),
            label: String::new(),
            body: String::new(),
            body_content: text_editor::Content::new(),
            editing_index: None,
        }
    }

    /// Create an editor state for editing an existing snippet
    pub fn edit(trigger: String, label: String, body: String, index: usize) -> Self {
        let body_content = text_editor::Content::with_text(&body);
        Self {
            trigger,
            label,
            body,
            body_content,
            editing_index: Some(index),
        }
    }

    /// Check if required fields are filled
    pub fn is_valid(&self) -> bool {
        !self.trigger.is_empty() && !self.body.is_empty()
    }
}
