use super::{Snippet, SnippetManager};
use rusqlite::Connection;

/// Bootstrapper for initializing the database with default snippets on first run
pub struct Bootstrapper;

impl Bootstrapper {
    /// Seed the database with default snippets if it's empty
    pub fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
        let manager = SnippetManager::new(conn);

        // Only seed if database is empty
        if !manager.is_empty()? {
            return Ok(());
        }

        // Seed default snippets with commonly used shortcuts
        let defaults = vec![
            Snippet::new(";email".to_string(), "your.email@example.com".to_string()),
            Snippet::new(";phone".to_string(), "+1 (555) 123-4567".to_string()),
            Snippet::new(";addr".to_string(), "123 Main Street\nCity, State 12345\nUnited States".to_string()),
            Snippet::new(";sig".to_string(), "Best regards,\nYour Name\nyour.email@example.com".to_string()),
            Snippet::new(";meeting".to_string(), "Hi team,\n\nLet's schedule a meeting to discuss:\n- Topic 1\n- Topic 2\n- Topic 3\n\nPlease share your availability.\n\nThanks!".to_string()),
            Snippet::new(";date".to_string(), "{{date:%Y-%m-%d}}".to_string()),
            Snippet::new(";time".to_string(), "{{date:%H:%M:%S}}".to_string()),
            Snippet::new(";lorem".to_string(), "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".to_string()),
            Snippet::new(";thanks".to_string(), "Thank you for your email. I appreciate you taking the time to reach out.".to_string()),
            Snippet::new(";followup".to_string(), "Following up on my previous email. Did you have a chance to review this?".to_string()),
            Snippet::new(";schedule".to_string(), "I'm available on:\n- Monday 2-4 PM\n- Wednesday 10 AM - 12 PM\n- Friday 3-5 PM\n\nLet me know what works for you.".to_string()),
            Snippet::new(";code".to_string(), "```\n// Your code here\n```".to_string()),
        ];

        for snippet in defaults {
            manager.create(&snippet)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE snippets (
                trigger TEXT PRIMARY KEY NOT NULL,
                label TEXT,
                body TEXT NOT NULL,
                usage_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_seed_defaults_on_empty_db() {
        let conn = setup_test_db();

        Bootstrapper::seed_defaults(&conn).unwrap();

        let manager = SnippetManager::new(&conn);

        // Verify defaults were created
        assert!(manager.read(";email").unwrap().is_some());
        assert!(manager.read(";phone").unwrap().is_some());
        assert!(manager.read(";date").unwrap().is_some());

        // Verify content
        let email_snippet = manager.read(";email").unwrap().unwrap();
        assert_eq!(email_snippet.body, "your.email@example.com");

        let date_snippet = manager.read(";date").unwrap().unwrap();
        assert_eq!(date_snippet.body, "{{date:%Y-%m-%d}}");
    }

    #[test]
    fn test_seed_only_once() {
        let conn = setup_test_db();

        // First seed
        Bootstrapper::seed_defaults(&conn).unwrap();

        let manager = SnippetManager::new(&conn);
        let all_snippets = manager.list_all().unwrap();
        assert_eq!(all_snippets.len(), 12); // Updated from 3 to 12

        // Try to seed again - should do nothing
        Bootstrapper::seed_defaults(&conn).unwrap();

        let all_snippets_after = manager.list_all().unwrap();
        assert_eq!(all_snippets_after.len(), 12);
    }
}
