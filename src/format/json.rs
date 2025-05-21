use anyhow::Result;
use serde_json::json;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;

/// Convert TEI XML to JSON format
pub fn tei_to_json(tei: &str) -> Result<String> {
    // Create base JSON structure
    let mut json_obj = json!({
        "source": "grobid-rs",
        "version": env!("CARGO_PKG_VERSION"),
        "metadata": {}
    });
    
    // Extract basic metadata
    let metadata = json_obj["metadata"].as_object_mut().unwrap();
    
    // Simple extraction of title
    if let Some(title) = extract_element_text(tei, "title") {
        metadata.insert("title".to_string(), json!(title));
    }
    
    // Extract authors
    let authors = extract_authors(tei);
    if !authors.is_empty() {
        metadata.insert("authors".to_string(), json!(authors));
    }
    
    // Extract abstract
    if let Some(abstract_text) = extract_element_text(tei, "abstract") {
        metadata.insert("abstract".to_string(), json!(abstract_text));
    }
    
    // Pretty print the JSON output
    Ok(serde_json::to_string_pretty(&json_obj)?)
}

/// Extract text content from a specific element
fn extract_element_text(xml: &str, element_name: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_element = false;
    let mut result = String::new();
    
    let element_bytes = element_name.as_bytes();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == QName(element_bytes) => {
                in_element = true;
            },
            Ok(Event::End(ref e)) if e.name() == QName(element_bytes) => {
                in_element = false;
                if !result.is_empty() {
                    return Some(result);
                }
            },
            Ok(Event::Text(e)) if in_element => {
                if let Ok(text) = e.unescape() {
                    result.push_str(&text);
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extract author names from TEI
fn extract_authors(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_author = false;
    let mut current_author = String::new();
    let mut authors = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name() == QName(b"author") || e.name() == QName(b"persName") {
                    in_author = true;
                    current_author.clear();
                }
            },
            Ok(Event::End(ref e)) => {
                if e.name() == QName(b"author") || e.name() == QName(b"persName") {
                    in_author = false;
                    if !current_author.trim().is_empty() {
                        authors.push(current_author.trim().to_string());
                    }
                }
            },
            Ok(Event::Text(e)) if in_author => {
                if let Ok(text) = e.unescape() {
                    current_author.push_str(&text);
                    current_author.push(' ');
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    authors
}