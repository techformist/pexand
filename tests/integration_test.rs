use pexand::db::{Bootstrapper, Snippet, SnippetManager};
use std::env;
use std::fs;

#[test]
fn test_full_database_workflow() {
    // Set up a temporary database for testing
    let temp_dir = env::temp_dir();
    let db_path = temp_dir.join("test_pexand_integration.db");

    // Clean up any existing test database
    let _ = fs::remove_file(&db_path);

    // Initialize database manually for testing
    let conn = rusqlite::Connection::open(&db_path).unwrap();
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

    // Test 1: Seed defaults with bootstrapper
    Bootstrapper::seed_defaults(&conn).unwrap();

    let manager = SnippetManager::new(&conn);

    // Test 2: Verify default snippets exist
    let name_snippet = manager.read(";name").unwrap();
    assert!(name_snippet.is_some(), "Default ;name snippet should exist");
    assert_eq!(name_snippet.unwrap().body, "Neo Anderson");

    let email_snippet = manager.read(";email").unwrap();
    assert!(
        email_snippet.is_some(),
        "Default ;email snippet should exist"
    );
    assert_eq!(email_snippet.unwrap().body, "neo@matrix.com");

    let date_snippet = manager.read(";date").unwrap();
    assert!(date_snippet.is_some(), "Default ;date snippet should exist");
    assert_eq!(date_snippet.unwrap().body, "{{date:%Y-%m-%d}}");

    // Test 3: Add a new snippet
    let new_snippet = Snippet::new(";signature".to_string(), "Best regards,\nNeo".to_string());
    manager.create(&new_snippet).unwrap();

    let retrieved = manager.read(";signature").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().body, "Best regards,\nNeo");

    // Test 4: Update existing snippet
    let mut update_snippet = manager.read(";email").unwrap().unwrap();
    update_snippet.body = "neo@thematrix.net".to_string();
    update_snippet.touch();
    manager.update(&update_snippet).unwrap();

    let updated = manager.read(";email").unwrap().unwrap();
    assert_eq!(updated.body, "neo@thematrix.net");

    // Test 5: List all snippets
    let all_snippets = manager.list_all().unwrap();
    assert_eq!(all_snippets.len(), 4, "Should have 4 snippets total");

    // Test 6: Increment usage count
    manager.increment_usage(";name").unwrap();
    manager.increment_usage(";name").unwrap();
    manager.increment_usage(";name").unwrap();

    let name_after_use = manager.read(";name").unwrap().unwrap();
    assert_eq!(name_after_use.usage_count, 3);

    // Test 7: Delete a snippet
    manager.delete(";signature").unwrap();
    let deleted = manager.read(";signature").unwrap();
    assert!(deleted.is_none(), "Deleted snippet should not exist");

    // Test 8: Verify database persists across "restarts"
    drop(manager);
    drop(conn);

    // Reopen connection
    let conn2 = rusqlite::Connection::open(&db_path).unwrap();
    let manager2 = SnippetManager::new(&conn2);

    // Verify data persists
    let persisted_name = manager2.read(";name").unwrap().unwrap();
    assert_eq!(persisted_name.body, "Neo Anderson");
    assert_eq!(persisted_name.usage_count, 3, "Usage count should persist");

    let persisted_email = manager2.read(";email").unwrap().unwrap();
    assert_eq!(
        persisted_email.body, "neo@thematrix.net",
        "Updated body should persist"
    );

    // Verify deleted snippet stays deleted
    let still_deleted = manager2.read(";signature").unwrap();
    assert!(still_deleted.is_none());

    // Clean up
    drop(manager2);
    drop(conn2);
    let _ = fs::remove_file(&db_path);
}

#[test]
fn test_bootstrapper_seeds_only_once() {
    let temp_dir = env::temp_dir();
    let db_path = temp_dir.join("test_pexand_bootstrap.db");

    // Clean up any existing test database
    let _ = fs::remove_file(&db_path);

    // Initialize database
    let conn = rusqlite::Connection::open(&db_path).unwrap();
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

    // First seed
    Bootstrapper::seed_defaults(&conn).unwrap();

    let manager = SnippetManager::new(&conn);
    let first_count = manager.list_all().unwrap().len();
    assert_eq!(first_count, 3);

    // Try to seed again
    Bootstrapper::seed_defaults(&conn).unwrap();

    let second_count = manager.list_all().unwrap().len();
    assert_eq!(second_count, 3, "Should not duplicate defaults");

    // Clean up
    drop(manager);
    drop(conn);
    let _ = fs::remove_file(&db_path);
}
