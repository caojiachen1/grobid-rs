use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use tempfile::tempdir;
use std::env;

// Helper function to get the test PDF path
fn get_test_pdf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets")
        .join("sample.pdf")
}

// Helper function to build a command for the CLI
fn build_cli_command(args: &[&str]) -> Command {
    let cargo_bin = env!("CARGO_BIN_EXE_grobid-cli");
    let mut cmd = Command::new(cargo_bin);
    cmd.args(args);
    cmd
}

#[test]
#[ignore] // Requires building the CLI binary first
fn test_cli_default_flags() {
    // Use a temporary directory for cache during the test
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    let pdf_path = get_test_pdf_path();
    
    // Run CLI with default flags (cache enabled, skip_existing=true)
    let status = build_cli_command(&["header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI");
    
    assert!(status.success(), "CLI execution failed with default flags");
    
    // Check cache directory to ensure a file was created
    let cache_files_count = fs::read_dir(cache_dir.path())
        .expect("Failed to read cache dir")
        .count();
    
    assert!(cache_files_count > 0, "No cache files were created with default flags");
    
    // Clear the environment variable
    env::remove_var("GROBID_RS_CACHE_DIR");
}

#[test]
#[ignore] // Requires building the CLI binary first
fn test_cli_force_reprocess_flag() {
    // Use a temporary directory for cache during the test
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    let pdf_path = get_test_pdf_path();
    
    // First run to populate cache
    let status1 = build_cli_command(&["header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI");
    
    assert!(status1.success(), "First CLI execution failed");
    
    // Get modification time of the cached file
    let cache_file_path = fs::read_dir(cache_dir.path())
        .expect("Failed to read cache dir")
        .next()
        .expect("No cache file found")
        .unwrap()
        .path();
    
    let first_mod_time = fs::metadata(&cache_file_path)
        .expect("Failed to get metadata")
        .modified()
        .expect("Failed to get modification time");
    
    // Second run with --force-reprocess flag
    let status2 = build_cli_command(&["--force-reprocess", "header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI with force-reprocess");
    
    assert!(status2.success(), "Second CLI execution failed");
    
    // Check if cache file was updated
    let second_mod_time = fs::metadata(&cache_file_path)
        .expect("Failed to get metadata")
        .modified()
        .expect("Failed to get modification time");
    
    assert!(second_mod_time > first_mod_time, "Cache file wasn't updated with --force-reprocess");
    
    // Clear the environment variable
    env::remove_var("GROBID_RS_CACHE_DIR");
}

#[test]
#[ignore] // Requires building the CLI binary first
fn test_cli_no_cache_flag() {
    // Use a temporary directory for cache during the test
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    let pdf_path = get_test_pdf_path();
    
    // Run CLI with --no-cache flag
    let status = build_cli_command(&["--no-cache", "header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI");
    
    assert!(status.success(), "CLI execution failed with --no-cache flag");
    
    // Check cache directory to ensure no files were created
    let cache_files_count = fs::read_dir(cache_dir.path())
        .expect("Failed to read cache dir")
        .count();
    
    assert_eq!(cache_files_count, 0, "Cache files were created despite --no-cache flag");
    
    // Clear the environment variable
    env::remove_var("GROBID_RS_CACHE_DIR");
}

#[test]
#[ignore] // Requires building the CLI binary first
fn test_cli_skip_existing_flag() {
    // Use a temporary directory for cache during the test
    let cache_dir = tempdir().expect("Failed to create temp dir");
    
    // Set environment variable to control cache directory
    env::set_var("GROBID_RS_CACHE_DIR", cache_dir.path().to_str().unwrap());
    
    let pdf_path = get_test_pdf_path();
    
    // First run to populate cache
    let status1 = build_cli_command(&["header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI");
    
    assert!(status1.success(), "First CLI execution failed");
    
    // Get modification time of the cached file
    let cache_file_path = fs::read_dir(cache_dir.path())
        .expect("Failed to read cache dir")
        .next()
        .expect("No cache file found")
        .unwrap()
        .path();
    
    let first_mod_time = fs::metadata(&cache_file_path)
        .expect("Failed to get metadata")
        .modified()
        .expect("Failed to get modification time");
    
    // Second run with explicit --skip-existing flag (which is the default)
    let status2 = build_cli_command(&["--skip-existing", "header", pdf_path.to_str().unwrap()])
        .status()
        .expect("Failed to execute CLI with skip-existing");
    
    assert!(status2.success(), "Second CLI execution failed");
    
    // Check if cache file was NOT updated (should use existing cache)
    let second_mod_time = fs::metadata(&cache_file_path)
        .expect("Failed to get metadata")
        .modified()
        .expect("Failed to get modification time");
    
    assert_eq!(first_mod_time, second_mod_time, "Cache file was updated despite --skip-existing flag");
    
    // Clear the environment variable
    env::remove_var("GROBID_RS_CACHE_DIR");
}