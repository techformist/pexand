use std::collections::VecDeque;

/// Rolling buffer that maintains the last N characters typed
pub struct Buffer {
    data: VecDeque<char>,
    max_size: usize,
}

impl Buffer {
    /// Create a new buffer with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push a character onto the buffer
    /// If the buffer is full, the oldest character is removed
    pub fn push(&mut self, ch: char) {
        if self.data.len() >= self.max_size {
            self.data.pop_front();
        }
        self.data.push_back(ch);
    }

    /// Get the buffer contents as a String
    pub fn as_string(&self) -> String {
        self.data.iter().collect()
    }

    /// Clear all contents from the buffer
    pub fn clear(&mut self) {
        self.data.clear();
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
    pub fn last_n(&self, n: usize) -> String {
        let start = if self.data.len() > n {
            self.data.len() - n
        } else {
            0
        };
        self.data.iter().skip(start).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = Buffer::new(10);
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
}
