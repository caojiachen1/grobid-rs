use anyhow::Result;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use crate::models::{Reference, Date};

/// Extract bibliographic references from TEI XML
pub fn extract_references(tei: &str) -> Result<Vec<Reference>> {
    let mut references = Vec::new();
    
    let mut reader = Reader::from_str(tei);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_bibl = false;
    let mut current_ref_xml = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().into_inner() {
                    b"biblStruct" => {
                        in_bibl = true;
                        current_ref_xml.clear();
                            
                        // We'll capture the XML and process it when we hit the end tag
                        // No need to process attributes here as they'll be in the XML string
                    },
                    _ => {}
                }
                
                if in_bibl {
                    // Capture the entire XML for this reference to extract details later
                    if let Ok(tag) = String::from_utf8(e.name().into_inner().to_vec()) {
                        current_ref_xml.push_str("<");
                        current_ref_xml.push_str(&tag);
                        current_ref_xml.push_str(">");
                    }
                }
            },
            Ok(Event::End(ref e)) => {
                match e.name().into_inner() {
                    b"biblStruct" => {
                        in_bibl = false;
                        
                        // Process the collected XML for this reference
                        if !current_ref_xml.is_empty() {
                            if let Some(reference) = process_reference_xml(&current_ref_xml)? {
                                references.push(reference);
                            }
                        }
                    },
                    _ => {}
                }
                
                if in_bibl {
                    // Capture the entire XML for this reference
                    if let Ok(tag) = String::from_utf8(e.name().into_inner().to_vec()) {
                        current_ref_xml.push_str("</");
                        current_ref_xml.push_str(&tag);
                        current_ref_xml.push_str(">");
                    }
                }
            },
            Ok(Event::Text(e)) => {
                if in_bibl {
                    // Capture text for the current reference
                    if let Ok(text) = e.unescape() {
                        current_ref_xml.push_str(&text);
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    Ok(references)
}

/// Process a single reference XML string and convert it to a Reference struct
fn process_reference_xml(xml: &str) -> Result<Option<Reference>> {
    let mut reference = Reference::default();
    
    // Extract ID (look for xml:id or id attribute in biblStruct tag)
    if let Some(id_match) = xml.find("xml:id=\"") {
        let start = id_match + 8; // Length of 'xml:id="'
        if let Some(end) = xml[start..].find('"') {
            reference.id = Some(xml[start..start+end].to_string());
        }
    } else if let Some(id_match) = xml.find("id=\"") {
        let start = id_match + 4; // Length of 'id="'
        if let Some(end) = xml[start..].find('"') {
            reference.id = Some(xml[start..start+end].to_string());
        }
    }
    
    // Extract title
    reference.title = extract_element_text_from_str(xml, "title");
    
    // Extract authors
    extract_authors_from_str(xml, &mut reference.authors);
    
    // Extract date
    reference.date = extract_date_from_str(xml)?;
    
    // Extract venue details
    extract_venue_details_from_str(xml, &mut reference)?;
    
    // Extract DOI
    reference.doi = extract_doi_from_str(xml);
    
    // Store the raw reference text for debugging
    reference.raw = Some(xml.to_string());
    
    Ok(Some(reference))
}

/// Extract element text from a string
fn extract_element_text_from_str(xml: &str, element_name: &str) -> Option<String> {
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

/// Extract authors from a string and add to authors vector
fn extract_authors_from_str(xml: &str, authors: &mut Vec<String>) {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_author = false;
    let mut in_persname = false;
    let mut current_author = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().into_inner() {
                    b"author" => {
                        in_author = true;
                        current_author.clear();
                    },
                    b"persName" => {
                        in_persname = true;
                        current_author.clear();
                    },
                    _ => {}
                }
            },
            Ok(Event::End(ref e)) => {
                match e.name().into_inner() {
                    b"author" => {
                        in_author = false;
                        if !current_author.trim().is_empty() {
                            authors.push(current_author.trim().to_string());
                        }
                    },
                    b"persName" => {
                        in_persname = false;
                        if !current_author.trim().is_empty() {
                            authors.push(current_author.trim().to_string());
                        }
                    },
                    _ => {}
                }
            },
            Ok(Event::Text(e)) => {
                if in_author || in_persname {
                    if let Ok(text) = e.unescape() {
                        if !text.trim().is_empty() {
                            current_author.push_str(&text);
                            current_author.push(' ');
                        }
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
}

/// Extract date from a string
fn extract_date_from_str(xml: &str) -> Result<Option<Date>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut date = Date::default();
    let mut has_date_info = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == QName(b"date") => {
                // Check for year, month, day attributes
                for attr in e.attributes() {
                    if let Ok(attr) = attr {
                        match attr.key.into_inner() {
                            b"when" => {
                                let when = String::from_utf8_lossy(&attr.value).to_string();
                                date.raw = Some(when.clone());
                                
                                // Try to parse YYYY-MM-DD format
                                let parts: Vec<&str> = when.split('-').collect();
                                if parts.len() >= 1 {
                                    date.year = Some(parts[0].to_string());
                                    has_date_info = true;
                                }
                                if parts.len() >= 2 {
                                    date.month = Some(parts[1].to_string());
                                }
                                if parts.len() >= 3 {
                                    date.day = Some(parts[2].to_string());
                                }
                            },
                            b"year" => {
                                date.year = Some(String::from_utf8_lossy(&attr.value).to_string());
                                has_date_info = true;
                            },
                            b"month" => {
                                date.month = Some(String::from_utf8_lossy(&attr.value).to_string());
                                has_date_info = true;
                            },
                            b"day" => {
                                date.day = Some(String::from_utf8_lossy(&attr.value).to_string());
                                has_date_info = true;
                            },
                            _ => {}
                        }
                    }
                }
                
                // Also extract any text content (e.g., "2022")
                let text = get_text_until_end(&mut reader, b"date")?;
                if !text.trim().is_empty() {
                    if date.raw.is_none() {
                        date.raw = Some(text.trim().to_string());
                    }
                    
                    // If it's just a year, store it
                    if text.trim().len() == 4 && text.trim().chars().all(|c| c.is_digit(10)) {
                        date.year = Some(text.trim().to_string());
                        has_date_info = true;
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    if has_date_info {
        Ok(Some(date))
    } else {
        Ok(None)
    }
}

/// Extract venue details from a string and update reference
fn extract_venue_details_from_str(xml: &str, reference: &mut Reference) -> Result<()> {
    // Extract journal or conference name
    if let Some(journal) = extract_element_text_from_str(xml, "journal") {
        reference.venue = Some(journal);
    } else if let Some(conf) = extract_element_text_from_str(xml, "conference") {
        reference.venue = Some(conf);
    } else if let Some(booktitle) = extract_element_text_from_str(xml, "booktitle") {
        reference.venue = Some(booktitle);
    }
    
    // Extract volume, issue, pages
    if let Some(volume) = extract_element_text_from_str(xml, "volume") {
        reference.volume = Some(volume);
    }
    
    if let Some(issue) = extract_element_text_from_str(xml, "issue") {
        reference.issue = Some(issue);
    }
    
    if let Some(pages) = extract_element_text_from_str(xml, "pages") {
        reference.pages = Some(pages);
    }
    
    // Extract publisher
    if let Some(publisher) = extract_element_text_from_str(xml, "publisher") {
        reference.publisher = Some(publisher);
    }
    
    Ok(())
}

/// Extract DOI from a string
fn extract_doi_from_str(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut in_doi = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == QName(b"idno") => {
                // Look for type="DOI" attribute
                for attr in e.attributes() {
                    if let Ok(attr) = attr {
                        if attr.key == QName(b"type") && &*attr.value == b"DOI" {
                            in_doi = true;
                            break;
                        }
                    }
                }
            },
            Ok(Event::End(ref e)) if e.name() == QName(b"idno") => {
                in_doi = false;
            },
            Ok(Event::Text(e)) if in_doi => {
                if let Ok(text) = e.unescape() {
                    let doi = text.trim().to_string();
                    if !doi.is_empty() {
                        return Some(doi);
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    None
}

/// Helper function to get text content until an end tag
fn get_text_until_end(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<String> {
    let mut buf = Vec::new();
    let mut content = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    content.push_str(&text);
                }
            },
            Ok(Event::End(ref e)) if e.name() == QName(end_tag) => {
                break;
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }
    
    Ok(content.trim().to_string())
}