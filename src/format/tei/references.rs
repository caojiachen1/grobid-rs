//! References section parsing for TEI documents
//!
//! This module provides functionality for parsing bibliographic references
//! from TEI XML documents.

use crate::errors::GrobidError;
use crate::models::{Date, Reference};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use tracing::{debug, trace};

use super::parser::SectionParser;
use super::utils::{get_attribute, normalize_whitespace, strip_namespace};

/// Parser for TEI document bibliographic references
#[derive(Debug, Default)]
pub struct ReferencesParser {
    /// Current reference being parsed
    current_ref: Reference,
    /// Flag indicating we're inside a biblStruct element
    in_biblstruct: bool,
    /// List of accumulated references
    references: Vec<Reference>,
}

impl ReferencesParser {
    /// Creates a new references parser
    pub fn new() -> Self {
        Self {
            current_ref: Reference::default(),
            in_biblstruct: false,
            references: Vec::new(),
        }
    }

    /// Parse a single bibliographic reference
    fn parse_single_reference(
        &mut self,
        element: &BytesStart,
        reader: &mut Reader<&[u8]>,
    ) -> Result<(), GrobidError> {
        // Reset current reference
        self.current_ref = Reference::default();

        // Get reference ID if available
        if let Some(id) = get_attribute(element, "xml:id").or_else(|| get_attribute(element, "id"))
        {
            self.current_ref.id = Some(id.to_string());
        }

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
                            context: "decoding tag name in reference".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    match tag {
                        "title" => {
                            // Check if this is an article title
                            if let Some(level) = get_attribute(e, "level") {
                                if level == "a" || level == "analytic" {
                                    let text = self.extract_text(reader, "title")?;
                                    self.current_ref.title = Some(normalize_whitespace(&text));
                                    depth -= 1; // We've already consumed the end element
                                } else if level == "j" || level == "m" {
                                    // Journal or monograph title
                                    let text = self.extract_text(reader, "title")?;
                                    self.current_ref.venue = Some(normalize_whitespace(&text));
                                    depth -= 1;
                                }
                            } else {
                                // No level attribute, assume it's the main title
                                let text = self.extract_text(reader, "title")?;
                                if self.current_ref.title.is_none() {
                                    self.current_ref.title = Some(normalize_whitespace(&text));
                                }
                                depth -= 1;
                            }
                        }
                        "author" => {
                            let text = self.extract_text(reader, "author")?;
                            self.current_ref.authors.push(normalize_whitespace(&text));
                            depth -= 1;
                        }
                        "date" => {
                            // Parse date information
                            let mut date = Date::default();

                            // Check for year attribute
                            if let Some(year) =
                                get_attribute(e, "when").or_else(|| get_attribute(e, "year"))
                            {
                                date.year = Some(year.to_string());
                            }

                            let text = self.extract_text(reader, "date")?;
                            if !text.is_empty() {
                                date.raw = Some(normalize_whitespace(&text));
                            }

                            self.current_ref.date = Some(date);
                            depth -= 1;
                        }
                        "biblScope" => {
                            // Check for unit attribute to determine what kind of scope this is
                            if let Some(unit) = get_attribute(e, "unit") {
                                let text = self.extract_text(reader, "biblScope")?;
                                match unit.as_ref() {
                                    "volume" => {
                                        self.current_ref.volume = Some(normalize_whitespace(&text));
                                    }
                                    "issue" => {
                                        self.current_ref.issue = Some(normalize_whitespace(&text));
                                    }
                                    "page" => {
                                        self.current_ref.pages = Some(normalize_whitespace(&text));
                                    }
                                    _ => {
                                        // Ignore other biblScope units
                                    }
                                }
                            }
                            depth -= 1;
                        }
                        "publisher" => {
                            let text = self.extract_text(reader, "publisher")?;
                            self.current_ref.publisher = Some(normalize_whitespace(&text));
                            depth -= 1;
                        }
                        "idno" => {
                            // Check if this is a DOI
                            if let Some(id_type) = get_attribute(e, "type") {
                                if id_type == "DOI" {
                                    let text = self.extract_text(reader, "idno")?;
                                    self.current_ref.doi = Some(normalize_whitespace(&text));
                                }
                            }
                            depth -= 1;
                        }
                        "ptr" => {
                            // Check for target attribute with DOI
                            if let Some(target) = get_attribute(e, "target") {
                                if target.starts_with("http://dx.doi.org/")
                                    || target.starts_with("https://doi.org/")
                                {
                                    let doi = target
                                        .replace("http://dx.doi.org/", "")
                                        .replace("https://doi.org/", "");
                                    self.current_ref.doi = Some(doi);
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
                            context: "decoding end tag in reference".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "biblStruct" && depth < 0 {
                        // Save the reference and break
                        self.references.push(std::mem::take(&mut self.current_ref));
                        break;
                    }
                }
                Ok(Event::Text(ref t)) => {
                    // Capturing raw text content at the biblStruct level for raw field
                    if depth == 0 {
                        let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "unescaping text in reference".into(),
                        })?;

                        if self.current_ref.raw.is_none() {
                            self.current_ref.raw = Some(text.to_string());
                        } else if let Some(raw) = &mut self.current_ref.raw {
                            raw.push(' ');
                            raw.push_str(&text);
                        }
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing reference".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing reference".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
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

impl SectionParser<Vec<Reference>> for ReferencesParser {
    fn is_target_tag(&self, element: &BytesStart) -> bool {
        let name = element.name();
        let tag_str = String::from_utf8_lossy(name.as_ref());
        let tag = strip_namespace(&tag_str);
        tag == "listBibl" || tag == "back"
    }

    fn section_name(&self) -> &'static str {
        "references"
    }

    fn parse(
        &mut self,
        reader: &mut Reader<&[u8]>,
        _element: &BytesStart,
    ) -> Result<Vec<Reference>, GrobidError> {
        trace!("Starting to parse references section");

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
                            context: "decoding tag name in references".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "biblStruct" {
                        self.in_biblstruct = true;
                        self.parse_single_reference(e, reader)?;
                        self.in_biblstruct = false;
                        depth -= 1; // We've already consumed the end element
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in references".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if (tag == "listBibl" || tag == "back") && depth < 0 {
                        // We've reached the end of the references section
                        break;
                    }
                }
                Ok(Event::Eof) => {
                    // End of file reached while parsing references
                    break;
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing references".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        debug!(
            "Finished parsing references, found {} references",
            self.references.len()
        );

        Ok(self.references.clone())
    }
}

/// Convenience function to parse just the references section from a TEI document
pub fn parse_references(tei: &str) -> Result<Vec<Reference>, GrobidError> {
    let mut parser = ReferencesParser::new();
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
                    "No references section found in document".into(),
                ));
            }
            Err(e) => {
                return Err(GrobidError::XmlParseError {
                    message: e.to_string(),
                    context: "searching for references section".into(),
                });
            }
            _ => {}
        }
        buffer.clear();
    }
}
