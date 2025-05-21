//! Utility functions for TEI XML parsing
//!
//! This module provides helper functions for handling XML namespaces,
//! attribute extraction, and other common operations needed during TEI parsing.

use quick_xml::events::BytesStart;

/// Strips namespace prefix from an XML tag name
///
/// # Examples
///
/// ```
/// use crate::format::tei::utils::strip_namespace;
///
/// let tag_with_ns = "tei:title";
/// assert_eq!(strip_namespace(tag_with_ns), "title");
///
/// let tag_without_ns = "abstract";
/// assert_eq!(strip_namespace(tag_without_ns), "abstract");
/// ```
pub fn strip_namespace(tag: &str) -> &str {
    match tag.find(':') {
        Some(pos) => &tag[pos + 1..],
        None => tag,
    }
}

/// Extracts an attribute value from an XML element
///
/// # Arguments
///
/// * `element` - The XML element to extract the attribute from
/// * `attr_name` - The name of the attribute to extract
///
/// # Returns
///
/// An Option containing the attribute value if found, None otherwise
pub fn get_attribute(element: &BytesStart, attr_name: &str) -> Option<String> {
    element
        .attributes()
        .filter_map(Result::ok)
        .find(|attr| attr.key.as_ref() == attr_name.as_bytes())
        .map(|attr| attr.value)
        .map(|value| String::from_utf8_lossy(&value).to_string())
}

/// Combines multiple text fragments, trimming whitespace
///
/// This is useful when collecting text content that may be split
/// across multiple XML text events.
pub fn concat_text(fragments: &[String]) -> String {
    fragments.join(" ").trim().to_string()
}

/// Safely parses a string as a number, returning None if parsing fails
pub fn parse_number<T: std::str::FromStr>(value: &str) -> Option<T> {
    value.trim().parse::<T>().ok()
}

/// Normalizes whitespace in a string by replacing multiple spaces with a single space
pub fn normalize_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_whitespace = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_whitespace {
                result.push(' ');
                last_was_whitespace = true;
            }
        } else {
            result.push(c);
            last_was_whitespace = false;
        }
    }

    result.trim().to_string()
}
