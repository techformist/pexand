use rusqlite::{params, Connection, Result};
use super::Snippet;

/// Manager for snippet CRUD operations
pub struct SnippetManager<'a> {
    conn: &'a Connection,
}

impl<'a> SnippetManager<'a> {
    /// Create a new SnippetManager with a database connection
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new snippet in the database
    pub fn create(&self, snippet: &Snippet) -> Result<()> {
        snippet.validate()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e,
            ))))?;

        self.conn.execute(
            "INSERT INTO snippets (trigger, label, body, usage_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snippet.trigger,
                snippet.label,
                snippet.body,
                snippet.usage_count,
                snippet.created_at,
                snippet.updated_at
            ],
        )?;
        Ok(())
    }

    /// Read a snippet by trigger
    pub fn read(&'a self, trigger: &str) -> Result<Option<Snippet>> {
        let mut stmt = self.conn.prepare(
            "SELECT trigger, label, body, usage_count, created_at, updated_at 
             FROM snippets WHERE trigger = ?1"
        )?;

        let mut rows = stmt.query(params![trigger])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(Snippet::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Update an existing snippet
    pub fn update(&self, snippet: &Snippet) -> Result<()> {
        snippet.validate()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e,
            ))))?;

        self.conn.execute(
            "UPDATE snippets 
             SET label = ?1, body = ?2, usage_count = ?3, updated_at = ?4
             WHERE trigger = ?5",
            params![
                snippet.label,
                snippet.body,
                snippet.usage_count,
                snippet.updated_at,
                snippet.trigger
            ],
        )?;
        Ok(())
    }

    /// Delete a snippet by trigger
    pub fn delete(&'a self, trigger: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM snippets WHERE trigger = ?1",
            params![trigger],
        )?;
        Ok(())
    }

    /// List all snippets
    pub fn list_all(&'a self) -> Result<Vec<Snippet>> {
        let mut stmt = self.conn.prepare(
            "SELECT trigger, label, body, usage_count, created_at, updated_at 
             FROM snippets 
             ORDER BY trigger"
        )?;

        let snippet_iter = stmt.query_map([], |row| {
            Snippet::from_row(row)
        })?;

        let mut snippets = Vec::new();
        for snippet in snippet_iter {
            snippets.push(snippet?);
        }

        Ok(snippets)
    }

    /// Check if the database is empty (no snippets)
    pub fn is_empty(&'a self) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM snippets",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Increment the usage count for a snippet
    pub fn increment_usage(&'a self, trigger: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE snippets SET usage_count = usage_count + 1 WHERE trigger = ?1",
            params![trigger],
        )?;
        Ok(())
    }

    /// Get all triggers (for building Trie)
    pub fn get_all_triggers(&'a self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT trigger FROM snippets")?;
        let trigger_iter = stmt.query_map([], |row| row.get(0))?;

        let mut triggers = Vec::new();
        for trigger in trigger_iter {
            triggers.push(trigger?);
        }

        Ok(triggers)
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
        ).unwrap();
        conn
    }

    #[test]
    fn test_create_and_read() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let snippet = Snippet::new(";name".to_string(), "Neo Anderson".to_string());
        manager.create(&snippet).unwrap();

        let retrieved = manager.read(";name").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().body, "Neo Anderson");
    }

    #[test]
    fn test_create_duplicate_fails() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let snippet1 = Snippet::new(";name".to_string(), "Neo Anderson".to_string());
        manager.create(&snippet1).unwrap();

        let snippet2 = Snippet::new(";name".to_string(), "Another Name".to_string());
        let result = manager.create(&snippet2);
        assert!(result.is_err());
    }

    #[test]
    fn test_update() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let mut snippet = Snippet::new(";email".to_string(), "old@example.com".to_string());
        manager.create(&snippet).unwrap();

        snippet.body = "new@example.com".to_string();
        snippet.touch();
        manager.update(&snippet).unwrap();

        let retrieved = manager.read(";email").unwrap().unwrap();
        assert_eq!(retrieved.body, "new@example.com");
    }

    #[test]
    fn test_delete() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let snippet = Snippet::new(";temp".to_string(), "temporary".to_string());
        manager.create(&snippet).unwrap();

        manager.delete(";temp").unwrap();

        let retrieved = manager.read(";temp").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_all() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let snippet1 = Snippet::new(";a".to_string(), "first".to_string());
        let snippet2 = Snippet::new(";b".to_string(), "second".to_string());
        let snippet3 = Snippet::new(";c".to_string(), "third".to_string());

        manager.create(&snippet1).unwrap();
        manager.create(&snippet2).unwrap();
        manager.create(&snippet3).unwrap();

        let all = manager.list_all().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].trigger, ";a");
        assert_eq!(all[1].trigger, ";b");
        assert_eq!(all[2].trigger, ";c");
    }

    #[test]
    fn test_is_empty() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        assert!(manager.is_empty().unwrap());

        let snippet = Snippet::new(";test".to_string(), "test".to_string());
        manager.create(&snippet).unwrap();

        assert!(!manager.is_empty().unwrap());
    }

    #[test]
    fn test_increment_usage() {
        let conn = setup_test_db();
        let manager = SnippetManager::new(&conn);

        let snippet = Snippet::new(";counter".to_string(), "count".to_string());
        manager.create(&snippet).unwrap();

        manager.increment_usage(";counter").unwrap();
        manager.increment_usage(";counter").unwrap();

        let retrieved = manager.read(";counter").unwrap().unwrap();
        assert_eq!(retrieved.usage_count, 2);
    }
}
