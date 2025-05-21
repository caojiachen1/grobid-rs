//! References processing API functions
//!
//! This module contains functions for extracting and processing bibliographic references
//! from PDF documents. These functions handle the extraction of citations and bibliographic
//! entries from the reference section of academic papers.

use super::common::process_pdf;
use crate::engine::process_references;
use crate::errors::GrobidError;
use crate::format::{tei::ParsedTei, FormatConverter};
use crate::models::Reference;
use std::path::Path;

/// Process a PDF file and extract bibliographic references as a JSON string.
///
/// This function extracts the references/bibliography section from a PDF document
/// and returns it as a JSON-formatted string with pretty formatting.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let json = grobid_rs::process_references_json(&pdf_path).unwrap();
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
pub fn process_references_json(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_references(path)?;
        FormatConverter::references_to_json(&tei)
            .map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Process a PDF file and extract references as a JSON string with formatting options.
///
/// This function extracts the references from a PDF document
/// and returns them as a JSON-formatted string with control over formatting.
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
/// let compact_json = grobid_rs::process_references_json_with_options(&pdf_path, false).unwrap();
/// // Get pretty-printed JSON
/// let pretty_json = grobid_rs::process_references_json_with_options(&pdf_path, true).unwrap();
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
/// - The JSON serialization fails
pub fn process_references_json_with_options(
    pdf_path: &Path,
    pretty: bool,
) -> Result<String, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_references(path)?;
        FormatConverter::references_to_json_with_options(&tei, pretty)
            .map_err(|e| GrobidError::Conversion(e.to_string()))
    })
}

/// Extract references and return as structured objects
///
/// This function extracts the bibliographic references from a PDF document
/// and returns them as a vector of structured `Reference` objects.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let references = grobid_rs::process_references_structured(&pdf_path).unwrap();
/// println!("Found {} references", references.len());
/// for (i, reference) in references.iter().enumerate() {
///     println!("Ref {}: {:?}", i + 1, reference.title);
/// }
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
/// - The TEI output cannot be parsed
pub fn process_references_structured(pdf_path: &Path) -> Result<Vec<Reference>, GrobidError> {
    process_pdf(pdf_path, |path| {
        let tei = process_references(path)?;
        match crate::format::tei::parse_tei(&tei)? {
            ParsedTei::References(references) => Ok(references),
            _ => crate::format::tei::parse_references(&tei),
        }
    })
}

/// Extract raw TEI XML for the references section
///
/// This function processes a PDF document and returns the raw TEI XML
/// representation of the references section. This is useful for custom processing
/// or debugging.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let tei_xml = grobid_rs::api::references::extract_references_tei(&pdf_path).unwrap();
/// println!("{}", tei_xml);
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The PDF file does not exist
/// - The PDF file cannot be processed
#[allow(dead_code)]
pub fn extract_references_tei(pdf_path: &Path) -> Result<String, GrobidError> {
    process_pdf(pdf_path, process_references)
}
