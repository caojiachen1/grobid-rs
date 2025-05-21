use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::Reader;

/// Extract plain text from TEI XML
pub fn tei_to_text(tei: &str) -> Result<String> {
    let mut reader = Reader::from_str(tei);
    reader.trim_text(true);

    let mut result = String::new();
    let mut buf = Vec::new();
    let mut in_text_element = false;
    let mut section_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name == QName(b"title") {
                    in_text_element = true;
                    section_name = "TITLE".to_string();
                } else if name == QName(b"abstract") {
                    in_text_element = true;
                    section_name = "ABSTRACT".to_string();
                } else if name == QName(b"p") || name == QName(b"head") {
                    in_text_element = true;
                } else if name == QName(b"author") || name == QName(b"persName") {
                    in_text_element = true;
                    section_name = "AUTHOR".to_string();
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name == QName(b"title")
                    || name == QName(b"abstract")
                    || name == QName(b"p")
                    || name == QName(b"head")
                    || name == QName(b"author")
                    || name == QName(b"persName")
                {
                    in_text_element = false;
                    result.push('\n');
                    section_name.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    if let Ok(text) = e.unescape() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            // Add section name if available
                            if !section_name.is_empty() {
                                result.push_str(&format!("[{}] ", section_name));
                                section_name.clear();
                            }
                            result.push_str(trimmed);
                            result.push(' ');
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing XML: {}", e)),
            _ => (),
        }
        buf.clear();
    }

    Ok(result)
}
