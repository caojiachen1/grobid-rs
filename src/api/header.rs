//! Header processing API functions
//!
//! This module contains functions for extracting and processing metadata from the header
//! sections of PDF documents. These functions operate on the title page, abstract, and
//! other front-matter elements to extract structured bibliographic information.

use super::common::process_pdf;
use crate::engine::process_header;
use crate::errors::GrobidError;
use crate::format::{tei::ParsedTei, FormatConverter};
use crate::models::DocumentMetadata;
use std::path::Path;

/// Process a PDF file and extract header metadata as a JSON string.
///
/// This function extracts the header information from a PDF document
/// and returns it as a JSON-formatted string with pretty formatting.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let json = grobid_rs::process_header_json(&pdf_path).unwrap();
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
pub fn process_header_json(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_header(path)?;
        FormatConverter::header_to_json(&tei).map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Process a PDF file and extract header metadata as a JSON string with formatting options.
///
/// This function extracts the header information from a PDF document
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
/// let compact_json = grobid_rs::process_header_json_with_options(&pdf_path, false).unwrap();
/// // Get pretty-printed JSON
/// let pretty_json = grobid_rs::process_header_json_with_options(&pdf_path, true).unwrap();
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
/// - The JSON serialization fails
pub fn process_header_json_with_options(
    pdf_path: &Path,
    pretty: bool,
) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_header(path)?;
        FormatConverter::header_to_json_with_options(&tei, pretty)
            .map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Extract header metadata and return as a structured object
///
/// This function extracts the header information from a PDF document
/// and returns it as a structured `DocumentMetadata` object.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let header = grobid_rs::process_header_structured(&pdf_path).unwrap();
/// println!("Title: {:?}", header.title);
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
pub fn process_header_structured(pdf_path: &Path) -> Result<DocumentMetadata, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_header(path)?;
        match crate::format::tei::parse_tei(&tei)? {
            ParsedTei::Header(metadata) => Ok(metadata),
            _ => crate::format::tei::parse_header(&tei),
        }
    })
}

/// Extract raw TEI XML for the header section
///
/// This function processes a PDF document and returns the raw TEI XML
/// representation of the header section. This is useful for custom processing
/// or debugging.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let tei_xml = grobid_rs::api::header::extract_header_tei(&pdf_path).unwrap();
/// println!("{}", tei_xml);
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
#[allow(dead_code)]
pub fn extract_header_tei(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, process_header)
}
