//! TEI (Text Encoding Initiative) parsing module
//!
//! This module provides functionality for parsing TEI XML documents into structured Rust types.
//! It includes parsers for various sections of TEI documents like headers, references, and body text.

// Export submodules
pub mod body;
pub mod header;
pub mod parser;
pub mod references;
pub mod utils;

// Re-export core types and functions
pub use self::body::{parse_body, BodyParser};
pub use self::header::{parse_header, HeaderParser};
pub use self::parser::{parse_tei, ParsedTei, SectionParser};
pub use self::references::{parse_references, ReferencesParser};
pub use self::utils::{concat_text, get_attribute, normalize_whitespace, strip_namespace};

// Convenience functions for JSON conversion
use crate::errors::GrobidError;
use serde::Serialize;

/// Convert any serializable value to JSON
pub fn to_json<T: Serialize>(value: &T, pretty: bool) -> Result<String, GrobidError> {
    if pretty {
        serde_json::to_string_pretty(value).map_err(GrobidError::from)
    } else {
        serde_json::to_string(value).map_err(GrobidError::from)
    }
}

/// Convert TEI XML to JSON format
pub fn tei_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let parsed = parse_tei(tei)?;
    to_json(&parsed, pretty)
}

/// Convert only header metadata to JSON
pub fn header_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let metadata = parse_header(tei)?;
    to_json(&metadata, pretty)
}

/// Convert just references to JSON
pub fn references_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let references = parse_references(tei)?;
    to_json(&references, pretty)
}

/// Convert just body/full text to JSON
pub fn body_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let full_text = parse_body(tei)?;
    to_json(&full_text, pretty)
}
