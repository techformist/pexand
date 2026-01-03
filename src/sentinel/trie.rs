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
}

impl Trie {
    /// Create a new empty Trie
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    /// Insert a trigger into the Trie
    pub fn insert(&mut self, trigger: &str) {
        let mut current = &mut self.root;

        for ch in trigger.chars() {
            current = current.children.entry(ch).or_insert_with(TrieNode::new);
        }

        current.is_end_of_word = true;
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
    pub fn find_matching_trigger(&self, text: &str) -> Option<String> {
        let chars: Vec<char> = text.chars().collect();
        
        // Try each possible starting position, starting from the end
        for start_idx in 0..chars.len() {
            let mut current = &self.root;
            let mut matched = String::new();
            let mut last_match = None;

            for i in start_idx..chars.len() {
                let ch = chars[i];
                
                match current.children.get(&ch) {
                    Some(node) => {
                        current = node;
                        matched.push(ch);
                        
                        if current.is_end_of_word {
                            last_match = Some(matched.clone());
                        }
                    }
                    None => break,
                }
            }

            if last_match.is_some() {
                return last_match;
            }
        }

        None
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
        assert_eq!(trie.find_matching_trigger("hello ;name"), Some(";name".to_string()));
        
        // Match in the middle (should find it)
        assert_eq!(trie.find_matching_trigger(";email and more"), Some(";email".to_string()));
        
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
}
