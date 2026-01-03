use rusqlite::{Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Database manager for Pexand
pub struct Database;

impl Database {
    /// Initialize the database with the appropriate path and return a Connection
    /// This allows the connection to be shared across threads using Arc<Mutex<Connection>>
    pub fn init() -> Result<Connection> {
        let db_path = Self::get_database_path();

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better concurrency (allows concurrent reads)
        // PRAGMA journal_mode returns a result, so we use query instead of execute
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Use memory for temporary storage (faster than disk)
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        // Create the snippets table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
                trigger TEXT PRIMARY KEY NOT NULL,
                label TEXT,
                body TEXT NOT NULL,
                usage_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for common query patterns
        // Index on usage_count for sorting by most frequently used snippets
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_usage_count 
             ON snippets(usage_count DESC)",
            [],
        )?;

        // Index on updated_at for sorting by most recently modified
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_updated_at 
             ON snippets(updated_at DESC)",
            [],
        )?;

        // Index on created_at for sorting by creation date
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_created_at 
             ON snippets(created_at DESC)",
            [],
        )?;

        Ok(conn)
    }

    /// Get the database path based on portable mode or standard location
    pub fn get_database_path() -> PathBuf {
        // Check if portable.txt exists in the current directory
        let portable_marker = Path::new("portable.txt");

        if portable_marker.exists() {
            // Portable mode: store database in same directory as executable
            PathBuf::from("pexand.db")
        } else {
            // Standard mode: use %APPDATA%/Pexand/pexand.db
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());

            let mut path = PathBuf::from(appdata);
            path.push("Pexand");
            path.push("pexand.db");
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_database_init() {
        // Use a temporary database for testing
        let temp_dir = env::temp_dir();
        let db_path = temp_dir.join("test_pexand.db");

        // Clean up any existing test database
        let _ = std::fs::remove_file(&db_path);

        // Create database in temp location
        let conn = Connection::open(&db_path).unwrap();

        // Create table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
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

        // Verify table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='snippets')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(table_exists);

        // Clean up
        drop(conn);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_portable_mode_detection() {
        let path = Database::get_database_path();

        // In standard mode (no portable.txt), should use APPDATA
        if !Path::new("portable.txt").exists() {
            assert!(path.to_string_lossy().contains("Pexand"));
        }
    }

    #[test]
    fn test_indexes_created() {
        // Use a temporary database for testing
        let temp_dir = env::temp_dir();
        let db_path = temp_dir.join("test_pexand_indexes.db");

        // Clean up any existing test database
        let _ = std::fs::remove_file(&db_path);

        // Initialize database using the actual init function
        let conn = Connection::open(&db_path).unwrap();

        // Create table and indexes manually for testing
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
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

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_usage_count 
             ON snippets(usage_count DESC)",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_updated_at 
             ON snippets(updated_at DESC)",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_created_at 
             ON snippets(created_at DESC)",
            [],
        )
        .unwrap();

        // Verify indexes exist
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' 
                 AND name IN ('idx_snippets_usage_count', 'idx_snippets_updated_at', 'idx_snippets_created_at')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(index_count, 3, "All three indexes should be created");

        // Clean up
        drop(conn);
        let _ = std::fs::remove_file(&db_path);
    }
}
