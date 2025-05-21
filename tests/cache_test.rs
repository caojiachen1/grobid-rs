use grobid_rs::{fulltext_to_tei_cached, init_with_config, CacheConfig, GrobidConfig};
use insta::assert_snapshot;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

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
fn test_cache_behavior() {
    // Create a temporary directory for caching
    let cache_dir = tempdir().expect("Failed to create temp dir");
    let cache_path = cache_dir.path().to_path_buf();

    // Initialize Grobid
    initialize_grobid();

    // Get test PDF path
    let pdf_path = get_test_pdf_path();

    // Helper function to get the number of files in the cache directory
    let count_cache_files = || {
        fs::read_dir(&cache_path)
            .expect("Failed to read cache dir")
            .count()
    };

    // Test 1: First run should process the PDF and store in cache
    {
        // Configure to use our temporary cache directory
        let old_cache_dir = std::env::var("GROBID_RS_CACHE_DIR").ok();
        std::env::set_var("GROBID_RS_CACHE_DIR", cache_path.to_str().unwrap());

        // Set up cache config for first run
        let cache_config = CacheConfig {
            enabled: true,
            skip_existing: true,
            force_reprocess: false,
        };

        // Process PDF (should cache the result)
        let result1 =
            fulltext_to_tei_cached(&pdf_path, cache_config).expect("Failed to process PDF");

        // Check that something was cached
        assert_eq!(
            count_cache_files(),
            1,
            "Cache should contain 1 file after first run"
        );

        // Reset environment if needed
        if let Some(old_dir) = old_cache_dir {
            std::env::set_var("GROBID_RS_CACHE_DIR", old_dir);
        } else {
            std::env::remove_var("GROBID_RS_CACHE_DIR");
        }

        // The output is large, so we'll take just the first 500 characters for snapshot testing
        let result_sample = result1.chars().take(500).collect::<String>();
        assert_snapshot!("first_run_output_sample", result_sample);
    }

    // Test 2: Second run should use the cache
    {
        // Use the same temporary cache directory
        let old_cache_dir = std::env::var("GROBID_RS_CACHE_DIR").ok();
        std::env::set_var("GROBID_RS_CACHE_DIR", cache_path.to_str().unwrap());

        // Set up cache config for second run (using cache)
        let cache_config = CacheConfig {
            enabled: true,
            skip_existing: true,
            force_reprocess: false,
        };

        // Process PDF again (should use cache)
        let result2 =
            fulltext_to_tei_cached(&pdf_path, cache_config).expect("Failed to process PDF");

        // Check that cache file count hasn't changed
        assert_eq!(
            count_cache_files(),
            1,
            "Cache should still contain 1 file after second run"
        );

        // Reset environment if needed
        if let Some(old_dir) = old_cache_dir {
            std::env::set_var("GROBID_RS_CACHE_DIR", old_dir);
        } else {
            std::env::remove_var("GROBID_RS_CACHE_DIR");
        }

        // Take a sample for snapshot testing
        let result_sample = result2.chars().take(500).collect::<String>();
        assert_snapshot!("second_run_output_sample", result_sample);
    }

    // Test 3: Run with force_reprocess should ignore cache
    {
        // Use the same temporary cache directory
        let old_cache_dir = std::env::var("GROBID_RS_CACHE_DIR").ok();
        std::env::set_var("GROBID_RS_CACHE_DIR", cache_path.to_str().unwrap());

        // Set up cache config with force_reprocess
        let cache_config = CacheConfig {
            enabled: true,
            skip_existing: true,
            force_reprocess: true,
        };

        // Process PDF with force_reprocess (should ignore cache)
        let _ = fulltext_to_tei_cached(&pdf_path, cache_config).expect("Failed to process PDF");

        // Check that cache file count is still the same
        assert_eq!(
            count_cache_files(),
            1,
            "Cache should still contain 1 file after force reprocess"
        );

        // Reset environment if needed
        if let Some(old_dir) = old_cache_dir {
            std::env::set_var("GROBID_RS_CACHE_DIR", old_dir);
        } else {
            std::env::remove_var("GROBID_RS_CACHE_DIR");
        }
    }

    // Test 4: Run with cache disabled should not use or update cache
    {
        // Use the same temporary cache directory
        let old_cache_dir = std::env::var("GROBID_RS_CACHE_DIR").ok();
        std::env::set_var("GROBID_RS_CACHE_DIR", cache_path.to_str().unwrap());

        // Set up cache config with cache disabled
        let cache_config = CacheConfig {
            enabled: false,
            skip_existing: true,
            force_reprocess: false,
        };

        // Process PDF with cache disabled
        let _ = fulltext_to_tei_cached(&pdf_path, cache_config).expect("Failed to process PDF");

        // Check that cache file count is still the same
        assert_eq!(
            count_cache_files(),
            1,
            "Cache should still contain 1 file after disabled cache run"
        );

        // Reset environment if needed
        if let Some(old_dir) = old_cache_dir {
            std::env::set_var("GROBID_RS_CACHE_DIR", old_dir);
        } else {
            std::env::remove_var("GROBID_RS_CACHE_DIR");
        }
    }
}
