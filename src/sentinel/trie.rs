use std::collections::HashMap;

/// A node in the Trie
#[derive(Debug)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end_of_word: bool,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end_of_word: false,
        }
    }
}

/// Radix Trie for efficient pattern matching of triggers
pub struct Trie {
    root: TrieNode,
    max_trigger_length: usize,
}

impl Trie {
    /// Create a new empty Trie
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            max_trigger_length: 0,
        }
    }

    /// Insert a trigger into the Trie
    pub fn insert(&mut self, trigger: &str) {
        let mut current = &mut self.root;
        let trigger_len = trigger.chars().count();

        for ch in trigger.chars() {
            current = current.children.entry(ch).or_insert_with(TrieNode::new);
        }

        current.is_end_of_word = true;
        self.max_trigger_length = self.max_trigger_length.max(trigger_len);
    }

    /// Search for an exact match of a trigger
    pub fn search(&self, trigger: &str) -> bool {
        let mut current = &self.root;

        for ch in trigger.chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return false,
            }
        }

        current.is_end_of_word
    }

    /// Check if there is any trigger with the given prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        let mut current = &self.root;

        for ch in prefix.chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return false,
            }
        }

        true
    }

    /// Find all triggers that match suffixes of the given text
    /// Returns the longest matching trigger found
    /// Optimized to limit matching attempts to max_trigger_length per position
    pub fn find_matching_trigger(&self, text: &str) -> Option<String> {
        if self.max_trigger_length == 0 {
            return None;
        }

        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();

        if text_len == 0 {
            return None;
        }

        let mut longest_match: Option<(usize, String)> = None;

        // Optimization: prioritize searching from the end where triggers are most likely
        // We still search the full text but limit how deep we go from each position
        // This reduces worst-case from O(n²) to O(n * m) where m = max_trigger_length

        for start_idx in 0..text_len {
            let mut current = &self.root;
            let mut match_len = 0;
            let mut last_match_len = 0;

            // Limit search from this position to max_trigger_length
            // This is the key optimization: we don't need to check beyond max trigger length
            let end_pos = (start_idx + self.max_trigger_length + 1).min(text_len);

            for i in start_idx..end_pos {
                let ch = chars[i];

                match current.children.get(&ch) {
                    Some(node) => {
                        current = node;
                        match_len += 1;

                        if current.is_end_of_word {
                            last_match_len = match_len;
                        }
                    }
                    None => break,
                }
            }

            // If we found a match, store it if it's the longest so far
            if last_match_len > 0 {
                // Build the matched string using slice to avoid repeated allocations
                let matched_str: String = chars[start_idx..start_idx + last_match_len]
                    .iter()
                    .collect();

                // Keep track of longest match
                match &longest_match {
                    None => longest_match = Some((last_match_len, matched_str)),
                    Some((prev_len, _)) => {
                        if last_match_len > *prev_len {
                            longest_match = Some((last_match_len, matched_str));
                        }
                    }
                }
            }
        }

        longest_match.map(|(_, s)| s)
    }

    /// Load multiple triggers into the Trie
    pub fn load_triggers(&mut self, triggers: &[String]) {
        for trigger in triggers {
            self.insert(trigger);
        }
    }

    /// Clear all triggers from the Trie
    pub fn clear(&mut self) {
        self.root = TrieNode::new();
        self.max_trigger_length = 0;
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut trie = Trie::new();
        trie.insert(";name");
        trie.insert(";email");
        trie.insert(";date");

        assert!(trie.search(";name"));
        assert!(trie.search(";email"));
        assert!(trie.search(";date"));
        assert!(!trie.search(";unknown"));
        assert!(!trie.search(";nam")); // Prefix but not complete
    }

    #[test]
    fn test_starts_with() {
        let mut trie = Trie::new();
        trie.insert(";name");
        trie.insert(";namespace");

        assert!(trie.starts_with(";"));
        assert!(trie.starts_with(";n"));
        assert!(trie.starts_with(";na"));
        assert!(trie.starts_with(";nam"));
        assert!(trie.starts_with(";name"));
        assert!(trie.starts_with(";names"));
        assert!(!trie.starts_with(";x"));
    }

    #[test]
    fn test_find_matching_trigger() {
        let mut trie = Trie::new();
        trie.insert(";name");
        trie.insert(";email");

        // Match at the end
        assert_eq!(
            trie.find_matching_trigger("hello ;name"),
            Some(";name".to_string())
        );

        // Match in the middle (should find it)
        assert_eq!(
            trie.find_matching_trigger(";email and more"),
            Some(";email".to_string())
        );

        // No match
        assert_eq!(trie.find_matching_trigger("hello world"), None);

        // Partial match doesn't count
        assert_eq!(trie.find_matching_trigger("hello ;nam"), None);
    }

    #[test]
    fn test_load_triggers() {
        let mut trie = Trie::new();
        let triggers = vec![
            ";name".to_string(),
            ";email".to_string(),
            ";phone".to_string(),
        ];

        trie.load_triggers(&triggers);

        assert!(trie.search(";name"));
        assert!(trie.search(";email"));
        assert!(trie.search(";phone"));
    }

    #[test]
    fn test_clear() {
        let mut trie = Trie::new();
        trie.insert(";name");
        trie.insert(";email");

        assert!(trie.search(";name"));

        trie.clear();

        assert!(!trie.search(";name"));
        assert!(!trie.search(";email"));
    }

    #[test]
    fn test_overlapping_triggers() {
        let mut trie = Trie::new();
        trie.insert(";n");
        trie.insert(";name");

        // Should find the longest match
        let result = trie.find_matching_trigger("test ;name here");
        assert_eq!(result, Some(";name".to_string()));
    }

    #[test]
    fn test_empty_trie() {
        let trie = Trie::new();
        assert!(!trie.search(";anything"));
        assert_eq!(trie.find_matching_trigger("test ;name"), None);
    }

    #[test]
    fn test_performance_with_long_buffer() {
        let mut trie = Trie::new();
        trie.insert(";test");

        // Create a long buffer (100 chars) with trigger near the end
        let long_text = "a".repeat(90) + " ;test";

        // Should still find the trigger efficiently
        // With optimization: ~100 * 5 = 500 operations (O(n * m))
        // Without optimization: ~100 * 100 = 10,000 operations (O(n²))
        assert_eq!(
            trie.find_matching_trigger(&long_text),
            Some(";test".to_string())
        );

        // Test with trigger at the beginning
        let long_text_start = ";test ".to_string() + &"b".repeat(90);
        assert_eq!(
            trie.find_matching_trigger(&long_text_start),
            Some(";test".to_string())
        );
    }

    #[test]
    fn test_max_trigger_length_tracking() {
        let mut trie = Trie::new();
        assert_eq!(trie.max_trigger_length, 0);

        trie.insert(";a");
        assert_eq!(trie.max_trigger_length, 2);

        trie.insert(";longer");
        assert_eq!(trie.max_trigger_length, 7);

        trie.insert(";x");
        assert_eq!(trie.max_trigger_length, 7); // Should stay at 7

        trie.clear();
        assert_eq!(trie.max_trigger_length, 0);
    }
}
