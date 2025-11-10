/// Finds the index of the first occurrence of a substring
/// Returns -1 if not found
pub fn index_of(haystack: &str, needle: &str) -> i64 {
    haystack.find(needle).map(|i| i as i64).unwrap_or(-1)
}

/// Splits a string by a delimiter
pub fn split(text: &str, delimiter: &str) -> Vec<String> {
    text.split(delimiter).map(|s| s.to_string()).collect()
}

/// Checks if a string contains a substring
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Replaces all occurrences of a search string with a replacement
pub fn replace(text: &str, search: &str, replacement: &str) -> String {
    text.replace(search, replacement)
}

/// Converts a string to lowercase
pub fn to_lower(text: &str) -> String {
    text.to_lowercase()
}

/// Converts a string to uppercase
pub fn to_upper(text: &str) -> String {
    text.to_uppercase()
}
