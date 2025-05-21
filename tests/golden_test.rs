use std::path::{Path, PathBuf};
use std::fs;
use tempfile::tempdir;
use std::env;
use insta::{assert_snapshot, with_settings};
use insta::Settings;
use grobid_rs::{init_with_config, GrobidConfig, CacheConfig, fulltext_to_tei_cached};

// Helper function to get the test PDF path
fn get_test_pdf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join("sample.pdf")
}

fn initialize_grobid() {
    let grobid_home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("grobid_assets")
        .join("grobid-0.8.2");
    
    // Only initialize if not already initialized
    let config = GrobidConfig::new(grobid_home);
    let _ = init_with_config(&config);
}

// This test creates a golden snapshot of the TEI output to ensure 
// compatibility across versions
#[test]
fn test_golden_tei_output() {
    // Initialize Grobid
    initialize_grobid();
    
    // Get test PDF path
    let pdf_path = get_test_pdf_path();
    
    // Process the PDF without caching
    let cache_config = CacheConfig {
        enabled: false,
        skip_existing: false,
        force_reprocess: true,
    };
    
    let tei_result = fulltext_to_tei_cached(&pdf_path, cache_config)
        .expect("Failed to process PDF");
    
    // We'll need to configure insta to properly handle the large XML output
    // and redact any dynamic content that might change between runs
    with_settings!({
        // Only keep the first 2000 characters for snapshot size management
        // while still capturing the essential structure
        snapshot_path => PathBuf::from("snapshots/tei_golden"),
        description => "TEI XML golden output",
        omit_expression => true,
        filters => {
            // Redact timestamps which can change between runs
            let re_timestamp = regex::Regex::new(r#"when="[^"]+"#).unwrap();
            move |content: &str| {
                let content = re_timestamp.replace_all(content, r#"when="REDACTED_TIMESTAMP""#);
                content.to_string()
            }
        }
    }, {
        // Just take the first 2000 characters of the XML to keep snapshots manageable
        // but still capture the essential structure
        let sample = tei_result.chars().take(2000).collect::<String>();
        assert_snapshot!("tei_golden_output", sample);
    });
}

// Test to verify that cache output matches the direct output
#[test]
fn test_cache_output_consistency() {
    // Create a temporary directory for caching
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Initialize Grobid
    initialize_grobid();
    
    // Get test PDF path
    let pdf_path = get_test_pdf_path();
    
    // First, get direct output without caching
    let direct_config = CacheConfig {
        enabled: false,
        skip_existing: false,
        force_reprocess: true,
    };
    
    let direct_result = fulltext_to_tei_cached(&pdf_path, direct_config)
        .expect("Failed to process PDF directly");
    
    // Now process with caching enabled
    let old_cache_dir = std::env::var("GROBID_RS_CACHE_DIR").ok();
    std::env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    let cache_config = CacheConfig {
        enabled: true,
        skip_existing: false,
        force_reprocess: false,
    };
    
    // First run to populate cache
    let _ = fulltext_to_tei_cached(&pdf_path, cache_config)
        .expect("Failed to process PDF with caching");
    
    // Second run to read from cache
    let cached_config = CacheConfig {
        enabled: true,
        skip_existing: true,
        force_reprocess: false,
    };
    
    let cached_result = fulltext_to_tei_cached(&pdf_path, cached_config)
        .expect("Failed to read from cache");
    
    // Reset environment if needed
    if let Some(old_dir) = old_cache_dir {
        std::env::set_var("GROBID_RS_CACHE_DIR", old_dir);
    } else {
        std::env::remove_var("GROBID_RS_CACHE_DIR");
    }
    
    // Check that cached result matches direct result
    assert_eq!(direct_result, cached_result, "Cached output doesn't match direct output");
}