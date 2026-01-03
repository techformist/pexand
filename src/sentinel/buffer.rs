use std::collections::VecDeque;

/// Rolling buffer that maintains the last N characters typed
pub struct Buffer {
    data: VecDeque<char>,
    max_size: usize,
    cached_string: String,
    dirty: bool,
}

impl Buffer {
    /// Create a new buffer with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size),
            max_size,
            cached_string: String::with_capacity(max_size),
            dirty: false,
        }
    }

    /// Push a character onto the buffer
    /// If the buffer is full, the oldest character is removed
    pub fn push(&mut self, ch: char) {
        if self.data.len() >= self.max_size {
            self.data.pop_front();
        }
        self.data.push_back(ch);
        self.dirty = true;
    }

    /// Get the buffer contents as a string slice
    /// Returns a reference to the cached string representation
    /// The cache is updated only when the buffer has been modified
    pub fn as_string(&mut self) -> &str {
        if self.dirty {
            self.cached_string.clear();
            self.cached_string.extend(self.data.iter());
            self.dirty = false;
        }
        &self.cached_string
    }

    /// Clear all contents from the buffer
    pub fn clear(&mut self) {
        self.data.clear();
        self.cached_string.clear();
        self.dirty = false;
    }

    /// Get the current length of the buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the last N characters as a string
    pub fn last_n(&mut self, n: usize) -> &str {
        // Ensure cache is up to date
        let full_string = self.as_string();

        // Calculate start position for last n chars
        let char_count = full_string.chars().count();
        if n >= char_count {
            return full_string;
        }

        // Find byte offset for start of last n characters
        let skip_count = char_count - n;
        let byte_offset = full_string
            .char_indices()
            .nth(skip_count)
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        &full_string[byte_offset..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let mut buffer = Buffer::new(10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.as_string(), "");
    }

    #[test]
    fn test_push_characters() {
        let mut buffer = Buffer::new(5);
        buffer.push('h');
        buffer.push('e');
        buffer.push('l');
        buffer.push('l');
        buffer.push('o');

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_string(), "hello");
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = Buffer::new(3);
        buffer.push('a');
        buffer.push('b');
        buffer.push('c');
        buffer.push('d');
        buffer.push('e');

        // Should only keep last 3 characters
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.as_string(), "cde");
    }

    #[test]
    fn test_clear_buffer() {
        let mut buffer = Buffer::new(10);
        buffer.push('t');
        buffer.push('e');
        buffer.push('s');
        buffer.push('t');

        assert_eq!(buffer.len(), 4);

        buffer.clear();

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.as_string(), "");
    }

    #[test]
    fn test_last_n() {
        let mut buffer = Buffer::new(20); // Larger buffer to fit "hello world" (11 chars)
        "hello world".chars().for_each(|c| buffer.push(c));

        assert_eq!(buffer.last_n(5), "world");
        assert_eq!(buffer.last_n(11), "hello world");
        assert_eq!(buffer.last_n(20), "hello world"); // More than available
    }

    #[test]
    fn test_trigger_detection() {
        let mut buffer = Buffer::new(50);

        // Type some text including a trigger
        "This is ;name".chars().for_each(|c| buffer.push(c));

        let contents = buffer.as_string();
        assert!(contents.contains(";name"));
        assert_eq!(buffer.last_n(5), ";name");
    }

    #[test]
    fn test_cache_efficiency() {
        let mut buffer = Buffer::new(10);
        buffer.push('t');
        buffer.push('e');
        buffer.push('s');
        buffer.push('t');

        // First call should build the cache
        assert_eq!(buffer.as_string(), "test");

        // Second call should use the cache (no rebuild)
        // Verify the cached value is used correctly
        assert_eq!(buffer.as_string(), "test");

        // After a push, cache should be invalidated and rebuilt
        buffer.push('!');
        assert_eq!(buffer.as_string(), "test!");

        // Verify cache works after clear
        buffer.clear();
        assert_eq!(buffer.as_string(), "");
    }
}
