//! Batch Processing Example
//!
//! This example demonstrates how to process multiple PDF documents in parallel or sequentially.
//! When built with the "parallel" feature, it uses rayon for parallel processing.
//!
//! To run with parallel processing:
//! ```
//! cargo run --example batch_processing --features "parallel"
//! ```

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Conditionally import rayon for parallel processing
#[cfg(feature = "parallel")]
use rayon::prelude::*;

// Maximum concurrent processing threads when parallel feature is enabled
#[cfg(feature = "parallel")]
const MAX_THREADS: usize = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    #[cfg(feature = "parallel")]
    let required_args = 4;
    #[cfg(not(feature = "parallel"))]
    let required_args = 3;

    if args.len() < required_args + 1 {
        #[cfg(feature = "parallel")]
        {
            eprintln!(
                "Usage: {} <grobid_home_path> <input_dir> <output_dir> [num_threads]",
                args[0]
            );
            eprintln!("  num_threads: Optional, defaults to {}", MAX_THREADS);
        }
        #[cfg(not(feature = "parallel"))]
        {
            eprintln!(
                "Usage: {} <grobid_home_path> <input_dir> <output_dir>",
                args[0]
            );
        }
        process::exit(1);
    }

    let grobid_home = Path::new(&args[1]);
    let input_dir = Path::new(&args[2]);
    let output_dir = Path::new(&args[3]);

    // Parse optional thread count when parallel feature is enabled
    #[cfg(feature = "parallel")]
    let thread_count = if args.len() > 4 {
        args[4].parse::<usize>().unwrap_or(MAX_THREADS)
    } else {
        MAX_THREADS
    };

    // Verify paths
    if !grobid_home.exists() || !grobid_home.is_dir() {
        eprintln!(
            "Error: Grobid home directory not found at {}",
            grobid_home.display()
        );
        process::exit(1);
    }

    if !input_dir.exists() || !input_dir.is_dir() {
        eprintln!(
            "Error: Input directory not found at {}",
            input_dir.display()
        );
        process::exit(1);
    }

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Initialize Grobid
    println!("Initializing Grobid from {}", grobid_home.display());
    let config = grobid_rs::GrobidConfig::new(grobid_home);
    grobid_rs::init_with_config(&config)?;

    // Find all PDF files in the input directory
    let pdf_files: Vec<PathBuf> = fs::read_dir(input_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();

    let total_files = pdf_files.len();
    println!("Found {} PDF files to process", total_files);

    if total_files == 0 {
        println!("No PDF files found in {}", input_dir.display());
        return Ok(());
    }

    // Setup progress tracking
    let processed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    // Share errors across threads
    let errors = Arc::new(Mutex::new(Vec::<(String, String)>::new()));

    #[cfg(feature = "parallel")]
    {
        // Configure thread pool
        println!("Processing with {} threads", thread_count);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();

        // Process files in parallel
        pool.install(|| {
            pdf_files.par_iter().for_each(|pdf_path| {
                process_file(
                    pdf_path,
                    output_dir,
                    &processed,
                    &failed,
                    total_files,
                    &errors,
                );
            });
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        println!("Processing files sequentially (parallel feature not enabled)");

        // Process files sequentially
        for pdf_path in &pdf_files {
            process_file(
                pdf_path,
                output_dir,
                &processed,
                &failed,
                total_files,
                &errors,
            );
        }
    };

    // Report results
    let elapsed = start_time.elapsed();
    let success_count = processed.load(Ordering::SeqCst) - failed.load(Ordering::SeqCst);

    println!("\nProcessing Summary:");
    println!("-------------------");
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!("Total files: {}", total_files);
    println!("Successfully processed: {}", success_count);
    println!("Failed: {}", failed.load(Ordering::SeqCst));
    println!(
        "Average processing time: {:.2}s per document",
        elapsed.as_secs_f64() / total_files as f64
    );

    // Report errors if any
    let error_list = errors.lock().unwrap();
    if !error_list.is_empty() {
        println!("\nErrors:");
        println!("-------");
        for (file, error) in error_list.iter() {
            println!("{}: {}", file, error);
        }
    }

    Ok(())
}

// Helper function to process a single file
fn process_file(
    pdf_path: &Path,
    output_dir: &Path,
    processed: &Arc<AtomicUsize>,
    failed: &Arc<AtomicUsize>,
    total_files: usize,
    errors: &Arc<Mutex<Vec<(String, String)>>>,
) {
    let file_name = pdf_path.file_name().unwrap().to_string_lossy().to_string();
    let file_stem = pdf_path.file_stem().unwrap().to_string_lossy().to_string();
    let output_path = output_dir.join(format!("{}.tei.xml", file_stem));

    println!("Processing: {}", file_name);

    match grobid_rs::fulltext_to_tei(pdf_path) {
        Ok(tei_xml) => {
            // Write the TEI XML to the output file
            match File::create(&output_path) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(tei_xml.as_bytes()) {
                        let err_msg = format!("Failed to write output file: {}", e);
                        errors.lock().unwrap().push((file_name.clone(), err_msg));
                        failed.fetch_add(1, Ordering::SeqCst);
                    } else {
                        println!("✓ Completed: {} -> {}", file_name, output_path.display());
                    }
                }
                Err(e) => {
                    let err_msg = format!("Failed to create output file: {}", e);
                    errors.lock().unwrap().push((file_name.clone(), err_msg));
                    failed.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        Err(e) => {
            let err_msg = format!("Processing error: {}", e);
            errors.lock().unwrap().push((file_name.clone(), err_msg));
            failed.fetch_add(1, Ordering::SeqCst);
        }
    }

    let count = processed.fetch_add(1, Ordering::SeqCst) + 1;
    println!(
        "Progress: {}/{} files ({:.1}%)",
        count,
        total_files,
        (count as f64 / total_files as f64) * 100.0
    );
}
