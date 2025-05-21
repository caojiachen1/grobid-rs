use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure for a complete Grobid document
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrobidDocument {
    /// Source of the document (always "grobid-rs")
    #[serde(default = "default_source")]
    pub source: String,
    /// Version of grobid-rs that processed the document
    #[serde(default = "default_version")]
    pub version: String,
    /// Document metadata (title, authors, abstract, etc.)
    #[serde(default)]
    pub metadata: DocumentMetadata,
    /// Full text content divided into sections
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text: Option<FullText>,
    /// Bibliographic references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
}

/// Returns the default source string
fn default_source() -> String {
    "grobid-rs".to_string()
}

/// Returns the default version string
fn default_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Structure representing document metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// Document title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Document authors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Author>,
    /// Document abstract
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    /// Publication date
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    /// Digital Object Identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Publication venue (journal, conference, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub venue: Option<Venue>,
    /// Document keywords
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Additional metadata fields
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub other: HashMap<String, String>,
}

/// Structure representing an author
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Author {
    /// First name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Middle name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    /// Last name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// Full name (if parsed from a single string)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// Author email
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Author affiliation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    /// ORCID or other identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// Structure representing a date
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Date {
    /// Year
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// Month
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
    /// Day
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    /// Raw date string
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

/// Structure representing a publication venue
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Venue {
    /// Journal or conference name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Volume information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    /// Publisher
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// Structure representing full text content
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullText {
    /// Document sections
    #[serde(default)]
    pub sections: Vec<Section>,
    /// Figures in the document
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub figures: Vec<Figure>,
    /// Tables in the document
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<Table>,
    /// Equations in the document
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equations: Vec<Equation>,
}

/// Structure representing a document section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Section {
    /// Section title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Section level (1 = top level)
    #[serde(default)]
    pub level: u8,
    /// Section text content
    #[serde(default)]
    pub content: String,
    /// Subsections
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subsections: Vec<Section>,
}

/// Structure representing a figure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Figure {
    /// Figure identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Figure caption
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Figure description or text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Structure representing a table
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Table {
    /// Table identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Table caption
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Table content (usually HTML or simplified representation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Structure representing an equation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Equation {
    /// Equation identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Equation content (usually MathML or LaTeX)
    #[serde(default)]
    pub content: String,
    /// Equation description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Structure representing a bibliographic reference
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Reference {
    /// Reference identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Reference title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Reference authors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// Publication date
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    /// Journal or venue
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub venue: Option<String>,
    /// Volume information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    /// Publisher
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// DOI or other identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Raw citation text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}