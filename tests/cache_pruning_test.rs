use std::path::{Path, PathBuf};
use std::fs;
use tempfile::tempdir;
use std::env;
use std::time::Duration;
use std::thread;

// Helper function to get a test PDF path
fn get_test_pdf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join("sample.pdf")
}

// Initialize Grobid for testing
fn initialize_grobid() {
    // Use the bundled test resources
    let grobid_base = PathBuf::from(env!("GROBID_RS_ASSETS_PATH"));
    grobid_rs::init_with_config(&grobid_rs::GrobidConfig::new(&grobid_base))
        .expect("Failed to initialize Grobid");
}

#[test]
fn test_cache_pruning() {
    // Create a temporary directory for caching
    let cache_dir = tempdir().expect("Failed to create temp dir");
    let cache_path = cache_dir.path().to_path_buf();
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_path.to_str().unwrap());
    
    // Set small cache size limit for testing
    env::set_var("GROBID_RS_CACHE_MAX_SIZE", "1048576"); // 1MB
    
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
    
    // Helper function to get total size of cache
    let get_cache_dir_size = || {
        let mut total_size = 0;
        for entry in fs::read_dir(&cache_path).expect("Failed to read cache dir") {
            let entry = entry.expect("Failed to read directory entry");
            let metadata = entry.metadata().expect("Failed to get metadata");
            if metadata.is_file() {
                total_size += metadata.len();
            }
        }
        total_size
    };
    
    // Test 1: Process a few times to populate cache
    {
        let cache_config = grobid_rs::CacheConfig {
            enabled: true,
            skip_existing: true,
            force_reprocess: false,
        };
        
        // Process 5 times with different cache keys (we'll simulate this by modifying the output type)
        for i in 0..5 {
            // Set up a unique output type to create multiple cache entries
            let output_type = match i % 3 {
                0 => grobid_rs::OutputType::Tei,
                1 => grobid_rs::OutputType::Json,
                _ => grobid_rs::OutputType::Bibtex,
            };
            
            // Get cache path for this type
            let cache_path = grobid_rs::get_cache_path(&pdf_path, output_type)
                .expect("Failed to get cache path");
            
            // Process file which will create a cache entry
            let result = grobid_rs::process_with_cache(
                &pdf_path,
                output_type,
                cache_config,
                || Ok(format!("Test content for iteration {}", i))
            ).expect("Failed to process file");
            
            assert!(result.contains(&format!("Test content for iteration {}", i)));
            assert!(cache_path.exists(), "Cache file should exist for iteration {}", i);
            
            // Add some delay to ensure different access times
            thread::sleep(Duration::from_millis(100));
        }
        
        // Verify we have cache files
        let file_count = count_cache_files();
        assert!(file_count > 0, "Cache should contain files after processing");
        println!("Cache contains {} files after initial population", file_count);
        
        // Check current cache size
        let initial_size = get_cache_dir_size();
        println!("Initial cache size: {} bytes", initial_size);
    }
    
    // Test 2: Run pruning and verify it cleaned up files
    {
        // Get size before pruning
        let before_size = get_cache_dir_size();
        let before_count = count_cache_files();
        
        // Set a very small max size to force pruning
        let max_size = 1; // 1 byte - this should remove all files
        
        // Run prune
        let (files_removed, bytes_removed) = grobid_rs::prune_cache(max_size)
            .expect("Failed to prune cache");
        
        // Verify files were removed
        let after_count = count_cache_files();
        let after_size = get_cache_dir_size();
        
        println!("Before pruning: {} files, {} bytes", before_count, before_size);
        println!("After pruning: {} files, {} bytes", after_count, after_size);
        println!("Reported removed: {} files, {} bytes", files_removed, bytes_removed);
        
        assert!(files_removed > 0, "Should have removed at least one file");
        assert!(bytes_removed > 0, "Should have removed some bytes");
        assert!(after_count < before_count, "Should have fewer files after pruning");
        assert!(after_size < before_size, "Cache should be smaller after pruning");
        assert!(after_size <= max_size, "Cache should respect max size");
    }
    
    // Test 3: Clear cache and verify all files are removed
    {
        // First create some new cache files
        let cache_config = grobid_rs::CacheConfig {
            enabled: true,
            skip_existing: false,
            force_reprocess: true,
        };
        
        // Process to create a cache entry
        let result = grobid_rs::process_with_cache(
            &pdf_path,
            grobid_rs::OutputType::Tei,
            cache_config,
            || Ok("Clear test content".to_string())
        ).expect("Failed to process file");
        
        assert!(result.contains("Clear test content"));
        
        // Verify we have at least one cache file
        let before_clear = count_cache_files();
        assert!(before_clear > 0, "Should have at least one cache file before clearing");
        
        // Clear cache
        let (files_removed, _) = grobid_rs::clear_cache()
            .expect("Failed to clear cache");
        
        // Verify all files were removed
        let after_clear = count_cache_files();
        assert_eq!(after_clear, 0, "Cache should be empty after clearing");
        assert_eq!(files_removed, before_clear, "Should have removed all files");
    }
    
    // Clean up
    env::remove_var("GROBID_RS_CACHE_DIR");
    env::remove_var("GROBID_RS_CACHE_MAX_SIZE");
}

#[test]
fn test_cache_size_reporting() {
    // Create a temporary directory for caching
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    // Test cache size reporting functions
    let size = grobid_rs::get_cache_size().expect("Failed to get cache size");
    assert_eq!(size, 0, "Empty cache should report 0 size");
    
    // Create a test file in the cache directory
    let test_file = cache_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    
    // Check size is now non-zero
    let size = grobid_rs::get_cache_size().expect("Failed to get cache size");
    assert!(size > 0, "Cache size should be non-zero after adding file");
    
    // Test human readable size
    let human_size = grobid_rs::get_human_readable_cache_size()
        .expect("Failed to get human readable size");
    assert!(!human_size.is_empty(), "Human readable size should not be empty");
    
    // Get cache summary
    let summary = grobid_rs::get_cache_summary().expect("Failed to get cache summary");
    assert!(!summary.is_empty(), "Cache summary should not be empty");
    
    // Clean up
    env::remove_var("GROBID_RS_CACHE_DIR");
}