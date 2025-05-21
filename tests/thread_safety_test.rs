use grobid_rs::{fulltext_to_tei_cached, init_with_config, CacheConfig, GrobidConfig};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

#[test]
#[ignore] // Only run when explicitly requested, can be resource intensive
fn test_thread_safety() {
    // Initialize Grobid
    initialize_grobid();

    // Get test PDF path
    let pdf_path = get_test_pdf_path();

    // Number of parallel operations to run
    let num_operations = 5;

    // Create a vector to store results
    let results = Arc::new(Mutex::new(Vec::new()));

    // Configure cache
    let cache_config = CacheConfig {
        enabled: false, // Disable cache to ensure we're testing threading
        skip_existing: false,
        force_reprocess: true,
    };

    // Process the PDF in parallel
    (0..num_operations).into_par_iter().for_each(|i| {
        // Process the PDF
        match fulltext_to_tei_cached(&pdf_path, cache_config) {
            Ok(result) => {
                // Store success result
                let success_marker = format!("Thread {} succeeded", i);

                // Check result size as a basic validation
                let result_size = result.len();
                assert!(result_size > 1000, "Result too small to be valid");

                // Lock and store the success
                let mut results = results.lock().unwrap();
                results.push(success_marker);
            }
            Err(e) => {
                // Store error result
                let error_marker = format!("Thread {} failed: {}", i, e);

                // Lock and store the error
                let mut results = results.lock().unwrap();
                results.push(error_marker);
            }
        }
    });

    // Check results
    let final_results = results.lock().unwrap();
    assert_eq!(
        final_results.len(),
        num_operations,
        "Not all threads completed"
    );

    // Ensure all operations succeeded
    let failures: Vec<_> = final_results
        .iter()
        .filter(|r| r.contains("failed"))
        .collect();

    assert!(
        failures.is_empty(),
        "Some thread operations failed: {:?}",
        failures
    );
}
