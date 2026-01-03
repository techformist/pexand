use enigo::{Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Text injector that simulates backspace and typing
pub struct Injector {
    enigo: Enigo,
    delay_ms: u64,
}

impl Injector {
    /// Create a new injector with default settings
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap_or_else(|_| {
                panic!("Failed to create Enigo instance")
            }),
            delay_ms: 10, // 10ms delay between keystrokes
        }
    }

    /// Create a new injector with custom delay
    pub fn with_delay(delay_ms: u64) -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap_or_else(|_| {
                panic!("Failed to create Enigo instance")
            }),
            delay_ms,
        }
    }

    /// Delete the trigger text by simulating backspaces
    pub fn delete_trigger(&mut self, trigger_length: usize) {
        for _ in 0..trigger_length {
            let _ = self.enigo.key(Key::Backspace, enigo::Direction::Click);
            thread::sleep(Duration::from_millis(self.delay_ms));
        }
    }

    /// Type the expansion text character by character
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            // Handle newlines specially
            if ch == '\n' {
                let _ = self.enigo.key(Key::Return, enigo::Direction::Click);
            } else {
                let _ = self.enigo.text(&ch.to_string());
            }
            thread::sleep(Duration::from_millis(self.delay_ms));
        }
    }

    /// Perform a complete expansion: delete trigger and type expansion
    pub fn expand(&mut self, trigger: &str, expansion: &str) {
        // Delete the trigger
        self.delete_trigger(trigger.len());
        
        // Small delay before typing
        thread::sleep(Duration::from_millis(self.delay_ms * 2));
        
        // Type the expansion
        self.type_text(expansion);
    }

    /// Set the delay between keystrokes
    pub fn set_delay(&mut self, delay_ms: u64) {
        self.delay_ms = delay_ms;
    }
}

impl Default for Injector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injector_creation() {
        let injector = Injector::new();
        assert_eq!(injector.delay_ms, 10);
    }

    #[test]
    fn test_injector_with_delay() {
        let injector = Injector::with_delay(20);
        assert_eq!(injector.delay_ms, 20);
    }

    #[test]
    fn test_set_delay() {
        let mut injector = Injector::new();
        injector.set_delay(15);
        assert_eq!(injector.delay_ms, 15);
    }

    // Note: We cannot test actual keyboard injection in unit tests
    // as it requires a real desktop environment and would interfere
    // with the test process. These would need integration tests
    // in a controlled environment.
}
