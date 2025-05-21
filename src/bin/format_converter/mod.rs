// Module for converting between TEI and various output formats
mod json;
mod text;
mod bibtex;

/// Handles conversions between different output formats for Grobid results
pub struct FormatConverter;

impl FormatConverter {
    /// Convert TEI XML to JSON
    pub fn tei_to_json(tei: &str) -> anyhow::Result<String> {
        json::tei_to_json(tei)
    }
    
    /// Extract plain text from TEI XML
    pub fn tei_to_text(tei: &str) -> anyhow::Result<String> {
        text::tei_to_text(tei)
    }
    
    /// Convert TEI XML references to BibTeX
    pub fn tei_refs_to_bibtex(tei: &str) -> anyhow::Result<String> {
        bibtex::tei_to_bibtex(tei)
    }
}