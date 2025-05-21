//! JSON Extraction Example
//!
//! This example demonstrates how to use grobid-rs to extract structured information
//! from PDF documents in JSON format using the typed Rust data structures.

use std::env;
use std::path::Path;
use std::process;
use serde_json;

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
        eprintln!("Error: Grobid home directory not found at {}", grobid_home.display());
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

    // Extract header information using the new JSON API
    println!("\n--- Processing Header with JSON API ---");
    match grobid_rs::process_header_json(pdf_path) {
        Ok(header) => {
            println!("Successfully extracted header information:");
            
            // Print title
            if let Some(title) = &header.title {
                println!("Title: {}", title);
            } else {
                println!("Title: [Not found]");
            }
            
            // Print authors
            println!("Authors:");
            if header.authors.is_empty() {
                println!("  [None found]");
            } else {
                for (i, author) in header.authors.iter().enumerate() {
                    print!("  {}. ", i + 1);
                    if let Some(name) = &author.full_name {
                        println!("{}", name);
                    } else {
                        let mut name_parts = Vec::new();
                        if let Some(first) = &author.first_name {
                            name_parts.push(first.clone());
                        }
                        if let Some(middle) = &author.middle_name {
                            name_parts.push(middle.clone());
                        }
                        if let Some(last) = &author.last_name {
                            name_parts.push(last.clone());
                        }
                        println!("{}", name_parts.join(" "));
                    }
                }
            }
            
            // Print abstract
            if let Some(abstract_text) = &header.abstract_text {
                println!("Abstract: {}", truncate_text(abstract_text, 200));
            }
            
            // Print DOI
            if let Some(doi) = &header.doi {
                println!("DOI: {}", doi);
            }
            
            // Convert to JSON and print
            println!("\nFull header metadata as JSON:");
            let json = serde_json::to_string_pretty(&header)?;
            println!("{}", truncate_text(&json, 500));
        }
        Err(e) => {
            eprintln!("Error processing header: {}", e);
        }
    }

    // Extract references using the JSON API
    println!("\n--- Processing References with JSON API ---");
    match grobid_rs::process_references_json(pdf_path) {
        Ok(references) => {
            println!("Successfully extracted {} references:", references.len());
            
            // Print the first few references
            let count = std::cmp::min(3, references.len());
            for (i, reference) in references.iter().enumerate().take(count) {
                println!("Reference {}:", i + 1);
                
                if let Some(title) = &reference.title {
                    println!("  Title: {}", title);
                }
                
                if !reference.authors.is_empty() {
                    println!("  Authors: {}", reference.authors.join(", "));
                }
                
                if let Some(date) = &reference.date {
                    if let Some(year) = &date.year {
                        println!("  Year: {}", year);
                    }
                }
                
                if let Some(venue) = &reference.venue {
                    println!("  Venue: {}", venue);
                }
                
                println!();
            }
            
            if references.len() > count {
                println!("... and {} more references.", references.len() - count);
            }
        }
        Err(e) => {
            eprintln!("Error processing references: {}", e);
        }
    }

    // Extract full text using the JSON API
    println!("\n--- Processing Full Text with JSON API ---");
    match grobid_rs::fulltext_to_json(pdf_path) {
        Ok(document) => {
            println!("Successfully extracted full document:");
            println!("  Title: {}", document.metadata.title.clone().unwrap_or_else(|| "[Not found]".to_string()));
            println!("  Authors: {} author(s)", document.metadata.authors.len());
            
            if let Some(full_text) = &document.full_text {
                println!("  Sections: {} section(s)", full_text.sections.len());
                println!("  Figures: {} figure(s)", full_text.figures.len());
                println!("  Tables: {} table(s)", full_text.tables.len());
            }
            
            println!("  References: {} reference(s)", document.references.len());
            
            // Print as JSON
            println!("\nFull document metadata as JSON (truncated):");
            let document_clone = document.clone();
            let json = serde_json::to_string_pretty(&document_clone)?;
            println!("{}", truncate_text(&json, 500));
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
        format!("{}... [truncated {} more characters]", 
                &text[..max_length], 
                text.len() - max_length)
    }
}