use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a text expansion snippet stored in the database
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    /// The trigger text that will be expanded (e.g., ";name")
    pub trigger: String,
    /// Optional label/description for the snippet
    pub label: Option<String>,
    /// The expansion text that replaces the trigger
    pub body: String,
    /// Number of times this snippet has been used
    pub usage_count: i64,
    /// Unix timestamp when the snippet was created
    pub created_at: i64,
    /// Unix timestamp when the snippet was last updated
    pub updated_at: i64,
}

impl Snippet {
    /// Creates a new snippet with the given trigger and body
    pub fn new(trigger: String, body: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Self {
            trigger,
            label: None,
            body,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new snippet with all fields specified
    pub fn with_label(trigger: String, label: Option<String>, body: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Self {
            trigger,
            label,
            body,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Validates that the snippet meets all requirements
    pub fn validate(&self) -> Result<(), String> {
        if self.trigger.is_empty() {
            return Err("Trigger cannot be empty".to_string());
        }
        if self.body.is_empty() {
            return Err("Body cannot be empty".to_string());
        }
        Ok(())
    }

    /// Creates a Snippet from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            trigger: row.get(0)?,
            label: row.get(1)?,
            body: row.get(2)?,
            usage_count: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_snippet() {
        let snippet = Snippet::new(";name".to_string(), "John Doe".to_string());
        assert_eq!(snippet.trigger, ";name");
        assert_eq!(snippet.body, "John Doe");
        assert_eq!(snippet.label, None);
        assert_eq!(snippet.usage_count, 0);
        assert!(snippet.created_at > 0);
        assert!(snippet.updated_at > 0);
        assert_eq!(snippet.created_at, snippet.updated_at);
    }

    #[test]
    fn test_with_label() {
        let snippet = Snippet::with_label(
            ";email".to_string(),
            Some("Email address".to_string()),
            "john@example.com".to_string(),
        );
        assert_eq!(snippet.trigger, ";email");
        assert_eq!(snippet.label, Some("Email address".to_string()));
        assert_eq!(snippet.body, "john@example.com");
    }

    #[test]
    fn test_validate_empty_trigger() {
        let snippet = Snippet::new("".to_string(), "body".to_string());
        assert!(snippet.validate().is_err());
        assert_eq!(snippet.validate().unwrap_err(), "Trigger cannot be empty");
    }

    #[test]
    fn test_validate_empty_body() {
        let snippet = Snippet::new(";trigger".to_string(), "".to_string());
        assert!(snippet.validate().is_err());
        assert_eq!(snippet.validate().unwrap_err(), "Body cannot be empty");
    }

    #[test]
    fn test_validate_valid_snippet() {
        let snippet = Snippet::new(";trigger".to_string(), "body".to_string());
        assert!(snippet.validate().is_ok());
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut snippet = Snippet::new(";test".to_string(), "test body".to_string());
        let original_updated_at = snippet.updated_at;

        // Sleep long enough to ensure timestamp changes (Unix timestamps are in seconds)
        std::thread::sleep(std::time::Duration::from_secs(1));

        snippet.touch();
        assert!(snippet.updated_at >= original_updated_at);
    }
}
