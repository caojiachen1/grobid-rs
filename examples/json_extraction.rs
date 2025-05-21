//! XML/TEI Extraction Example
//!
//! This example demonstrates how to use grobid-rs to extract structured information
//! from PDF documents in XML/TEI format.

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
    grobid_rs::init_with_config(&config)?;

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

    println!("\nAll processing completed.");
    Ok(())
}

/// Helper function to truncate long text for display
fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!(
            "{}... [truncated {} more characters]",
            &text[..max_length],
            text.len() - max_length
        )
    }
}
