use anyhow::Result;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;

/// Convert TEI XML references to BibTeX format
pub fn tei_to_bibtex(tei: &str) -> Result<String> {
    let mut reader = Reader::from_str(tei);
    reader.trim_text(true);
    
    let mut result = String::new();
    let mut buf = Vec::new();
    let mut in_bibl = false;
    let mut ref_count = 0;
    
    // Reference fields
    let mut title = String::new();
    let mut authors = Vec::new();
    let mut year = String::new();
    let mut journal = String::new();
    let mut volume = String::new();
    let mut pages = String::new();
    let mut doi = String::new();
    
    let mut current_element = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name == QName(b"biblStruct") {
                    in_bibl = true;
                    
                    // Reset reference fields
                    title.clear();
                    authors.clear();
                    year.clear();
                    journal.clear();
                    volume.clear();
                    pages.clear();
                    doi.clear();
                } else if in_bibl {
                    current_element = String::from_utf8_lossy(name.into_inner()).into_owned();
                }
            },
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name == QName(b"biblStruct") {
                    in_bibl = false;
                    ref_count += 1;
                    
                    // Generate a BibTeX entry
                    let key = format!("ref{}", ref_count);
                    result.push_str(&format!("@article{{{},\n", key));
                    
                    if !title.is_empty() {
                        result.push_str(&format!("  title = {{{}}},\n", escape_bibtex(&title)));
                    } else {
                        result.push_str("  title = {Unknown Title},\n");
                    }
                    
                    if !authors.is_empty() {
                        result.push_str(&format!("  author = {{{}}},\n", 
                            authors.join(" and ")
                        ));
                    }
                    
                    if !year.is_empty() {
                        result.push_str(&format!("  year = {{{}}},\n", year));
                    }
                    
                    if !journal.is_empty() {
                        result.push_str(&format!("  journal = {{{}}},\n", escape_bibtex(&journal)));
                    }
                    
                    if !volume.is_empty() {
                        result.push_str(&format!("  volume = {{{}}},\n", volume));
                    }
                    
                    if !pages.is_empty() {
                        result.push_str(&format!("  pages = {{{}}},\n", pages));
                    }
                    
                    if !doi.is_empty() {
                        result.push_str(&format!("  doi = {{{}}},\n", doi));
                    }
                    
                    // Close the entry
                    result.push_str("}\n\n");
                } else if in_bibl {
                    current_element.clear();
                }
            },
            Ok(Event::Text(e)) => {
                if in_bibl {
                    if let Ok(text) = e.unescape() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            match current_element.as_str() {
                                "title" if title.is_empty() => title = trimmed.to_string(),
                                "persName" | "author" => authors.push(trimmed.to_string()),
                                "date" if year.is_empty() => year = trimmed.to_string(),
                                "journal" if journal.is_empty() => journal = trimmed.to_string(),
                                "publisher" if journal.is_empty() => journal = trimmed.to_string(),
                                "volume" if volume.is_empty() => volume = trimmed.to_string(),
                                "biblScope" if pages.is_empty() => pages = trimmed.to_string(),
                                "idno" if doi.is_empty() => doi = trimmed.to_string(),
                                _ => {}
                            }
                        }
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing XML: {}", e)),
            _ => (),
        }
        buf.clear();
    }
    
    if result.is_empty() {
        result = "% No references found in the document\n".to_string();
    }
    
    Ok(result)
}

/// Escape special characters in BibTeX strings
fn escape_bibtex(s: &str) -> String {
    s.replace("\\", "\\\\")
     .replace("{", "\\{")
     .replace("}", "\\}")
     .replace("_", "\\_")
     .replace("&", "\\&")
     .replace("#", "\\#")
     .replace("%", "\\%")
     .replace("$", "\\$")
     .replace("^", "\\^")
     .replace("~", "\\~")
}