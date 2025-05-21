//! Test for modular library implementation
//!
//! This example demonstrates the use of the refactored grobid-rs library.
//! It shows how to initialize Grobid and process a PDF file using the
//! three main functions.

use std::env;
use std::path::Path;
use std::process;

fn main() -> Result<(), grobid_rs::GrobidError> {
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

    // Initialize Grobid with configuration
    println!("Initializing Grobid from: {}", grobid_home.display());
    let config = grobid_rs::GrobidConfig::builder()
        .base_path(grobid_home)
        .max_memory("1G")
        .build();

    grobid_rs::init_with_config(&config)?;

    // Extract header information (title, authors, abstract, etc.)
    println!("\n--- Processing Header ---");
    match grobid_rs::process_header(pdf_path) {
        Ok(header_xml) => {
            println!("Successfully extracted header information");
            println!("{}", truncate_output(&header_xml, 400));
        }
        Err(e) => {
            eprintln!("Error processing header: {}", e);
        }
    }

    // Extract full text (entire document content)
    println!("\n--- Processing Full Text ---");
    match grobid_rs::fulltext_to_tei(pdf_path) {
        Ok(fulltext_xml) => {
            println!("Successfully extracted full text");
            println!("{}", truncate_output(&fulltext_xml, 400));
        }
        Err(e) => {
            eprintln!("Error processing full text: {}", e);
        }
    }

    // Extract references (bibliography)
    println!("\n--- Processing References ---");
    match grobid_rs::process_references(pdf_path) {
        Ok(references_xml) => {
            println!("Successfully extracted references");
            println!("{}", truncate_output(&references_xml, 400));
        }
        Err(e) => {
            eprintln!("Error processing references: {}", e);
        }
    }

    println!("\nAll processing completed.");
    Ok(())
}

/// Helper function to truncate long XML output for display
fn truncate_output(text: &str, max_length: usize) -> String {
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
