# JSON Output Implementation Plan

## Overview

This document outlines the implementation plan for adding comprehensive JSON output support to grobid-rs. The goal is to create a modular system that provides type-safe access to Grobid data through well-defined Serde structs, with clear separation between the library API and CLI functionality.

## Goals

1. Define comprehensive Serde structs for Grobid outputs (header, citations, fulltext)
2. Implement TEI to struct conversion utilities
3. Provide library API functions for JSON output
4. Integrate with CLI in a modular way
5. Ensure compatibility with HTTP service API
6. Add appropriate tests and documentation

## Implementation Phases

### Phase 1: Core Data Models

**Tasks:**

1. Create `src/models/` directory with these files:
   - `mod.rs` - Module organization
   - `header.rs` - Header metadata structs
   - `references.rs` - Bibliography entry structs
   - `fulltext.rs` - Full document structs

2. Define header metadata structs:
   ```rust
   // src/models/header.rs
   use serde::{Deserialize, Serialize};
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct HeaderMetadata {
       pub title: Option<String>,
       pub authors: Vec<Author>,
       pub abstract_text: Option<String>,
       pub journal: Option<JournalInfo>,
       pub doi: Option<String>,
       pub publication_date: Option<PublicationDate>,
       pub keywords: Vec<String>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Author {
       pub first_name: Option<String>,
       pub middle_name: Option<String>,
       pub last_name: Option<String>,
       pub email: Option<String>,
       pub affiliations: Vec<Affiliation>,
       pub orcid: Option<String>,
   }
   
   // Additional structs...
   ```

3. Define bibliography structures:
   ```rust
   // src/models/references.rs
   use serde::{Deserialize, Serialize};
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BibEntry {
       pub id: Option<String>,
       pub title: Option<String>,
       pub authors: Vec<String>,
       pub journal: Option<String>,
       pub volume: Option<String>,
       pub issue: Option<String>,
       pub year: Option<u16>,
       pub pages: Option<PageRange>,
       pub doi: Option<String>,
       pub raw_text: Option<String>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PageRange {
       pub start: Option<String>,
       pub end: Option<String>,
   }
   ```

4. Define fulltext structures:
   ```rust
   // src/models/fulltext.rs
   use serde::{Deserialize, Serialize};
   use super::header::HeaderMetadata;
   use super::references::BibEntry;
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Document {
       pub header: HeaderMetadata,
       pub body_text: Vec<TextSection>,
       pub references: Vec<BibEntry>,
       pub figures: Vec<Figure>,
       pub tables: Vec<Table>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TextSection {
       pub heading: Option<String>,
       pub paragraphs: Vec<Paragraph>,
       pub level: u8,
   }
   
   // Additional structs...
   ```

### Phase 2: Converters

**Tasks:**

1. Create `src/converters/` directory with these files:
   - `mod.rs` - Module organization
   - `tei.rs` - TEI to struct conversions
   - `json.rs` - Struct to JSON utilities

2. Implement TEI parser for header:
   ```rust
   // src/converters/tei.rs
   use crate::models::header::HeaderMetadata;
   use crate::GrobidError;
   use quick_xml::events::Event;
   use quick_xml::reader::Reader;
   
   pub fn parse_tei_header(tei: &str) -> Result<HeaderMetadata, GrobidError> {
       let mut reader = Reader::from_str(tei);
       reader.trim_text(true);
       
       // Extract fields from TEI XML
       // ...
       
       // Return populated HeaderMetadata
   }
   ```

3. Implement TEI parser for references:
   ```rust
   // src/converters/tei.rs
   use crate::models::references::BibEntry;
   
   pub fn parse_tei_references(tei: &str) -> Result<Vec<BibEntry>, GrobidError> {
       // Parse <listBibl> section from TEI
       // ...
   }
   ```

4. Implement JSON utilities:
   ```rust
   // src/converters/json.rs
   use crate::models::header::HeaderMetadata;
   use crate::models::references::BibEntry;
   use crate::GrobidError;
   
   pub fn to_json<T: serde::Serialize>(value: &T) -> Result<String, GrobidError> {
       serde_json::to_string_pretty(value)
           .map_err(|e| GrobidError::Serialization(e.to_string()))
   }
   
   pub fn to_json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, GrobidError> {
       serde_json::to_value(value)
           .map_err(|e| GrobidError::Serialization(e.to_string()))
   }
   ```

### Phase 3: Library API

**Tasks:**

1. Update error types:
   ```rust
   // src/errors.rs
   #[derive(Error, Debug)]
   pub enum GrobidError {
       // Existing variants...
       
       #[error("Serialization error: {0}")]
       Serialization(String),
       
       #[error("Parsing error: {0}")]
       Parsing(String),
   }
   ```

2. Add JSON output functions to `lib.rs`:
   ```rust
   /// Extract header metadata and return as a JSON string
   pub fn process_header_json(pdf_path: &Path) -> Result<String, GrobidError> {
       let tei = process_header(pdf_path)?;
       let header = converters::tei::parse_tei_header(&tei)?;
       converters::json::to_json(&header)
   }
   
   /// Extract header metadata and return as a structured object
   pub fn process_header_structured(pdf_path: &Path) 
       -> Result<models::header::HeaderMetadata, GrobidError> 
   {
       let tei = process_header(pdf_path)?;
       converters::tei::parse_tei_header(&tei)
   }
   
   /// Extract references and return as JSON string
   pub fn process_references_json(pdf_path: &Path) -> Result<String, GrobidError> {
       let tei = process_references(pdf_path)?;
       let refs = converters::tei::parse_tei_references(&tei)?;
       converters::json::to_json(&refs)
   }
   
   /// Extract references and return as structured objects
   pub fn process_references_structured(pdf_path: &Path) 
       -> Result<Vec<models::references::BibEntry>, GrobidError> 
   {
       let tei = process_references(pdf_path)?;
       converters::tei::parse_tei_references(&tei)
   }
   
   /// Process full text and return as JSON string
   pub fn fulltext_to_json(pdf_path: &Path) -> Result<String, GrobidError> {
       let tei = fulltext_to_tei(pdf_path)?;
       let doc = converters::tei::parse_tei_document(&tei)?;
       converters::json::to_json(&doc)
   }
   
   /// Process full text and return as structured document
   pub fn fulltext_to_structured(pdf_path: &Path) 
       -> Result<models::fulltext::Document, GrobidError> 
   {
       let tei = fulltext_to_tei(pdf_path)?;
       converters::tei::parse_tei_document(&tei)
   }
   ```

### Phase 4: CLI and HTTP Integration

**Tasks:**

1. Update the `src/bin/format_converter/json.rs` module to use the library:
   ```rust
   use anyhow::Result;
   use grobid_rs::converters::tei::{parse_tei_header, parse_tei_references, parse_tei_document};
   use grobid_rs::converters::json::to_json;
   
   /// Convert TEI XML to JSON format
   pub fn tei_to_json(tei: &str) -> Result<String> {
       // Determine what type of TEI we're dealing with
       if tei.contains("<teiHeader>") && !tei.contains("<text>") {
           // Just a header
           let header = parse_tei_header(tei)?;
           Ok(to_json(&header)?)
       } else if tei.contains("<listBibl>") {
           // References
           let refs = parse_tei_references(tei)?;
           Ok(to_json(&refs)?)
       } else {
           // Full document
           let doc = parse_tei_document(tei)?;
           Ok(to_json(&doc)?)
       }
   }
   ```

2. Update CLI command handlers in `src/bin/cli.rs` to use new functions when appropriate:
   ```rust
   match &args.command {
       Commands::Header {
           pdf_file,
           output_format,
       } => {
           println!("Processing header from PDF: {}", pdf_file.display());
           
           let output = match output_format {
               OutputFormat::Tei => grobid_rs::process_header(pdf_file)?,
               OutputFormat::Json => grobid_rs::process_header_json(pdf_file)?,
               OutputFormat::Text => FormatConverter::tei_to_text(&grobid_rs::process_header(pdf_file)?)?,
               // ...
           };
           
           write_output(&output, &args.output_file)?;
       },
       // ...
   }
   ```

3. Ensure HTTP service compatibility:
   ```rust
   // In an HTTP route handler (e.g., axum or actix-web)
   async fn process_header_document(
       multipart: Multipart,
       accept: Option<HeaderValue>,
   ) -> impl IntoResponse {
       // Extract PDF from multipart form
       let pdf_bytes = extract_pdf_from_multipart(multipart).await?;
       let temp_file = save_to_temp_file(pdf_bytes).await?;
       
       // Determine output format based on Accept header
       let output = if accept_json(accept) {
           grobid_rs::process_header_json(&temp_file)?
       } else {
           grobid_rs::process_header(&temp_file)?
       };
       
       // Set appropriate content type and return
       let content_type = if accept_json(accept) {
           "application/json"
       } else {
           "application/tei+xml"
       };
       
       (
           StatusCode::OK,
           [(header::CONTENT_TYPE, content_type)],
           output
       )
   }
   ```

### Phase 5: Testing and Documentation

**Tasks:**

1. Create unit tests for model serialization:
   ```rust
   // src/models/header.rs (at the bottom)
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_header_serialization() {
           let header = HeaderMetadata {
               title: Some("Test Paper".to_string()),
               authors: vec![
                   Author {
                       first_name: Some("Jane".to_string()),
                       last_name: Some("Doe".to_string()),
                       middle_name: None,
                       email: None,
                       affiliations: vec![],
                       orcid: None,
                   }
               ],
               abstract_text: Some("This is a test abstract.".to_string()),
               journal: None,
               doi: None,
               publication_date: None,
               keywords: vec![],
           };
           
           let json = serde_json::to_string_pretty(&header).unwrap();
           assert!(json.contains("Test Paper"));
           assert!(json.contains("Jane"));
           assert!(json.contains("Doe"));
       }
   }
   ```

2. Create integration tests for TEI parsing:
   ```rust
   // tests/json_conversion_test.rs
   use grobid_rs::converters::tei::parse_tei_header;
   use grobid_rs::models::header::HeaderMetadata;
   
   #[test]
   fn test_parse_header_tei() {
       let sample_tei = r#"
       <TEI xmlns="http://www.tei-c.org/ns/1.0">
         <teiHeader>
           <titleStmt>
             <title>Sample Title</title>
           </titleStmt>
           <sourceDesc>
             <biblStruct>
               <analytic>
                 <author>
                   <persName>
                     <forename type="first">John</forename>
                     <surname>Smith</surname>
                   </persName>
                 </author>
               </analytic>
             </biblStruct>
           </sourceDesc>
         </teiHeader>
       </TEI>
       "#;
       
       let result = parse_tei_header(sample_tei).unwrap();
       assert_eq!(result.title.as_deref(), Some("Sample Title"));
       assert_eq!(result.authors.len(), 1);
       assert_eq!(result.authors[0].first_name.as_deref(), Some("John"));
       assert_eq!(result.authors[0].last_name.as_deref(), Some("Smith"));
   }
   ```

3. Update documentation in `lib.rs`:
   ```rust
   /// Process a PDF file and extract header metadata as a JSON string.
   ///
   /// This function extracts the header information from a PDF document
   /// and returns it as a JSON-formatted string.
   ///
   /// # Examples
   ///
   /// ```no_run
   /// use std::path::Path;
   /// use grobid_rs;
   ///
   /// let pdf_path = Path::new("path/to/document.pdf");
   /// let json = grobid_rs::process_header_json(&pdf_path).unwrap();
   /// println!("JSON output: {}", json);
   /// ```
   ///
   /// # Errors
   ///
   /// Returns a `GrobidError` if:
   /// - The PDF file cannot be processed
   /// - The TEI output cannot be parsed
   /// - The JSON serialization fails
   pub fn process_header_json(pdf_path: &Path) -> Result<String, GrobidError> {
       // ...
   }
   ```

## Feature Requirements

Ensure these modules are properly feature-gated in `Cargo.toml`:

```toml
[features]
default = []
cli = ["clap", "quick-xml", "serde", "serde_json"]
json = ["serde", "serde_json"]  # New feature

[dependencies]
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }
```

## Success Criteria

The implementation will be considered successful when:

1. **Library API**: `process_header_json`, `process_references_json`, and other JSON functions are available in the library API
2. **Type Safety**: All outputs are properly typed with comprehensive Serde structs
3. **CLI Integration**: CLI commands use the library's JSON functions
4. **HTTP Compatibility**: JSON output can be returned from HTTP endpoints matching Grobid's servlet API
5. **Test Coverage**: Proper unit and integration tests cover the functionality
6. **Documentation**: Well-documented API with examples

## Future Enhancements

1. Add schema validation for JSON outputs
2. Support for custom serialization options (pretty-print, minified)
3. Add more specialized structs for specific document types (patents, etc.)
4. Implement incremental parsing for very large documents
5. Add content negotiation for HTTP endpoints (Accept: application/json vs application/tei+xml)
6. Support streaming JSON responses for large documents in HTTP API
7. Add OpenAPI/Swagger documentation for the HTTP service