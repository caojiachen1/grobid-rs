//! Body section parsing for TEI documents
//!
//! This module provides functionality for parsing the main text content of TEI XML documents.
//! It extracts structured sections, figures, tables, and equations.

use crate::errors::GrobidError;
use crate::models::{Equation, Figure, FullText, Section, Table};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use tracing::{debug, trace};

use super::parser::SectionParser;
use super::utils::{get_attribute, normalize_whitespace, strip_namespace};

/// Parser for TEI document body/full text
#[derive(Debug, Default)]
pub struct BodyParser {
    /// Accumulated sections
    sections: Vec<Section>,
    /// Accumulated figures
    figures: Vec<Figure>,
    /// Accumulated tables
    tables: Vec<Table>,
    /// Accumulated equations
    equations: Vec<Equation>,
    /// Current section being parsed
    #[allow(dead_code)]
    current_section: Option<Section>,
    /// Stack of section levels for tracking hierarchy
    section_stack: Vec<(u8, usize)>, // (level, index in sections vector)
}

impl BodyParser {
    /// Creates a new body parser
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            figures: Vec::new(),
            tables: Vec::new(),
            equations: Vec::new(),
            current_section: None,
            section_stack: Vec::new(),
        }
    }

    /// Parse a section element
    fn parse_section(
        &mut self,
        element: &BytesStart,
        reader: &mut Reader<&[u8]>,
        level: u8,
        section: &mut Section,
    ) -> Result<(), GrobidError> {
        // Update section level
        section.level = level;

        // Get section ID if available
        if let Some(_id) = get_attribute(element, "xml:id").or_else(|| get_attribute(element, "id"))
        {
            // Store ID in other field if needed
        }

        let mut buffer = Vec::new();
        let mut content = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding tag name in section".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    match tag {
                        "head" => {
                            // Parse section title
                            let text = self.extract_text(reader, "head")?;
                            section.title = Some(normalize_whitespace(&text));
                            depth -= 1; // We've already consumed the end element
                        }
                        "div" | "div1" | "div2" | "div3" | "div4" | "div5" => {
                            // Subsection - recursively parse it
                            let subsection_level = level + 1;
                            // Section index no longer needed with new approach

                            // Save current section before processing subsection
                            if !content.is_empty() {
                                let trimmed = content.trim();
                                if !trimmed.is_empty() {
                                    if !section.content.is_empty() {
                                        section.content.push(' ');
                                    }
                                    section.content.push_str(trimmed);
                                }
                                content.clear();
                            }

                            // Create a new subsection
                            let mut subsection = Section {
                                level: subsection_level,
                                ..Default::default()
                            };

                            // Parse subsection
                            self.parse_section(e, reader, subsection_level, &mut subsection)?;

                            // Add subsection to current section
                            section.subsections.push(subsection);

                            depth -= 1; // We've already consumed the end element
                        }
                        "figure" => {
                            // Parse figure
                            self.parse_figure(e, reader)?;
                            depth -= 1; // We've already consumed the end element
                        }
                        "table" => {
                            // Parse table
                            self.parse_table(e, reader)?;
                            depth -= 1; // We've already consumed the end element
                        }
                        "formula" => {
                            // Parse equation/formula
                            self.parse_equation(e, reader)?;
                            depth -= 1; // We've already consumed the end element
                        }
                        _ => {
                            // Other elements are simply traversed
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in section".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if (tag == "div"
                        || tag == "div1"
                        || tag == "div2"
                        || tag == "div3"
                        || tag == "div4"
                        || tag == "div5"
                        || tag == "body")
                        && depth < 0
                    {
                        // End of section
                        break;
                    }
                }
                Ok(Event::Text(ref t)) => {
                    let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "unescaping text in section".into(),
                    })?;

                    // Add to section content
                    if !text.trim().is_empty() {
                        if !content.is_empty() {
                            content.push(' ');
                        }
                        content.push_str(&text);
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing section".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing section".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        // Add remaining content
        if !content.is_empty() {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if !section.content.is_empty() {
                    section.content.push(' ');
                }
                section.content.push_str(trimmed);
            }
        }

        // If this is a top-level section, add it to the sections list
        if level == 1 && self.section_stack.is_empty() {
            self.sections.push(section.clone());
        }

        Ok(())
    }

    /// Parse a figure element
    fn parse_figure(
        &mut self,
        element: &BytesStart,
        reader: &mut Reader<&[u8]>,
    ) -> Result<(), GrobidError> {
        let mut figure = Figure::default();

        // Get figure ID if available
        if let Some(id) = get_attribute(element, "xml:id").or_else(|| get_attribute(element, "id"))
        {
            figure.id = Some(id.to_string());
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
                            context: "decoding tag name in figure".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    match tag {
                        "head" | "figDesc" => {
                            // Parse figure caption/description
                            let text = self.extract_text(reader, tag)?;
                            if tag == "head" {
                                figure.caption = Some(normalize_whitespace(&text));
                            } else {
                                figure.description = Some(normalize_whitespace(&text));
                            }
                            depth -= 1; // We've already consumed the end element
                        }
                        _ => {
                            // Other elements are simply traversed
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in figure".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "figure" && depth < 0 {
                        // End of figure
                        break;
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing figure".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing figure".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        // Add figure to the list if it has at least one field
        if figure.id.is_some() || figure.caption.is_some() || figure.description.is_some() {
            self.figures.push(figure);
        }

        Ok(())
    }

    /// Parse a table element
    fn parse_table(
        &mut self,
        element: &BytesStart,
        reader: &mut Reader<&[u8]>,
    ) -> Result<(), GrobidError> {
        let mut table = Table::default();

        // Get table ID if available
        if let Some(id) = get_attribute(element, "xml:id").or_else(|| get_attribute(element, "id"))
        {
            table.id = Some(id.to_string());
        }

        let mut buffer = Vec::new();
        let mut content = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding tag name in table".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "head" {
                        // Parse table caption
                        let text = self.extract_text(reader, "head")?;
                        table.caption = Some(normalize_whitespace(&text));
                        depth -= 1; // We've already consumed the end element
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in table".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "table" && depth < 0 {
                        // End of table
                        break;
                    }
                }
                Ok(Event::Text(ref t)) => {
                    let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "unescaping text in table".into(),
                    })?;

                    // Add to table content
                    if !text.trim().is_empty() {
                        if !content.is_empty() {
                            content.push(' ');
                        }
                        content.push_str(&text);
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing table".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing table".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        // Set table content
        if !content.is_empty() {
            table.content = Some(normalize_whitespace(&content));
        }

        // Add table to the list if it has at least one field
        if table.id.is_some() || table.caption.is_some() || table.content.is_some() {
            self.tables.push(table);
        }

        Ok(())
    }

    /// Parse an equation/formula element
    fn parse_equation(
        &mut self,
        element: &BytesStart,
        reader: &mut Reader<&[u8]>,
    ) -> Result<(), GrobidError> {
        let mut equation = Equation::default();

        // Get equation ID if available
        if let Some(id) = get_attribute(element, "xml:id").or_else(|| get_attribute(element, "id"))
        {
            equation.id = Some(id.to_string());
        }

        let mut buffer = Vec::new();
        let mut content = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding tag name in equation".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if tag == "desc" {
                        // Parse equation description
                        let text = self.extract_text(reader, "desc")?;
                        equation.description = Some(normalize_whitespace(&text));
                        depth -= 1; // We've already consumed the end element
                    }
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    let name = e.name();
                    let decoded_tag = reader.decoder().decode(name.as_ref()).map_err(|e| {
                        GrobidError::XmlParseError {
                            message: e.to_string(),
                            context: "decoding end tag in equation".into(),
                        }
                    })?;
                    let tag = strip_namespace(&decoded_tag);

                    if (tag == "formula" || tag == "equation") && depth < 0 {
                        // End of equation
                        break;
                    }
                }
                Ok(Event::Text(ref t)) => {
                    let text = t.unescape().map_err(|e| GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "unescaping text in equation".into(),
                    })?;

                    // Add to equation content
                    if !text.trim().is_empty() {
                        if !content.is_empty() {
                            content.push(' ');
                        }
                        content.push_str(&text);
                    }
                }
                Ok(Event::Eof) => {
                    return Err(GrobidError::XmlParseError {
                        message: "Unexpected end of file".into(),
                        context: "parsing equation".into(),
                    });
                }
                Err(e) => {
                    return Err(GrobidError::XmlParseError {
                        message: e.to_string(),
                        context: "parsing equation".into(),
                    });
                }
                _ => {}
            }
            buffer.clear();
        }

        // Set equation content
        if !content.is_empty() {
            equation.content = normalize_whitespace(&content);
        }

        // Add equation to the list if it has content
        if !equation.content.is_empty() || equation.description.is_some() {
            self.equations.push(equation);
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

impl SectionParser<FullText> for BodyParser {
    fn is_target_tag(&self, element: &BytesStart) -> bool {
        let name = element.name();
        let tag_str = String::from_utf8_lossy(name.as_ref());
        let tag = strip_namespace(&tag_str);
        tag == "body" || tag == "text"
    }

    fn section_name(&self) -> &'static str {
        "body"
    }

    fn parse(
        &mut self,
        reader: &mut Reader<&[u8]>,
        element: &BytesStart,
    ) -> Result<FullText, GrobidError> {
        trace!("Starting to parse body section");

        // Reset any previous state
        self.sections.clear();
        self.figures.clear();
        self.tables.clear();
        self.equations.clear();
        self.section_stack.clear();

        // Parse the body content
        let mut section = Section {
            level: 1,
            ..Default::default()
        };
        self.parse_section(element, reader, 1, &mut section)?;

        debug!(
            "Finished parsing body, found {} sections, {} figures, {} tables, {} equations",
            self.sections.len(),
            self.figures.len(),
            self.tables.len(),
            self.equations.len()
        );

        // Construct the full text object
        let full_text = FullText {
            sections: self.sections.clone(),
            figures: self.figures.clone(),
            tables: self.tables.clone(),
            equations: self.equations.clone(),
        };

        Ok(full_text)
    }
}

/// Convenience function to parse just the body section from a TEI document
pub fn parse_body(tei: &str) -> Result<FullText, GrobidError> {
    let mut parser = BodyParser::new();
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
                    "No body section found in document".into(),
                ));
            }
            Err(e) => {
                return Err(GrobidError::XmlParseError {
                    message: e.to_string(),
                    context: "searching for body section".into(),
                });
            }
            _ => {}
        }
        buffer.clear();
    }
}
