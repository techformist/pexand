use super::{Buffer, Injector, Trie, VariableParser};
use crate::db::SnippetManager;
use crossbeam_channel::{bounded, Receiver, Sender};
use rdev::{listen, Event, EventType, Key};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

/// Tracks keyboard modifier state (Shift, etc.)
struct ModifierState {
    shift_pressed: bool,
}

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
    db_conn: Arc<Mutex<Connection>>,
    modifier_state: Arc<Mutex<ModifierState>>,
}

impl Sentinel {
    /// Create a new Sentinel with a shared database connection
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Result<Self, Box<dyn std::error::Error>> {
        let buffer = Arc::new(Mutex::new(Buffer::new(50)));
        let trie = Arc::new(Mutex::new(Trie::new()));
        let injector = Arc::new(Mutex::new(Injector::new()));

        // Load triggers into Trie
        {
            let conn = db_conn.lock().unwrap();
            let manager = SnippetManager::new(&conn);
            let triggers = manager.get_all_triggers()?;

            let mut trie_guard = trie.lock().unwrap();
            trie_guard.load_triggers(&triggers);
        }

        Ok(Self {
            buffer,
            trie,
            injector,
            db_conn,
            modifier_state: Arc::new(Mutex::new(ModifierState {
                shift_pressed: false,
            })),
        })
    }

    /// Reload triggers from the database
    pub fn reload_trie(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.db_conn.lock().unwrap();
        let manager = SnippetManager::new(&conn);
        let triggers = manager.get_all_triggers()?;

        let mut trie = self.trie.lock().unwrap();
        trie.clear();
        trie.load_triggers(&triggers);

        Ok(())
    }

    /// Start the Sentinel in a background thread
    pub fn start(self) -> (thread::JoinHandle<()>, Sender<SentinelMessage>) {
        let (tx, rx) = bounded(10); // Small bounded channel for control messages

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
        let db_conn = Arc::clone(&self.db_conn);

        // Spawn keyboard listener thread
        let buffer_clone = Arc::clone(&buffer);
        let trie_clone = Arc::clone(&trie);
        let injector_clone = Arc::clone(&injector);
        let db_conn_clone = Arc::clone(&db_conn);
        let modifier_state_clone = Arc::clone(&self.modifier_state);

        thread::spawn(move || {
            let callback = move |event: Event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        // Track modifier keys
                        if matches!(key, Key::ShiftLeft | Key::ShiftRight) {
                            let mut state = modifier_state_clone.lock().unwrap();
                            state.shift_pressed = true;
                        } else {
                            handle_key_press(
                                key,
                                &buffer_clone,
                                &trie_clone,
                                &injector_clone,
                                &db_conn_clone,
                                &modifier_state_clone,
                            );
                        }
                    }
                    EventType::KeyRelease(key) => {
                        // Release modifier keys
                        if matches!(key, Key::ShiftLeft | Key::ShiftRight) {
                            let mut state = modifier_state_clone.lock().unwrap();
                            state.shift_pressed = false;
                        }
                    }
                    _ => {}
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
    db_conn: &Arc<Mutex<Connection>>,
    modifier_state: &Arc<Mutex<ModifierState>>,
) {
    // Skip processing if current window is pexand itself
    if is_current_app_blacklisted() {
        return;
    }

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
    let state = modifier_state.lock().unwrap();
    let ch = match key_to_char(key, state.shift_pressed) {
        Some(c) => c,
        None => {
            // For non-character keys, check if it's a delimiter that should trigger expansion
            if is_delimiter {
                // Check buffer for trigger match before the delimiter
                let mut buffer_guard = buffer.lock().unwrap();
                let buffer_text = buffer_guard.as_string().to_string();
                drop(buffer_guard);

                let trie_guard = trie.lock().unwrap();
                // Normalize to lowercase for case-insensitive matching
                let buffer_text_lower = buffer_text.to_lowercase();
                if let Some(trigger) = trie_guard.find_matching_trigger(&buffer_text_lower) {
                    drop(trie_guard);
                    perform_expansion(&trigger, buffer, injector, db_conn, true);
                }
            }
            return;
        }
    };

    // Add character to buffer
    let mut buffer_guard = buffer.lock().unwrap();
    buffer_guard.push(ch);
    let buffer_text = buffer_guard.as_string().to_string();
    drop(buffer_guard);

    // If this character is a delimiter, check for trigger match
    if is_delimiter {
        let trie_guard = trie.lock().unwrap();
        // Normalize to lowercase for case-insensitive matching
        let buffer_text_lower = buffer_text.to_lowercase();
        if let Some(trigger) = trie_guard.find_matching_trigger(&buffer_text_lower) {
            drop(trie_guard);
            perform_expansion(&trigger, buffer, injector, db_conn, true);
        }
    }
}

/// Perform the actual text expansion
fn perform_expansion(
    trigger: &str,
    buffer: &Arc<Mutex<Buffer>>,
    injector: &Arc<Mutex<Injector>>,
    db_conn: &Arc<Mutex<Connection>>,
    include_delimiter: bool,
) {
    // Check if current app is blacklisted
    if is_current_app_blacklisted() {
        return; // Don't expand in blacklisted apps
    }

    // Get expansion from database using shared connection
    let conn = db_conn.lock().unwrap();
    let manager = SnippetManager::new(&conn);

    if let Ok(Some(snippet)) = manager.read(trigger) {
        println!(
            "[SENTINEL] Found snippet, body length: {}",
            snippet.body.len()
        );
        // Parse variables in the body
        let mut parser = VariableParser::new();
        let expanded_body = parser.parse(&snippet.body).unwrap_or(snippet.body.clone());

        // Release the lock before performing IO operations
        drop(conn);

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

        // Increment usage count (reacquire lock)
        let conn = db_conn.lock().unwrap();
        let manager = SnippetManager::new(&conn);
        let _ = manager.increment_usage(trigger);
        drop(conn);

        // Clear buffer after expansion
        let mut buffer_guard = buffer.lock().unwrap();
        buffer_guard.clear();
    }
}

/// Convert rdev Key to character with modifier support
/// Handles Shift key for uppercase letters and special characters
fn key_to_char(key: Key, shift_pressed: bool) -> Option<char> {
    match key {
        // Letters - handle case based on Shift
        Key::KeyA => Some(if shift_pressed { 'A' } else { 'a' }),
        Key::KeyB => Some(if shift_pressed { 'B' } else { 'b' }),
        Key::KeyC => Some(if shift_pressed { 'C' } else { 'c' }),
        Key::KeyD => Some(if shift_pressed { 'D' } else { 'd' }),
        Key::KeyE => Some(if shift_pressed { 'E' } else { 'e' }),
        Key::KeyF => Some(if shift_pressed { 'F' } else { 'f' }),
        Key::KeyG => Some(if shift_pressed { 'G' } else { 'g' }),
        Key::KeyH => Some(if shift_pressed { 'H' } else { 'h' }),
        Key::KeyI => Some(if shift_pressed { 'I' } else { 'i' }),
        Key::KeyJ => Some(if shift_pressed { 'J' } else { 'j' }),
        Key::KeyK => Some(if shift_pressed { 'K' } else { 'k' }),
        Key::KeyL => Some(if shift_pressed { 'L' } else { 'l' }),
        Key::KeyM => Some(if shift_pressed { 'M' } else { 'm' }),
        Key::KeyN => Some(if shift_pressed { 'N' } else { 'n' }),
        Key::KeyO => Some(if shift_pressed { 'O' } else { 'o' }),
        Key::KeyP => Some(if shift_pressed { 'P' } else { 'p' }),
        Key::KeyQ => Some(if shift_pressed { 'Q' } else { 'q' }),
        Key::KeyR => Some(if shift_pressed { 'R' } else { 'r' }),
        Key::KeyS => Some(if shift_pressed { 'S' } else { 's' }),
        Key::KeyT => Some(if shift_pressed { 'T' } else { 't' }),
        Key::KeyU => Some(if shift_pressed { 'U' } else { 'u' }),
        Key::KeyV => Some(if shift_pressed { 'V' } else { 'v' }),
        Key::KeyW => Some(if shift_pressed { 'W' } else { 'w' }),
        Key::KeyX => Some(if shift_pressed { 'X' } else { 'x' }),
        Key::KeyY => Some(if shift_pressed { 'Y' } else { 'y' }),
        Key::KeyZ => Some(if shift_pressed { 'Z' } else { 'z' }),

        // Numbers - handle Shift for special characters (US layout)
        Key::Num0 => Some(if shift_pressed { ')' } else { '0' }),
        Key::Num1 => Some(if shift_pressed { '!' } else { '1' }),
        Key::Num2 => Some(if shift_pressed { '@' } else { '2' }),
        Key::Num3 => Some(if shift_pressed { '#' } else { '3' }),
        Key::Num4 => Some(if shift_pressed { '$' } else { '4' }),
        Key::Num5 => Some(if shift_pressed { '%' } else { '5' }),
        Key::Num6 => Some(if shift_pressed { '^' } else { '6' }),
        Key::Num7 => Some(if shift_pressed { '&' } else { '7' }),
        Key::Num8 => Some(if shift_pressed { '*' } else { '8' }),
        Key::Num9 => Some(if shift_pressed { '(' } else { '9' }),

        // Special characters
        Key::Space => Some(' '),
        Key::SemiColon => Some(if shift_pressed { ':' } else { ';' }),
        Key::Equal => Some(if shift_pressed { '+' } else { '=' }),
        Key::Comma => Some(if shift_pressed { '<' } else { ',' }),
        Key::Minus => Some(if shift_pressed { '_' } else { '-' }),
        Key::Dot => Some(if shift_pressed { '>' } else { '.' }),
        Key::Slash => Some(if shift_pressed { '?' } else { '/' }),
        Key::BackQuote => Some(if shift_pressed { '~' } else { '`' }),
        Key::LeftBracket => Some(if shift_pressed { '{' } else { '[' }),
        Key::BackSlash => Some(if shift_pressed { '|' } else { '\\' }),
        Key::RightBracket => Some(if shift_pressed { '}' } else { ']' }),
        Key::Quote => Some(if shift_pressed { '"' } else { '\'' }),

        // Ignore modifier keys themselves
        Key::ShiftLeft
        | Key::ShiftRight
        | Key::ControlLeft
        | Key::ControlRight
        | Key::Alt
        | Key::AltGr
        | Key::MetaLeft
        | Key::MetaRight => None,

        // All other keys (function keys, arrows, etc.)
        _ => None,
    }
}

/// Check if the current foreground application is blacklisted
fn is_current_app_blacklisted() -> bool {
    // Get the foreground window's executable name
    let current_exe = match get_foreground_window_exe() {
        Some(exe) => exe.to_lowercase(),
        None => return false, // If we can't get the window, allow expansion
    };

    // Always block pexand itself to avoid capturing keystrokes in its own UI
    if current_exe == "pexand.exe" {
        return true;
    }

    // Load blacklist from settings
    let blacklist = load_block_apps();

    // Check if current exe is in the blacklist (case-insensitive)
    blacklist
        .iter()
        .any(|blocked| blocked.to_lowercase() == current_exe)
}

/// Load blacklisted apps from settings
fn load_block_apps() -> Vec<String> {
    let path = get_settings_path("block_apps.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Get settings file path
fn get_settings_path(filename: &str) -> PathBuf {
    use std::path::PathBuf;

    let portable_marker = Path::new("portable.txt");

    if portable_marker.exists() {
        PathBuf::from(filename)
    } else {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(appdata);
        path.push("Pexand");
        path.push(filename);
        path
    }
}

/// Get the executable name of the foreground window (Windows-specific)
#[cfg(target_os = "windows")]
fn get_foreground_window_exe() -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        // Get the foreground window
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }

        // Get the process ID
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if process_id == 0 {
            return None;
        }

        // Open the process
        let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)
        {
            Ok(handle) => handle,
            Err(_) => return None,
        };

        // Get the executable path
        let mut buffer: [u16; 260] = [0; 260];
        let mut size: u32 = buffer.len() as u32;
        let pwstr = PWSTR(buffer.as_mut_ptr());

        match QueryFullProcessImageNameW(process_handle, PROCESS_NAME_WIN32, pwstr, &mut size) {
            Ok(_) => {
                let path = String::from_utf16_lossy(&buffer[..size as usize]);
                // Extract just the filename from the full path
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            }
            Err(_) => None,
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_window_exe() -> Option<String> {
    // Not implemented for non-Windows platforms
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_char() {
        // Test lowercase letters (no shift)
        assert_eq!(key_to_char(Key::KeyA, false), Some('a'));
        assert_eq!(key_to_char(Key::KeyZ, false), Some('z'));

        // Test uppercase letters (with shift)
        assert_eq!(key_to_char(Key::KeyA, true), Some('A'));
        assert_eq!(key_to_char(Key::KeyZ, true), Some('Z'));

        // Test numbers without shift
        assert_eq!(key_to_char(Key::Num1, false), Some('1'));
        assert_eq!(key_to_char(Key::Num0, false), Some('0'));

        // Test special characters with shift
        assert_eq!(key_to_char(Key::Num1, true), Some('!'));
        assert_eq!(key_to_char(Key::Num2, true), Some('@'));
        assert_eq!(key_to_char(Key::Num0, true), Some(')'));

        // Test punctuation
        assert_eq!(key_to_char(Key::Space, false), Some(' '));
        assert_eq!(key_to_char(Key::SemiColon, false), Some(';'));
        assert_eq!(key_to_char(Key::SemiColon, true), Some(':'));
        assert_eq!(key_to_char(Key::Comma, false), Some(','));
        assert_eq!(key_to_char(Key::Comma, true), Some('<'));

        // Test non-character keys
        assert_eq!(key_to_char(Key::Escape, false), None);
        assert_eq!(key_to_char(Key::ShiftLeft, false), None);
        assert_eq!(key_to_char(Key::ControlLeft, false), None);
    }
}
