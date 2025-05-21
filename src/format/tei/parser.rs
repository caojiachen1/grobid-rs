//! Core TEI parsing functionality
//!
//! This module provides the fundamental structures and functions for parsing TEI XML documents.
//! It defines the `SectionParser` trait that all section parsers must implement and
//! the main `parse_tei` function that coordinates the overall parsing process.

use crate::errors::GrobidError;
use crate::models::{DocumentMetadata, GrobidDocument, Reference};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Serialize;
use tracing::{debug, trace, warn};

/// Represents different types of parsed TEI content
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ParsedTei {
    /// Just the header/metadata was parsed
    Header(DocumentMetadata),
    /// Just the bibliographic references were parsed
    References(Vec<Reference>),
    /// The full document was parsed
    Full(GrobidDocument),
}

/// Enum to track the current section being parsed
#[derive(Debug, PartialEq)]
pub(crate) enum Section {
    /// No section currently being parsed
    None,
    /// Currently parsing the header section
    Header,
    /// Currently parsing the references section
    References,
    /// Currently parsing the body text section
    Body,
}

/// Trait defining the interface for section parsers
///
/// This trait must be implemented by all section parsers to provide
/// a consistent interface for parsing different parts of a TEI document.
pub trait SectionParser<T> {
    /// Determines if the given XML element is the target tag for this parser
    fn is_target_tag(&self, element: &BytesStart) -> bool;

    /// Returns the name of the section this parser handles
    fn section_name(&self) -> &'static str;

    /// Parses the section from the XML reader
    ///
    /// # Arguments
    ///
    /// * `reader` - The XML reader positioned at the start of the section
    /// * `element` - The starting element of the section
    ///
    /// # Returns
    ///
    /// The parsed section data or an error
    fn parse(&mut self, reader: &mut Reader<&[u8]>, element: &BytesStart)
        -> Result<T, GrobidError>;
}

/// Parses a TEI XML string into structured data
///
/// This function coordinates the parsing of different sections of a TEI document,
/// using specialized section parsers for each part.
///
/// # Arguments
///
/// * `tei` - The TEI XML content as a string
///
/// # Returns
///
/// A `ParsedTei` enum representing the parsed content, or an error
pub fn parse_tei(tei: &str) -> Result<ParsedTei, GrobidError> {
    // Handle potentially invalid UTF-8 input safely
    let tei = String::from_utf8_lossy(tei.as_bytes());
    let mut reader = Reader::from_str(&tei);
    reader.trim_text(true);
    reader.expand_empty_elements(true); // Handle self-closing tags properly

    let mut buffer = Vec::new();
    let mut document = GrobidDocument::default();

    // Track which sections we've found
    let mut found_header = false;
    let mut found_refs = false;
    let mut found_body = false;

    // Import parsers from their respective modules
    let mut header_parser = crate::format::tei::header::HeaderParser::new();
    let mut refs_parser = crate::format::tei::references::ReferencesParser::new();
    let mut body_parser = crate::format::tei::body::BodyParser::new();

    // State machine for processing document
    let mut section = Section::None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref e)) => {
                if section == Section::None {
                    // Try to identify which section we're entering
                    // Create a span for this parsing operation
                    trace!("identifying_section");
                    if header_parser.is_target_tag(e) {
                        debug!("Found header section");
                        let _ = Section::Header; // Set section for context but avoid unused warning
                                                 // Parse header
                        match header_parser.parse(&mut reader, e) {
                            Ok(metadata) => {
                                document.metadata = metadata;
                                found_header = true;
                            }
                            Err(err) => {
                                warn!("Failed to parse header: {}", err);
                                // Continue parsing other sections
                            }
                        }

                        section = Section::None;
                    } else if refs_parser.is_target_tag(e) {
                        debug!("Found references section");
                        let _ = Section::References; // Set section for context but avoid unused warning
                                                     // Parse references
                        match refs_parser.parse(&mut reader, e) {
                            Ok(refs) => {
                                document.references = refs;
                                found_refs = true;
                            }
                            Err(err) => {
                                warn!("Failed to parse references: {}", err);
                                // Continue parsing other sections
                            }
                        }

                        section = Section::None;
                    } else if body_parser.is_target_tag(e) {
                        debug!("Found body section");
                        let _ = Section::Body; // Set section for context but avoid unused warning
                                               // Parse body
                        match body_parser.parse(&mut reader, e) {
                            Ok(full_text) => {
                                document.full_text = Some(full_text);
                                found_body = true;
                            }
                            Err(err) => {
                                warn!("Failed to parse body: {}", err);
                                // Continue parsing other sections
                            }
                        }

                        section = Section::None;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(GrobidError::XmlParseError {
                    message: e.to_string(),
                    context: "at document root level".to_string(),
                })
            }
            _ => {}
        }
        buffer.clear();
    }

    // No recognizable TEI content
    if !found_header && !found_refs && !found_body {
        return Err(GrobidError::InvalidInput(
            "No recognizable TEI sections found in document".into(),
        ));
    }

    // Determine document type based on what was found
    let fields_count = count_metadata_fields(&document.metadata);
    match (found_header, found_refs, found_body) {
        (true, false, false) => {
            debug!(
                "Detected header-only document with {} metadata fields",
                fields_count
            );
            Ok(ParsedTei::Header(document.metadata))
        }
        (false, true, false) => {
            debug!(
                "Detected references-only document with {} references",
                document.references.len()
            );
            Ok(ParsedTei::References(document.references))
        }
        _ => {
            debug!(
                "Detected full or partial document with {} metadata fields and {} references",
                fields_count,
                document.references.len()
            );
            Ok(ParsedTei::Full(document))
        }
    }
}

/// Count non-empty metadata fields to assess document completeness
fn count_metadata_fields(metadata: &DocumentMetadata) -> usize {
    let mut count = 0;

    if metadata.title.is_some() {
        count += 1;
    }
    if !metadata.authors.is_empty() {
        count += 1;
    }
    if metadata.abstract_text.is_some() {
        count += 1;
    }
    if metadata.date.is_some() {
        count += 1;
    }
    if metadata.doi.is_some() {
        count += 1;
    }
    if metadata.venue.is_some() {
        count += 1;
    }
    if !metadata.keywords.is_empty() {
        count += 1;
    }
    if !metadata.other.is_empty() {
        count += 1;
    }

    count
}

/// Parse state for tracking a sequence of element traversal
///
/// This structure helps track the path of XML elements during parsing,
/// which can be useful for contextual parsing logic.
#[derive(Debug, Default)]
pub struct ParseState {
    /// Current path of elements being traversed
    path: Vec<String>,
    /// Whether we're currently collecting text content
    collecting_text: bool,
    /// Accumulated text content
    text_content: String,
}

impl ParseState {
    /// Creates a new parse state
    pub fn new() -> Self {
        Self::default()
    }

    /// Enters a new element in the path
    pub fn enter_element(&mut self, name: &str) {
        self.path.push(name.to_string());
    }

    /// Exits the current element
    pub fn exit_element(&mut self) -> Option<String> {
        self.path.pop()
    }

    /// Gets the current path as a string
    pub fn current_path(&self) -> String {
        self.path.join("/")
    }

    /// Starts collecting text content
    pub fn start_collecting_text(&mut self) {
        self.collecting_text = true;
        self.text_content.clear();
    }

    /// Adds text content
    pub fn add_text(&mut self, text: &str) {
        if self.collecting_text {
            if !self.text_content.is_empty() && !text.trim().is_empty() {
                self.text_content.push(' ');
            }
            self.text_content.push_str(text);
        }
    }

    /// Stops collecting text and returns the collected content
    pub fn stop_collecting_text(&mut self) -> String {
        self.collecting_text = false;
        std::mem::take(&mut self.text_content)
    }
}
