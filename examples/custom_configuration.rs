//! Custom Configuration Example
//!
//! This example demonstrates how to use custom configuration options
//! with grobid-rs to control various aspects of the document processing.

use std::env;
use std::path::Path;
use std::process;

/// Custom configuration for Grobid processing
#[derive(Debug, Clone)]
struct GrobidConfig {
    // Citation consolidation level (0=no, 1=fast, 2=full)
    consolidate_citations: i32,
    // Include raw citations in output
    include_raw_citations: bool,
    // Include raw affiliations in output
    include_raw_affiliations: bool,
    // Include TEI coordinates in output
    include_tei_coordinates: bool,
    // Start page for processing (None = from beginning)
    start_page: Option<i32>,
    // End page for processing (None = until end)
    end_page: Option<i32>,
    // Generate stable TEI IDs
    generate_ids: bool,
    // Whether to process figures/tables
    process_figures_tables: bool,
}

impl Default for GrobidConfig {
    fn default() -> Self {
        Self {
            consolidate_citations: 0,
            include_raw_citations: false,
            include_raw_affiliations: false,
            include_tei_coordinates: false,
            start_page: None,
            end_page: None,
            generate_ids: false,
            process_figures_tables: true,
        }
    }
}

/// Imagine these are the actual grobid-rs functions that accept a config parameter
fn process_fulltext_with_config(pdf_path: &Path, config: &GrobidConfig) -> Result<String, String> {
    // In a real implementation, this would call Grobid with the configuration
    // For this example, we'll just create some sample output
    let output = format!(
        r#"<TEI>
  <processingInfo>
    <config>
      <consolidateCitations>{}</consolidateCitations>
      <includeRawCitations>{}</includeRawCitations>
      <includeRawAffiliations>{}</includeRawAffiliations>
      <includeTeiCoordinates>{}</includeTeiCoordinates>
      <startPage>{}</startPage>
      <endPage>{}</endPage>
      <generateIds>{}</generateIds>
      <processFiguresTables>{}</processFiguresTables>
    </config>
  </processingInfo>
  <text>
    <body>
      <p>This is example output based on custom configuration.</p>
      <p>Processing PDF: {}</p>
    </body>
  </text>
</TEI>"#,
        config.consolidate_citations,
        config.include_raw_citations,
        config.include_raw_affiliations,
        config.include_tei_coordinates,
        config
            .start_page
            .map_or("none".to_string(), |p| p.to_string()),
        config
            .end_page
            .map_or("none".to_string(), |p| p.to_string()),
        config.generate_ids,
        config.process_figures_tables,
        pdf_path.display()
    );

    Ok(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <grobid_home_path> <pdf_file_path>", args[0]);
        process::exit(1);
    }

    let grobid_home = Path::new(&args[1]);
    let pdf_path = Path::new(&args[2]);

    // Verify paths (just for example consistency)
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

    // Initialize Grobid (in actual code)
    println!("Initializing Grobid from {}", grobid_home.display());
    // grobid_rs::init(grobid_home)?;

    // Example 1: Using default configuration
    println!("\n--- Example 1: Default Configuration ---");
    let default_config = GrobidConfig::default();
    println!("Configuration: {:?}", default_config);

    match process_fulltext_with_config(pdf_path, &default_config) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 2: Configuration for high-quality citation processing
    println!("\n--- Example 2: Citation-focused Configuration ---");
    let citation_config = GrobidConfig {
        consolidate_citations: 2, // Full consolidation
        include_raw_citations: true,
        ..GrobidConfig::default() // Keep other defaults
    };
    println!("Configuration: {:?}", citation_config);

    match process_fulltext_with_config(pdf_path, &citation_config) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 3: Configuration for layout analysis (with coordinates)
    println!("\n--- Example 3: Layout Analysis Configuration ---");
    let layout_config = GrobidConfig {
        include_tei_coordinates: true,
        generate_ids: true,
        ..GrobidConfig::default()
    };
    println!("Configuration: {:?}", layout_config);

    match process_fulltext_with_config(pdf_path, &layout_config) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 4: Processing specific page range
    println!("\n--- Example 4: Page Range Configuration ---");
    let page_range_config = GrobidConfig {
        start_page: Some(2),
        end_page: Some(5),
        ..GrobidConfig::default()
    };
    println!("Configuration: {:?}", page_range_config);

    match process_fulltext_with_config(pdf_path, &page_range_config) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\nAll configurations demonstrated successfully.");
    Ok(())
}
