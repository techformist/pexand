use chrono::Local;
use clipboard_win::{formats, get_clipboard};
use std::collections::HashSet;

/// Parse and expand dynamic variables in text
pub struct VariableParser {
    recursion_depth: usize,
    max_depth: usize,
}

impl VariableParser {
    /// Create a new variable parser with default max depth
    pub fn new() -> Self {
        Self {
            recursion_depth: 0,
            max_depth: 5,
        }
    }

    /// Parse and expand all variables in the given text
    pub fn parse(&mut self, text: &str) -> Result<String, String> {
        if self.recursion_depth >= self.max_depth {
            return Err("Maximum recursion depth exceeded".to_string());
        }

        self.recursion_depth += 1;
        let result = self.parse_internal(text);
        self.recursion_depth -= 1;

        result
    }

    fn parse_internal(&mut self, text: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'

                // Find the closing '}}'
                let mut var_name = String::new();
                let mut found_close = false;

                while let Some(ch) = chars.next() {
                    if ch == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        found_close = true;
                        break;
                    }
                    var_name.push(ch);
                }

                if found_close {
                    match self.expand_variable(&var_name) {
                        Ok(expanded) => result.push_str(&expanded),
                        Err(_) => {
                            // On error, keep the original variable syntax
                            result.push_str(&format!("{{{{{}}}}}", var_name));
                        }
                    }
                } else {
                    // No closing '}}', keep original
                    result.push_str("{{");
                    result.push_str(&var_name);
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    fn expand_variable(&self, var_spec: &str) -> Result<String, String> {
        // Parse variable name and optional format
        if let Some(colon_pos) = var_spec.find(':') {
            let var_name = &var_spec[..colon_pos];
            let format = &var_spec[colon_pos + 1..];

            match var_name {
                "date" => self.expand_date(format),
                _ => Err(format!("Unknown variable: {}", var_name)),
            }
        } else {
            // No format specified
            match var_spec {
                "date" => self.expand_date("%Y-%m-%d"),
                "clipboard" => self.expand_clipboard(),
                "cursor" => Ok(String::new()), // Cursor placeholder - not implemented yet
                _ => Err(format!("Unknown variable: {}", var_spec)),
            }
        }
    }

    fn expand_date(&self, format: &str) -> Result<String, String> {
        let now = Local::now();

        // Use chrono's strftime for full format support
        let formatted = now.format(format).to_string();

        Ok(formatted)
    }

    fn expand_clipboard(&self) -> Result<String, String> {
        get_clipboard(formats::Unicode).map_err(|e| format!("Failed to get clipboard: {:?}", e))
    }
}

impl Default for VariableParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a snippet body contains its own trigger (recursion detection)
pub fn contains_recursion(trigger: &str, body: &str) -> bool {
    body.contains(trigger)
}

/// Check if expansion would cause recursion in a set of snippets
pub fn detect_recursion_chain(
    trigger: &str,
    body: &str,
    all_snippets: &[(String, String)],
) -> Option<Vec<String>> {
    let mut visited = HashSet::new();
    let mut chain = Vec::new();

    if detect_recursion_internal(trigger, body, all_snippets, &mut visited, &mut chain) {
        Some(chain)
    } else {
        None
    }
}

fn detect_recursion_internal(
    trigger: &str,
    body: &str,
    all_snippets: &[(String, String)],
    visited: &mut HashSet<String>,
    chain: &mut Vec<String>,
) -> bool {
    if visited.contains(trigger) {
        chain.push(trigger.to_string());
        return true;
    }

    visited.insert(trigger.to_string());
    chain.push(trigger.to_string());

    // Check if body contains any triggers
    for (other_trigger, other_body) in all_snippets {
        if body.contains(other_trigger) {
            if detect_recursion_internal(other_trigger, other_body, all_snippets, visited, chain) {
                return true;
            }
        }
    }

    chain.pop();
    visited.remove(trigger);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_variables() {
        let mut parser = VariableParser::new();
        let result = parser.parse("Hello World").unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_parse_date_default() {
        let mut parser = VariableParser::new();
        let result = parser.parse("Today is {{date}}").unwrap();
        assert!(result.starts_with("Today is "));
        assert!(result.len() > "Today is ".len());
    }

    #[test]
    fn test_parse_date_custom_format() {
        let mut parser = VariableParser::new();
        let result = parser.parse("Date: {{date:%B %d, %Y}}").unwrap();
        assert!(result.starts_with("Date: "));
        // Should have month name, day, comma and year
        assert!(result.contains(","));
    }

    #[test]
    fn test_parse_multiple_variables() {
        let mut parser = VariableParser::new();
        let result = parser
            .parse("Date: {{date:%Y-%m-%d}}, same: {{date:%Y-%m-%d}}")
            .unwrap();
        assert!(result.contains("Date: "));
        assert!(result.contains(", same: "));
    }

    #[test]
    fn test_parse_unknown_variable() {
        let mut parser = VariableParser::new();
        let result = parser.parse("Unknown: {{unknown}}").unwrap();
        // Should keep the original variable
        assert_eq!(result, "Unknown: {{unknown}}");
    }

    #[test]
    fn test_contains_recursion_simple() {
        assert!(contains_recursion(";name", "My name is ;name"));
        assert!(!contains_recursion(";name", "My name is John"));
    }

    #[test]
    fn test_detect_recursion_direct() {
        let snippets = vec![(";name".to_string(), "I am ;name".to_string())];

        let chain = detect_recursion_chain(";name", "I am ;name", &snippets);
        assert!(chain.is_some());
    }

    #[test]
    fn test_detect_recursion_indirect() {
        let snippets = vec![
            (";a".to_string(), "This is ;b".to_string()),
            (";b".to_string(), "That is ;a".to_string()),
        ];

        let chain = detect_recursion_chain(";a", "This is ;b", &snippets);
        assert!(chain.is_some());
    }

    #[test]
    fn test_no_recursion() {
        let snippets = vec![
            (";name".to_string(), "John".to_string()),
            (";email".to_string(), "john@example.com".to_string()),
        ];

        let chain = detect_recursion_chain(";name", "John", &snippets);
        assert!(chain.is_none());
    }
}
