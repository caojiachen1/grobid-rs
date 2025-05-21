//! Test example to verify that vendored files are working correctly.
//! This will only initialize Grobid to verify the vendored files are properly loaded.

use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Grobid initialization with vendored files...");
    
    // The build.rs script sets the GROBID_RS_ASSETS_PATH environment variable
    // that points to where the Grobid resources are stored.
    // We can retrieve this variable to see where the vendored files were installed.
    if let Ok(assets_path) = env::var("GROBID_RS_ASSETS_PATH") {
        println!("GROBID_RS_ASSETS_PATH = {}", assets_path);
    } else {
        println!("GROBID_RS_ASSETS_PATH not set - this is unexpected");
    }
    
    // In a real application, you would pass a path to grobid_rs::init(),
    // but the library should use the vendored files automatically.
    println!("Initializing Grobid...");
    let config = grobid_rs::GrobidConfig::new(Path::new("."));
    match grobid_rs::init_with_config(&config) {
        Ok(_) => {
            println!("SUCCESS: Grobid initialized successfully!");
            println!("Vendored files are working correctly.");
        },
        Err(e) => {
            println!("ERROR: Failed to initialize Grobid: {}", e);
            println!("Vendored files might not be correctly set up.");
            return Err(e.into());
        }
    }
    
    Ok(())
}