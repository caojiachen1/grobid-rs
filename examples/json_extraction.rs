//! JSON and XML/TEI Extraction Example
//!
//! This example demonstrates how to use grobid-rs to extract structured information
//! from PDF documents in both JSON and XML/TEI formats.

use std::env;
use std::path::Path;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <grobid_home_path> <pdf_file_path>", args[0]);
        process::exit(1);
    }

    let grobid_home = Path::new(&args[1]);
    let pdf_path = Path::new(&args[2]);

    // Verify paths
    if !grobid_home.exists() || !grobid_home.is_dir() {
        eprintln!(
            "Error: Grobid home directory not found at {}",
            grobid_home.display()
        );
        process::exit(1);
    }

    if !pdf_path.exists() || !pdf_path.is_file() {
        eprintln!("Error: PDF file not found at {}", pdf_path.display());
        process::exit(1);
    }

    // Initialize Grobid with default configuration
    println!("Initializing Grobid from: {}", grobid_home.display());
    let config = grobid_rs::GrobidConfig::new(grobid_home);
    grobid_rs::init(&config)?;

    // Extract header information using the TEI API
    println!("\n--- Processing Header with TEI API ---");
    match grobid_rs::process_header(pdf_path) {
        Ok(header_xml) => {
            println!("Successfully extracted header information as TEI XML:");
            println!("TEI XML output (truncated):");
            println!("{}", truncate_text(&header_xml, 200));

            // Note: In a real application, you would typically use an XML parser
            // like quick-xml or roxmltree to extract specific elements from the TEI

            println!("\nTo extract specific data like title, authors, etc., you would");
            println!("need to parse the TEI XML. In this example, we're just showing the raw XML.");
        }
        Err(e) => {
            eprintln!("Error processing header: {}", e);
        }
    }

    // Extract header information as JSON
    println!("\n--- Processing Header with JSON API ---");
    match grobid_rs::process_header_json(pdf_path) {
        Ok(header_json) => {
            println!("Successfully extracted header information as JSON:");
            println!("JSON output (truncated):");
            println!("{}", truncate_text(&header_json, 200));

            println!("\nJSON output is ready to use with any JSON parser like serde_json.");

            // Also demonstrate compact JSON option
            println!("\n--- Demonstrating Compact JSON Format ---");
            match grobid_rs::process_header_json_with_options(pdf_path, false) {
                Ok(compact_json) => {
                    println!("Header as compact JSON (truncated):");
                    println!("{}", truncate_text(&compact_json, 100));

                    // Compare sizes
                    println!("\nJSON size comparison:");
                    println!("  Pretty:  {} bytes", header_json.len());
                    println!("  Compact: {} bytes", compact_json.len());
                    println!(
                        "  Savings: {:.1}%",
                        100.0 * (1.0 - (compact_json.len() as f64 / header_json.len() as f64))
                    );
                }
                Err(e) => {
                    eprintln!("Error processing compact JSON: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error processing header as JSON: {}", e);
        }
    }

    // Extract header information as a structured Rust object
    println!("\n--- Processing Header with Structured API ---");
    match grobid_rs::process_header_structured(pdf_path) {
        Ok(header_struct) => {
            println!("Successfully extracted header as a Rust struct:");
            println!("Title: {:?}", header_struct.title);
            println!("Authors: {} author(s)", header_struct.authors.len());
            if let Some(abstract_text) = &header_struct.abstract_text {
                println!("Abstract: {}", truncate_text(abstract_text, 100));
            }
            if let Some(doi) = &header_struct.doi {
                println!("DOI: {}", doi);
            }

            println!("\nThe structured API gives you typed access to the document metadata.");
        }
        Err(e) => {
            eprintln!("Error processing header as structured data: {}", e);
        }
    }

    // Extract references using the TEI API
    println!("\n--- Processing References with TEI API ---");
    match grobid_rs::process_references(pdf_path) {
        Ok(references_xml) => {
            println!("Successfully extracted references as TEI XML:");
            println!("TEI XML output (truncated):");
            println!("{}", truncate_text(&references_xml, 200));

            // Note: In a real application, you would typically use an XML parser
            // to extract the reference data from the TEI document

            println!("\nTo extract individual references and their metadata,");
            println!("you would need to parse the TEI XML structure.");
        }
        Err(e) => {
            eprintln!("Error processing references: {}", e);
        }
    }

    // Extract references as JSON
    println!("\n--- Processing References with JSON API ---");
    match grobid_rs::process_references_json(pdf_path) {
        Ok(references_json) => {
            println!("Successfully extracted references as JSON:");
            println!("JSON output (truncated):");
            println!("{}", truncate_text(&references_json, 200));

            println!("\nJSON references can be easily parsed or stored in a database.");

            // Also demonstrate compact JSON option
            println!("\n--- Demonstrating Compact JSON References ---");
            match grobid_rs::process_references_json_with_options(pdf_path, false) {
                Ok(compact_json) => {
                    println!("References as compact JSON (truncated):");
                    println!("{}", truncate_text(&compact_json, 100));

                    // Compare sizes
                    println!("\nReferences JSON size comparison:");
                    println!("  Pretty:  {} bytes", references_json.len());
                    println!("  Compact: {} bytes", compact_json.len());
                    println!(
                        "  Savings: {:.1}%",
                        100.0 * (1.0 - (compact_json.len() as f64 / references_json.len() as f64))
                    );
                }
                Err(e) => {
                    eprintln!("Error processing compact JSON: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error processing references as JSON: {}", e);
        }
    }

    // Extract references as structured Rust objects
    println!("\n--- Processing References with Structured API ---");
    match grobid_rs::process_references_structured(pdf_path) {
        Ok(references) => {
            println!(
                "Successfully extracted {} references as Rust structs:",
                references.len()
            );

            // Display the first few references
            for (i, reference) in references.iter().take(3).enumerate() {
                println!("\nReference #{}:", i + 1);
                if let Some(title) = &reference.title {
                    println!("  Title: {}", truncate_text(title, 50));
                }
                println!("  Authors: {}", reference.authors.join(", "));
                if let Some(doi) = &reference.doi {
                    println!("  DOI: {}", doi);
                }
            }

            if references.len() > 3 {
                println!("\n... and {} more references", references.len() - 3);
            }

            println!("\nThe structured API gives you typed access to bibliographic data.");
        }
        Err(e) => {
            eprintln!("Error processing references as structured data: {}", e);
        }
    }

    // Extract full text using the TEI API
    println!("\n--- Processing Full Text with TEI API ---");
    match grobid_rs::fulltext_to_tei(pdf_path) {
        Ok(document_xml) => {
            println!("Successfully extracted full document as TEI XML:");
            println!("TEI XML output (truncated):");
            println!("{}", truncate_text(&document_xml, 200));

            // Note: In a real application, you would typically use an XML parser
            // to extract the document structure, sections, figures, tables, etc.

            println!("\nTo extract structured data from the document,");
            println!("you would need to parse the TEI XML.");
        }
        Err(e) => {
            eprintln!("Error processing full text: {}", e);
        }
    }

    // Extract full text as JSON
    println!("\n--- Processing Full Text with JSON API ---");
    match grobid_rs::fulltext_to_json(pdf_path) {
        Ok(document_json) => {
            println!("Successfully extracted full document as JSON:");
            println!("JSON output (truncated):");
            println!("{}", truncate_text(&document_json, 200));

            println!("\nJSON document output contains metadata, body text, and references.");

            // Also demonstrate compact JSON option
            println!("\n--- Demonstrating Compact JSON Document ---");
            match grobid_rs::fulltext_to_json_with_options(pdf_path, false) {
                Ok(compact_json) => {
                    println!("Full document as compact JSON (truncated):");
                    println!("{}", truncate_text(&compact_json, 100));

                    // Compare sizes
                    println!("\nFull document JSON size comparison:");
                    println!("  Pretty:  {} bytes", document_json.len());
                    println!("  Compact: {} bytes", compact_json.len());
                    println!(
                        "  Savings: {:.1}%",
                        100.0 * (1.0 - (compact_json.len() as f64 / document_json.len() as f64))
                    );

                    println!("\nCompact JSON is beneficial when:");
                    println!("- Storing large numbers of documents");
                    println!("- Transmitting over network connections");
                    println!("- Working with bandwidth-constrained environments");
                }
                Err(e) => {
                    eprintln!("Error processing compact JSON: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error processing full text as JSON: {}", e);
        }
    }

    // Extract full text as a structured Rust object
    println!("\n--- Processing Full Text with Structured API ---");
    match grobid_rs::fulltext_to_structured(pdf_path) {
        Ok(document) => {
            println!("Successfully extracted document as a Rust struct:");

            // Display document metadata
            if let Some(title) = &document.metadata.title {
                println!("Title: {}", title);
            }
            println!("Authors: {} author(s)", document.metadata.authors.len());

            // Display document structure summary
            if let Some(full_text) = &document.full_text {
                println!("Sections: {}", full_text.sections.len());
                println!("Figures: {}", full_text.figures.len());
                println!("Tables: {}", full_text.tables.len());
            }

            println!("References: {}", document.references.len());

            println!("\nThe structured API gives you programmatic access to the entire document.");
        }
        Err(e) => {
            eprintln!("Error processing full text as structured data: {}", e);
        }
    }

    println!("\nAll processing completed.");
    Ok(())
}

/// Helper function to truncate long text for display
fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        // Ensure we don't cut in the middle of a UTF-8 character
        let truncated = text
            .char_indices()
            .take_while(|(i, _)| *i < max_length)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(max_length);

        format!(
            "{}... [truncated {} more characters]",
            &text[..truncated],
            text.len() - truncated
        )
    }
}
