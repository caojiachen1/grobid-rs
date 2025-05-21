//! Full-text processing API functions
//!
//! This module contains functions for extracting and processing the complete content
//! of PDF documents, including metadata, body text, and references. These functions
//! provide access to the full structured content of academic papers.

use super::common::process_pdf;
use crate::engine::fulltext_to_tei;
use crate::errors::GrobidError;
use crate::format::{tei::ParsedTei, FormatConverter};
use crate::models::GrobidDocument;
use std::path::Path;

/// Process a PDF file and extract its full content as a JSON string.
///
/// This function extracts the complete content from a PDF document
/// (metadata, body text, and references) and returns it as a JSON-formatted string
/// with pretty formatting.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let json = grobid_rs::fulltext_to_json(&pdf_path).unwrap();
/// println!("{}", json);
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
/// - The JSON serialization fails
pub fn fulltext_to_json(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = fulltext_to_tei(path)?;
        FormatConverter::tei_to_json(&tei).map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Process a PDF file and extract its full content as a JSON string with formatting options.
///
/// This function extracts the complete content from a PDF document
/// and returns it as a JSON-formatted string with control over formatting.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file to process
/// * `pretty` - If true, format the JSON with indentation for readability; if false, compact format
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// // Get compact JSON
/// let compact_json = grobid_rs::fulltext_to_json_with_options(&pdf_path, false).unwrap();
/// // Get pretty-printed JSON
/// let pretty_json = grobid_rs::fulltext_to_json_with_options(&pdf_path, true).unwrap();
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
/// - The JSON serialization fails
pub fn fulltext_to_json_with_options(pdf_path: &Path, pretty: bool) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = fulltext_to_tei(path)?;
        FormatConverter::tei_to_json_with_options(&tei, pretty)
            .map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Extract full text and return as a structured document
///
/// This function extracts the complete content from a PDF document
/// and returns it as a structured `GrobidDocument` object containing
/// metadata, body text, and references.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let document = grobid_rs::fulltext_to_structured(&pdf_path).unwrap();
/// println!("Title: {:?}", document.metadata.title);
/// println!("Found {} references", document.references.len());
/// if let Some(full_text) = &document.full_text {
///     println!("Document has {} sections", full_text.sections.len());
/// }
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
pub fn fulltext_to_structured(pdf_path: &Path) -> Result<GrobidDocument, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = fulltext_to_tei(path)?;
        match crate::format::tei::parse_tei(&tei)? {
            ParsedTei::Full(document) => Ok(document),
            ParsedTei::Header(metadata) => Ok(GrobidDocument {
                metadata,
                ..Default::default()
            }),
            ParsedTei::References(references) => Ok(GrobidDocument {
                references,
                ..Default::default()
            }),
        }
    })
}

/// Extract raw TEI XML for the full document
///
/// This function processes a PDF document and returns the raw TEI XML
/// representation of the entire document. This is useful for custom processing
/// or debugging.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let tei_xml = grobid_rs::api::fulltext::extract_fulltext_tei(&pdf_path).unwrap();
/// println!("{}", tei_xml);
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
#[allow(dead_code)]
pub fn extract_fulltext_tei(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, fulltext_to_tei)
}
