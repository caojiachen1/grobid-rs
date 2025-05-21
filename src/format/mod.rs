use std::error::Error;

/// Handles conversions between different output formats for Grobid results
pub struct FormatConverter;

impl FormatConverter {
    /// Convert TEI XML to JSON
    pub fn tei_to_json(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(tei::tei_to_json(tei, true)?)
    }

    /// Convert TEI XML to JSON with compact or pretty formatting
    pub fn tei_to_json_with_options(tei: &str, pretty: bool) -> Result<String, Box<dyn Error>> {
        let parsed = tei::parse_tei(tei)?;
        Ok(tei::to_json(&parsed, pretty)?)
    }

    /// Extract plain text from TEI XML
    pub fn tei_to_text(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(text::tei_to_text(tei)?)
    }

    /// Convert TEI XML references to BibTeX
    pub fn tei_refs_to_bibtex(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(bibtex::tei_to_bibtex(tei)?)
    }

    /// Convert just header metadata from TEI XML to JSON
    pub fn header_to_json(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(tei::header_to_json(tei, true)?)
    }

    /// Convert just header metadata from TEI XML to JSON with compact or pretty formatting
    pub fn header_to_json_with_options(tei: &str, pretty: bool) -> Result<String, Box<dyn Error>> {
        let metadata = tei::parse_header(tei)?;
        Ok(tei::to_json(&metadata, pretty)?)
    }

    /// Convert just references from TEI XML to JSON
    pub fn references_to_json(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(tei::references_to_json(tei, true)?)
    }

    /// Convert just references from TEI XML to JSON with compact or pretty formatting
    pub fn references_to_json_with_options(
        tei: &str,
        pretty: bool,
    ) -> Result<String, Box<dyn Error>> {
        let references = tei::parse_references(tei)?;
        Ok(tei::to_json(&references, pretty)?)
    }

    /// Convert just body/full text from TEI XML to JSON
    pub fn body_to_json(tei: &str) -> Result<String, Box<dyn Error>> {
        Ok(tei::body_to_json(tei, true)?)
    }

    /// Convert just body/full text from TEI XML to JSON with compact or pretty formatting
    pub fn body_to_json_with_options(tei: &str, pretty: bool) -> Result<String, Box<dyn Error>> {
        let body = tei::parse_body(tei)?;
        Ok(tei::to_json(&body, pretty)?)
    }
}

pub mod bibtex;
pub mod json;
pub mod tei;
pub mod text;
