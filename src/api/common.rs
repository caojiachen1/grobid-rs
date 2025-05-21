//! Common API functions for grobid-rs
//!
//! This module contains common functions used across different processing modules.

use crate::errors::GrobidError;
use crate::format::tei::ParsedTei;
use std::path::Path;

/// Parse a TEI document string into a structured representation.
///
/// This function attempts to determine the type of TEI document (header, references, or full)
/// and parses it into the appropriate Rust structures.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use grobid_rs::ParsedTei;
///
/// let pdf_path = Path::new("path/to/document.pdf");
/// let tei = grobid_rs::process_header(pdf_path).unwrap();
/// let parsed = grobid_rs::parse_tei_str(&tei).unwrap();
///
/// match parsed {
///     ParsedTei::Header(metadata) => println!("Title: {:?}", metadata.title),
///     ParsedTei::References(refs) => println!("Found {} references", refs.len()),
///     ParsedTei::Full(doc) => println!("Full document with {} references", doc.references.len()),
/// }
/// ```
///
/// # Errors
///
/// Returns a `GrobidError` if:
/// - The TEI XML cannot be parsed
pub fn parse_tei_str(tei: &str) -> Result<ParsedTei, GrobidError> {
    crate::format::tei::parse_tei(tei)
}

/// Process a PDF with the specified processing function.
///
/// This is a utility function used internally by the various processing functions
/// to handle common error cases and standardize logging.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file to process
/// * `processor` - Function to process the PDF (e.g., process_header, process_references)
///
/// # Returns
///
/// Returns the result of the processing function or a GrobidError.
pub fn process_pdf<F, T>(pdf_path: &Path, processor: F) -> Result<T, GrobidError>
where
    F: FnOnce(&Path) -> Result<T, GrobidError>,
{
    // Validate the PDF file exists
    if !pdf_path.exists() {
        return Err(GrobidError::file_not_found(pdf_path));
    }

    // Call the processor function
    processor(pdf_path)
}

/// Determine if a path points to a PDF file based on extension.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// Returns true if the path has a .pdf extension (case insensitive).
#[allow(dead_code)]
pub fn is_pdf_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}
