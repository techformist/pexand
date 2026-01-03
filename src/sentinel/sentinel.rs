use super::{Buffer, Injector, Trie, VariableParser};
use crate::db::{Database, SnippetManager};
use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Messages that can be sent to the Sentinel
pub enum SentinelMessage {
    /// Reload the Trie from the database
    ReloadTrie,
    /// Shutdown the Sentinel
    Shutdown,
}

/// The Sentinel monitors keystrokes and performs text expansion
pub struct Sentinel {
    buffer: Arc<Mutex<Buffer>>,
    trie: Arc<Mutex<Trie>>,
    injector: Arc<Mutex<Injector>>,
    db_path: String,
}

impl Sentinel {
    /// Create a new Sentinel
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let buffer = Arc::new(Mutex::new(Buffer::new(50)));
        let trie = Arc::new(Mutex::new(Trie::new()));
        let injector = Arc::new(Mutex::new(Injector::new()));

        // Get database path
        let db = Database::init()?;
        let db_path = get_db_path();

        // Load triggers into Trie
        let conn = db.connection();
        let manager = SnippetManager::new(conn);
        let triggers = manager.get_all_triggers()?;

        let mut trie_guard = trie.lock().unwrap();
        trie_guard.load_triggers(&triggers);
        drop(trie_guard);

        Ok(Self {
            buffer,
            trie,
            injector,
            db_path,
        })
    }

    /// Reload triggers from the database
    pub fn reload_trie(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let manager = SnippetManager::new(&conn);
        let triggers = manager.get_all_triggers()?;

        let mut trie = self.trie.lock().unwrap();
        trie.clear();
        trie.load_triggers(&triggers);

        Ok(())
    }

    /// Start the Sentinel in a background thread
    pub fn start(self) -> (thread::JoinHandle<()>, Sender<SentinelMessage>) {
        let (tx, rx) = channel();

        let handle = thread::spawn(move || {
            self.run(rx);
        });

        (handle, tx)
    }

    /// Main run loop for the Sentinel
    fn run(self, rx: Receiver<SentinelMessage>) {
        let buffer = Arc::clone(&self.buffer);
        let trie = Arc::clone(&self.trie);
        let injector = Arc::clone(&self.injector);
        let db_path = self.db_path.clone();

        // Spawn keyboard listener thread
        let buffer_clone = Arc::clone(&buffer);
        let trie_clone = Arc::clone(&trie);
        let injector_clone = Arc::clone(&injector);
        let db_path_clone = db_path.clone();

        thread::spawn(move || {
            let callback = move |event: Event| {
                if let EventType::KeyPress(key) = event.event_type {
                    handle_key_press(
                        key,
                        &buffer_clone,
                        &trie_clone,
                        &injector_clone,
                        &db_path_clone,
                    );
                }
            };

            if let Err(e) = listen(callback) {
                eprintln!("Keyboard listener error: {:?}", e);
            }
        });

        // Message handling loop
        loop {
            match rx.recv() {
                Ok(SentinelMessage::ReloadTrie) => {
                    if let Err(e) = self.reload_trie() {
                        eprintln!("Failed to reload trie: {:?}", e);
                    }
                }
                Ok(SentinelMessage::Shutdown) => {
                    break;
                }
                Err(_) => {
                    // Channel closed, exit
                    break;
                }
            }
        }
    }
}

/// Handle a key press event
fn handle_key_press(
    key: Key,
    buffer: &Arc<Mutex<Buffer>>,
    trie: &Arc<Mutex<Trie>>,
    injector: &Arc<Mutex<Injector>>,
    db_path: &str,
) {
    // Check if this is a delimiter key (space, enter, tab, punctuation)
    let is_delimiter = matches!(
        key,
        Key::Space
            | Key::Return
            | Key::Tab
            | Key::Comma
            | Key::Dot
            | Key::SemiColon
            | Key::Slash
            | Key::BackSlash
    );

    // Convert key to character
    let ch = match key_to_char(key) {
        Some(c) => c,
        None => {
            // For non-character keys, check if it's a delimiter that should trigger expansion
            if is_delimiter {
                // Check buffer for trigger match before the delimiter
                let buffer_guard = buffer.lock().unwrap();
                let buffer_text = buffer_guard.as_string();
                drop(buffer_guard);

                let trie_guard = trie.lock().unwrap();
                if let Some(trigger) = trie_guard.find_matching_trigger(&buffer_text) {
                    drop(trie_guard);
                    perform_expansion(&trigger, buffer, injector, db_path, true);
                }
            }
            return;
        }
    };

    // Add character to buffer
    let mut buffer_guard = buffer.lock().unwrap();
    buffer_guard.push(ch);
    let buffer_text = buffer_guard.as_string();
    drop(buffer_guard);

    // If this character is a delimiter, check for trigger match
    if is_delimiter {
        let trie_guard = trie.lock().unwrap();
        if let Some(trigger) = trie_guard.find_matching_trigger(&buffer_text) {
            drop(trie_guard);
            perform_expansion(&trigger, buffer, injector, db_path, true);
        }
    }
}

/// Perform the actual text expansion
fn perform_expansion(
    trigger: &str,
    buffer: &Arc<Mutex<Buffer>>,
    injector: &Arc<Mutex<Injector>>,
    db_path: &str,
    include_delimiter: bool,
) {
    // Get expansion from database
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let manager = SnippetManager::new(&conn);

        if let Ok(Some(snippet)) = manager.read(trigger) {
            // Parse variables in the body
            let mut parser = VariableParser::new();
            let expanded_body = parser.parse(&snippet.body).unwrap_or(snippet.body.clone());

            // Wait a moment to ensure all characters are on screen
            thread::sleep(std::time::Duration::from_millis(50));

            // Perform expansion
            let mut injector_guard = injector.lock().unwrap();
            // Delete the trigger + delimiter (if present)
            let delete_count = if include_delimiter {
                trigger.len() + 1
            } else {
                trigger.len()
            };
            injector_guard.delete_trigger(delete_count);
            injector_guard.type_text(&expanded_body);
            drop(injector_guard);

            // Increment usage count
            let _ = manager.increment_usage(trigger);

            // Clear buffer after expansion
            let mut buffer_guard = buffer.lock().unwrap();
            buffer_guard.clear();
        }
    }
}

/// Convert rdev Key to character
fn key_to_char(key: Key) -> Option<char> {
    match key {
        Key::KeyA => Some('a'),
        Key::KeyB => Some('b'),
        Key::KeyC => Some('c'),
        Key::KeyD => Some('d'),
        Key::KeyE => Some('e'),
        Key::KeyF => Some('f'),
        Key::KeyG => Some('g'),
        Key::KeyH => Some('h'),
        Key::KeyI => Some('i'),
        Key::KeyJ => Some('j'),
        Key::KeyK => Some('k'),
        Key::KeyL => Some('l'),
        Key::KeyM => Some('m'),
        Key::KeyN => Some('n'),
        Key::KeyO => Some('o'),
        Key::KeyP => Some('p'),
        Key::KeyQ => Some('q'),
        Key::KeyR => Some('r'),
        Key::KeyS => Some('s'),
        Key::KeyT => Some('t'),
        Key::KeyU => Some('u'),
        Key::KeyV => Some('v'),
        Key::KeyW => Some('w'),
        Key::KeyX => Some('x'),
        Key::KeyY => Some('y'),
        Key::KeyZ => Some('z'),
        Key::Num0 => Some('0'),
        Key::Num1 => Some('1'),
        Key::Num2 => Some('2'),
        Key::Num3 => Some('3'),
        Key::Num4 => Some('4'),
        Key::Num5 => Some('5'),
        Key::Num6 => Some('6'),
        Key::Num7 => Some('7'),
        Key::Num8 => Some('8'),
        Key::Num9 => Some('9'),
        Key::Space => Some(' '),
        Key::SemiColon => Some(';'),
        Key::Equal => Some('='),
        Key::Comma => Some(','),
        Key::Minus => Some('-'),
        Key::Dot => Some('.'),
        Key::Slash => Some('/'),
        Key::BackQuote => Some('`'),
        Key::LeftBracket => Some('['),
        Key::BackSlash => Some('\\'),
        Key::RightBracket => Some(']'),
        Key::Quote => Some('\''),
        _ => None,
    }
}

/// Get the database path (same logic as in Database)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_char() {
        assert_eq!(key_to_char(Key::KeyA), Some('a'));
        assert_eq!(key_to_char(Key::Space), Some(' '));
        assert_eq!(key_to_char(Key::SemiColon), Some(';'));
        assert_eq!(key_to_char(Key::Escape), None);
    }

    #[test]
    fn test_get_db_path() {
        let path = get_db_path();
        assert!(!path.is_empty());
    }
}
