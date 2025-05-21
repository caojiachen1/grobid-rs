//! JSON format module for GROBID document serialization
//!
//! This module provides functionality for converting TEI XML output from GROBID
//! into JSON representations of the data. It uses the TEI parser module to extract
//! structured data from the TEI XML and then serializes that data to JSON.

use crate::errors::GrobidError;
use crate::format::tei::{parse_body, parse_header, parse_references, parse_tei};
use serde::Serialize;
// Re-export ParsedTei from tei module

/// Convert TEI XML to JSON format
///
/// Parses the entire TEI document and converts it to a JSON representation.
///
/// # Arguments
///
/// * `tei` - The TEI XML content as a string
/// * `pretty` - Whether to format the JSON output for readability (true) or compactness (false)
///
/// # Returns
///
/// A Result containing the JSON string or an error
pub fn tei_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let parsed = parse_tei(tei)?;
    to_json(&parsed, pretty)
}

/// Convert only header metadata to JSON
///
/// Extracts only the header/metadata section from a TEI document and converts it to JSON.
///
/// # Arguments
///
/// * `tei` - The TEI XML content as a string
/// * `pretty` - Whether to format the JSON output for readability (true) or compactness (false)
///
/// # Returns
///
/// A Result containing the JSON string or an error
pub fn header_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let metadata = parse_header(tei)?;
    to_json(&metadata, pretty)
}

/// Convert just references to JSON
///
/// Extracts only the bibliographic references from a TEI document and converts them to JSON.
///
/// # Arguments
///
/// * `tei` - The TEI XML content as a string
/// * `pretty` - Whether to format the JSON output for readability (true) or compactness (false)
///
/// # Returns
///
/// A Result containing the JSON string or an error
pub fn references_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let references = parse_references(tei)?;
    to_json(&references, pretty)
}

/// Convert just the body/full text to JSON
///
/// Extracts only the main textual content from a TEI document and converts it to JSON.
///
/// # Arguments
///
/// * `tei` - The TEI XML content as a string
/// * `pretty` - Whether to format the JSON output for readability (true) or compactness (false)
///
/// # Returns
///
/// A Result containing the JSON string or an error
pub fn body_to_json(tei: &str, pretty: bool) -> Result<String, GrobidError> {
    let full_text = parse_body(tei)?;
    to_json(&full_text, pretty)
}

/// Serialize any serializable value to JSON
///
/// # Arguments
///
/// * `value` - The value to serialize
/// * `pretty` - Whether to format the JSON output for readability (true) or compactness (false)
///
/// # Returns
///
/// A Result containing the JSON string or an error
pub fn to_json<T: Serialize>(value: &T, pretty: bool) -> Result<String, GrobidError> {
    if pretty {
        serde_json::to_string_pretty(value).map_err(GrobidError::from)
    } else {
        serde_json::to_string(value).map_err(GrobidError::from)
    }
}

/// Convenience function to get both pretty and compact JSON in one call
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// A tuple of (pretty_json, compact_json) strings, or an error
pub fn to_json_formats<T: Serialize>(value: &T) -> Result<(String, String), GrobidError> {
    let pretty = serde_json::to_string_pretty(value).map_err(GrobidError::from)?;
    let compact = serde_json::to_string(value).map_err(GrobidError::from)?;
    Ok((pretty, compact))
}

// Re-export the ParsedTei type for users of this module
pub use crate::format::tei::ParsedTei;
