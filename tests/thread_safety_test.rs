use grobid_rs::{fulltext_to_tei_cached, init, CacheConfig, GrobidConfig};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

// Helper function to initialize Grobid
fn initialize_grobid() {
    let grobid_home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("grobid_assets")
        .join(format!("grobid-{}", grobid_rs::GROBID_VERSION));

    // Only initialize if not already initialized
    let config = GrobidConfig::new(grobid_home);
    let _ = init(&config);
}

#[test]
fn test_parallel_processing() {
    // Initialize once at the start
    initialize_grobid();

    // Get test PDF path
    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join("sample.pdf");

    // If the test PDF doesn't exist, just return (test would be meaningless)
    if !pdf_path.exists() {
        println!("Test PDF not found, skipping thread safety test");
        return;
    }

    // Use a shared counter to track successful operations
    let success_counter = Arc::new(Mutex::new(0));

    // Define cache config
    let cache_config = CacheConfig {
        enabled: true,
        skip_existing: false,
        force_reprocess: true,
    };

    // Number of parallel operations to run
    let num_operations = 5;

    // Process the PDF in parallel
    let mut handles = vec![];
    for i in 0..num_operations {
        let pdf_path = pdf_path.clone();
        let cache_config = cache_config;
        let success_counter = Arc::clone(&success_counter);

        let handle = thread::spawn(move || {
            // Process the PDF
            match fulltext_to_tei_cached(&pdf_path, cache_config) {
                Ok(_result) => {
                    // Increment success counter
                    let mut counter = success_counter.lock().unwrap();
                    *counter += 1;
                    println!("Thread {} successfully processed document", i);
                }
                Err(e) => {
                    println!("Thread {} failed to process document: {}", i, e);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Check that all operations succeeded
    let final_count = *success_counter.lock().unwrap();
    assert_eq!(
        final_count, num_operations,
        "Not all parallel operations succeeded"
    );
}
