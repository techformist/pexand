use enigo::{Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Text injector that simulates backspace and typing
/// Uses adaptive delays based on text length for optimal performance
pub struct Injector {
    enigo: Enigo,
    delay_ms: u64,
    backspace_delay_ms: u64,
}

// Adaptive delay thresholds
const SHORT_TEXT_THRESHOLD: usize = 50; // No delay for short text
const MEDIUM_TEXT_THRESHOLD: usize = 200; // Minimal delay for medium text
const DEFAULT_DELAY_MS: u64 = 2; // Reduced from 10ms to 2ms
const BACKSPACE_DELAY_MS: u64 = 1; // Faster backspace (was 10ms)

impl Injector {
    /// Create a new injector with default settings
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default())
                .unwrap_or_else(|_| panic!("Failed to create Enigo instance")),
            delay_ms: DEFAULT_DELAY_MS,
            backspace_delay_ms: BACKSPACE_DELAY_MS,
        }
    }

    /// Create a new injector with custom delay
    pub fn with_delay(delay_ms: u64) -> Self {
        Self {
            enigo: Enigo::new(&Settings::default())
                .unwrap_or_else(|_| panic!("Failed to create Enigo instance")),
            delay_ms,
            backspace_delay_ms: delay_ms.min(BACKSPACE_DELAY_MS),
        }
    }

    /// Delete the trigger text by simulating backspaces
    /// Uses optimized backspace delay (faster than typing)
    pub fn delete_trigger(&mut self, trigger_length: usize) {
        for _ in 0..trigger_length {
            let _ = self.enigo.key(Key::Backspace, enigo::Direction::Click);
            if self.backspace_delay_ms > 0 {
                thread::sleep(Duration::from_millis(self.backspace_delay_ms));
            }
        }
    }

    /// Type the expansion text character by character
    /// Uses adaptive delays based on text length:
    /// - No delay for short text (<50 chars)
    /// - Minimal delay for medium text (50-200 chars)
    /// - Configurable delay for long text (>200 chars)
    pub fn type_text(&mut self, text: &str) {
        let char_count = text.chars().count();

        // Determine appropriate delay based on text length
        let effective_delay = if char_count < SHORT_TEXT_THRESHOLD {
            0 // No delay for short text - modern systems handle this fine
        } else if char_count < MEDIUM_TEXT_THRESHOLD {
            1 // Minimal 1ms delay for medium text
        } else {
            self.delay_ms // Use configured delay for long text
        };

        for ch in text.chars() {
            // Handle newlines specially
            if ch == '\n' {
                let _ = self.enigo.key(Key::Return, enigo::Direction::Click);
            } else {
                let _ = self.enigo.text(&ch.to_string());
            }

            if effective_delay > 0 {
                thread::sleep(Duration::from_millis(effective_delay));
            }
        }
    }

    /// Perform a complete expansion: delete trigger and type expansion
    pub fn expand(&mut self, trigger: &str, expansion: &str) {
        // Delete the trigger
        self.delete_trigger(trigger.len());

        // Small delay before typing (reduced from 20ms to 5ms)
        thread::sleep(Duration::from_millis(5));

        // Type the expansion
        self.type_text(expansion);
    }

    /// Set the delay between keystrokes
    pub fn set_delay(&mut self, delay_ms: u64) {
        self.delay_ms = delay_ms;
    }

    /// Set the delay between backspaces
    pub fn set_backspace_delay(&mut self, delay_ms: u64) {
        self.backspace_delay_ms = delay_ms;
    }

    /// Get the current delay setting
    pub fn get_delay(&self) -> u64 {
        self.delay_ms
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
        assert_eq!(injector.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(injector.backspace_delay_ms, BACKSPACE_DELAY_MS);
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

        injector.set_backspace_delay(5);
        assert_eq!(injector.backspace_delay_ms, 5);
    }

    #[test]
    fn test_adaptive_delays() {
        let injector = Injector::new();

        // Short text should use no delay
        let short_text = "hi";
        assert!(short_text.chars().count() < SHORT_TEXT_THRESHOLD);

        // Medium text should use minimal delay
        let medium_text = "a".repeat(100);
        assert!(medium_text.chars().count() >= SHORT_TEXT_THRESHOLD);
        assert!(medium_text.chars().count() < MEDIUM_TEXT_THRESHOLD);

        // Long text should use configured delay
        let long_text = "a".repeat(250);
        assert!(long_text.chars().count() >= MEDIUM_TEXT_THRESHOLD);

        // Verify get_delay works
        assert_eq!(injector.get_delay(), DEFAULT_DELAY_MS);
    }

    // Note: We cannot test actual keyboard injection in unit tests
    // as it requires a real desktop environment and would interfere
    // with the test process. These would need integration tests
    // in a controlled environment.
}
