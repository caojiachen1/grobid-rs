//! Header section parsing for TEI documents
//!
//! This module provides functionality for parsing the header/metadata section of TEI XML documents.
//! It extracts information such as title, authors, abstracts, and other metadata.

use crate::errors::GrobidError;
use crate::models::{Author, DocumentMetadata};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use tracing::{debug, trace};

use super::parser::SectionParser;
use super::utils::{get_attribute, normalize_whitespace, strip_namespace};

/// Parser for TEI document header/metadata
#[derive(Debug, Default)]
pub struct HeaderParser {
    /// Current field being parsed
    #[allow(dead_code)]
    current_field: String,
    /// Flag indicating we're inside a title element
    in_title: bool,
    /// Flag indicating we're inside an author element
    in_author: bool,
    /// Flag indicating we're inside an abstract element
    in_abstract: bool,
    /// Accumulated title text
    title: Option<String>,
    /// Accumulated abstract text
    abstract_text: Option<String>,
    /// List of authors
    authors: Vec<Author>,
    /// Current author being parsed
    current_author: Author,
    /// Document DOI
    doi: Option<String>,
}

impl HeaderParser {
    /// Creates a new header parser
    pub fn new() -> Self {
        Self {
            current_field: String::new(),
            in_title: false,
            in_author: false,
            in_abstract: false,
            title: None,
            abstract_text: None,
            authors: Vec::new(),
            current_author: Author::default(),
            doi: None,
        }
    }

    /// Parse a single title element
    fn parse_title(&mut self, text: &str) {
        if self.title.is_none() {
            self.title = Some(normalize_whitespace(text));
        }
    }

    /// Parse author information
    fn parse_author_info(
        &mut self,
        _element: &BytesStart,
        reader: &mut Reader<&[u8]>,
    ) -> Result<(), GrobidError> {
        let mut buffer = Vec::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    let _name = e.name();
                    let decoded_tag = reader.decoder().decode(_name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag for text extraction".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    match tag {
                        "persName" => {
                            // Inside a person name element
                        }
                        "forename" => {
                            // Parsing first name
                            let text = self.extract_text(reader, "forename")?;
                            if self.current_author.first_name.is_none() {
                                self.current_author.first_name = Some(normalize_whitespace(&text));
                            } else if self.current_author.middle_name.is_none() {
                                // If we already have a first name, this is probably a middle name
                                self.current_author.middle_name = Some(normalize_whitespace(&text));
                            }
                        }
                        "surname" => {
                            // Parsing last name
                            let text = self.extract_text(reader, "surname")?;
                            self.current_author.last_name = Some(normalize_whitespace(&text));
                        }
                        "email" => {
                            // Parsing email
                            let text = self.extract_text(reader, "email")?;
                            self.current_author.email = Some(normalize_whitespace(&text));
                        }
                        "affiliation" => {
                            // Parsing affiliation
                            let text = self.extract_text(reader, "affiliation")?;
                            self.current_author.affiliation = Some(normalize_whitespace(&text));
                        }
                        "idno" => {
                            // Parsing identifier (e.g., ORCID)
                            // Check for type attribute
                            if let Some(id_type) = get_attribute(e, "type") {
                                if id_type == "ORCID" {
                                    let text = self.extract_text(reader, "idno")?;
                                    self.current_author.identifier =
                                        Some(normalize_whitespace(&text));
                                }
                            }
                        }
                        _ => {
                            // Ignore other tags inside author
                        }
                    }
                }
                Ok(Event::End(ref _e)) => {
                    depth -= 1;
                    if depth < 0 {
                        // We've reached the end of the author element
                        break;
                    }
                }
                Ok(Event::Text(ref t)) => {
                    // If we get text directly under the author element, use it as full_name
                    if depth == 0 {
                        let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "unescaping text".into(),
                        })?;
                        if self.current_author.full_name.is_none() {
                            self.current_author.full_name = Some(normalize_whitespace(&text));
                        }
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing author".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing author".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        // Add the author if we have at least one name component
        if self.current_author.first_name.is_some()
            || self.current_author.last_name.is_some()
            || self.current_author.full_name.is_some()
        {
            self.authors.push(std::mem::take(&mut self.current_author));
        }

        Ok(())
    }

    /// Extract text content from an element
    fn extract_text(
        &mut self,
        reader: &mut Reader<&[u8]>,
        element_name: &str,
    ) -> Result<String, GrobidError> {
        let mut buffer = Vec::new();
        let mut content = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref _e)) => {
                    depth += 1;
                }
                Ok(Event::End(ref e)) => {
                    if depth == 0 {
                        let name = e.name();
                        let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                            GrobidError::XmlParseError {
                                message: e.to_string(),
                                context: format!("decoding end tag for {}", element_name),
                            }
                        })?;
                        let tag = strip_namespace(&decoded_tag);

                        if tag == element_name {
                            break;
                        }
                    }
                    depth -= 1;
                }
                Ok(Event::Text(ref t)) => {
                    let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: format!("unescaping text in {}", element_name),
                    })?;

                    if !content.is_empty() && !text.trim().is_empty() {
                        content.push(' ');
                    }
                    content.push_str(&text);
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: format!("extracting text from {}", element_name),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: format!("extracting text from {}", element_name),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        Ok(content)
    }
}

impl SectionParser<DocumentMetadata> for HeaderParser {
    fn is_target_tag(&self, element: &BytesStart) -> bool {
        let name = element.name();
        let tag_str = String::from_utf8_lossy(name.as_ref());
        let tag = strip_namespace(&tag_str);
        tag == "teiHeader" || tag == "fileDesc"
    }

    fn section_name(&self) -> &'static str {
        "header"
    }

    fn parse(
        &mut self,
        reader: &mut Reader<&[u8]>,
        _element: &BytesStart,
    ) -> Result<DocumentMetadata, GrobidError> {
        trace!("Starting to parse header section");

        let mut buffer = Vec::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding tag name in header".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    match tag {
                        "title" => {
                            self.in_title = true;
                            let text = self.extract_text(reader, "title")?;
                            self.parse_title(&text);
                            self.in_title = false;
                            depth -= 1; // We've already consumed the end element
                        }
                        "author" => {
                            self.in_author = true;
                            self.parse_author_info(e, reader)?;
                            self.in_author = false;
                            depth -= 1; // We've already consumed the end element
                        }
                        "abstract" => {
                            self.in_abstract = true;
                            let text = self.extract_text(reader, "abstract")?;
                            self.abstract_text = Some(normalize_whitespace(&text));
                            self.in_abstract = false;
                            depth -= 1; // We've already consumed the end element
                        }
                        "idno" => {
                            // Check if this is a DOI
                            if let Some(id_type) = get_attribute(e, "type") {
                                if id_type == "DOI" {
                                    let text = self.extract_text(reader, "idno")?;
                                    self.doi = Some(normalize_whitespace(&text));
                                    depth -= 1; // We've already consumed the end element
                                }
                            }
                        }
                        _ => {
                            // Continue parsing
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in header".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if (tag == "teiHeader" || tag == "fileDesc") && depth <= 0 {
                        // We've reached the end of the header section
                        break;
                    }
                }
                Ok(Event::Eof) => {
                    // End of file reached while parsing header
                    break;
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing header".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        debug!(
            "Finished parsing header, found title: {}, authors: {}, abstract: {}",
            self.title.is_some(),
            self.authors.len(),
            self.abstract_text.is_some()
        );

        // Construct the metadata object
        let metadata = DocumentMetadata {
            title: self.title.clone(),
            authors: self.authors.clone(),
            abstract_text: self.abstract_text.clone(),
            doi: self.doi.clone(),
            ..Default::default()
        };

        Ok(metadata)
    }
}

/// Convenience function to parse just the header section from a TEI document
pub fn parse_header(tei: &str) -> Result<DocumentMetadata, GrobidError> {
    let mut parser = HeaderParser::new();
    let tei = String::from_utf8_lossy(tei.as_bytes());
    let mut reader = Reader::from_str(&tei);
    reader.trim_text(true);
    reader.expand_empty_elements(true);

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref e)) => {
                if parser.is_target_tag(e) {
                    return parser.parse(&mut reader, e);
                }
            }
            Ok(Event::Eof) => {
                return Err(GrobidError::InvalidInput(
                    "No header section found in document".into(),
                ));
            }
            Err(e) => {
                return Err(GrobidError::XmlParseError {
                    message: e.to_string(),
                    context: "searching for header section".into(),
                });
            }
            _ => {}
        }
        buffer.clear();
    }
}
