use clipboard_win::{formats, get_clipboard};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

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
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?;
        let timestamp = duration.as_secs();

        // Simple date formatting - supports common patterns
        let formatted = match format {
            "%Y-%m-%d" => self.format_date_ymd(timestamp),
            "%Y/%m/%d" => self.format_date_ymd(timestamp).replace("-", "/"),
            "%d-%m-%Y" => self.format_date_dmy(timestamp),
            "%d/%m/%Y" => self.format_date_dmy(timestamp).replace("-", "/"),
            "%H:%M:%S" => self.format_time_hms(timestamp),
            "%H:%M" => self.format_time_hm(timestamp),
            "%Y-%m-%d %H:%M:%S" => format!("{} {}", self.format_date_ymd(timestamp), self.format_time_hms(timestamp)),
            _ => return Err(format!("Unsupported date format: {}. Supported: %Y-%m-%d, %Y/%m/%d, %d-%m-%Y, %d/%m/%Y, %H:%M:%S, %H:%M, %Y-%m-%d %H:%M:%S", format)),
        };

        Ok(formatted)
    }

    fn format_date_ymd(&self, timestamp: u64) -> String {
        let days_since_epoch = timestamp / 86400;
        let (year, month, day) = days_to_date(days_since_epoch as i64);
        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    fn format_date_dmy(&self, timestamp: u64) -> String {
        let days_since_epoch = timestamp / 86400;
        let (year, month, day) = days_to_date(days_since_epoch as i64);
        format!("{:02}-{:02}-{:04}", day, month, year)
    }

    fn format_time_hms(&self, timestamp: u64) -> String {
        let seconds_today = timestamp % 86400;
        let hours = seconds_today / 3600;
        let minutes = (seconds_today % 3600) / 60;
        let seconds = seconds_today % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    fn format_time_hm(&self, timestamp: u64) -> String {
        let seconds_today = timestamp % 86400;
        let hours = seconds_today / 3600;
        let minutes = (seconds_today % 3600) / 60;
        format!("{:02}:{:02}", hours, minutes)
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

// Helper function to convert days since Unix epoch to (year, month, day)
fn days_to_date(days: i64) -> (i32, u32, u32) {
    // Days since Unix epoch (1970-01-01)
    let mut year = 1970;
    let mut remaining_days = days;

    // Calculate year
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Calculate month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_month in &days_in_months {
        if remaining_days < days_in_month as i64 {
            break;
        }
        remaining_days -= days_in_month as i64;
        month += 1;
    }

    let day = remaining_days + 1;
    (year, month, day as u32)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
        let result = parser.parse("Year: {{date:%Y}}").unwrap();
        assert!(result.starts_with("Year: "));
        assert_eq!(result.len(), "Year: ".len() + 4); // Year is 4 digits
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
