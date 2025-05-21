use std::error::Error;

/// Handles conversions between different output formats for Grobid results
pub struct FormatConverter;

impl FormatConverter {
    /// Convert TEI XML to JSON
    pub fn tei_to_json(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(json::tei_to_json(tei)?)
    }

    /// Extract plain text from TEI XML
    pub fn tei_to_text(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(text::tei_to_text(tei)?)
    }

    /// Convert TEI XML references to BibTeX
    pub fn tei_refs_to_bibtex(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(bibtex::tei_to_bibtex(tei)?)
    }
}

pub mod bibtex;
pub mod json;
pub mod text;
