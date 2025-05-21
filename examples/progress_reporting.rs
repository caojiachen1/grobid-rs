//! Progress Reporting and Cancellation Example
//!
//! This example demonstrates how to implement progress reporting and
//! cancellation when processing documents with grobid-rs.

use std::env;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Processing stages for progress reporting
#[derive(Debug, Clone, Copy, PartialEq)]
enum ProcessingStage {
    Initializing,
    PdfToXml,
    SegmentingDocument,
    ProcessingHeader,
    ProcessingBody,
    ProcessingReferences,
    ProcessingFiguresTables,
    CleaningUp,
    Complete,
}

impl ProcessingStage {
    fn description(&self) -> &'static str {
        match self {
            Self::Initializing => "Initializing",
            Self::PdfToXml => "Converting PDF to XML",
            Self::SegmentingDocument => "Segmenting document",
            Self::ProcessingHeader => "Processing header",
            Self::ProcessingBody => "Processing document body",
            Self::ProcessingReferences => "Processing references",
            Self::ProcessingFiguresTables => "Processing figures and tables",
            Self::CleaningUp => "Cleaning up",
            Self::Complete => "Complete",
        }
    }

    fn percentage(&self) -> u8 {
        match self {
            Self::Initializing => 0,
            Self::PdfToXml => 10,
            Self::SegmentingDocument => 20,
            Self::ProcessingHeader => 35,
            Self::ProcessingBody => 50,
            Self::ProcessingReferences => 70,
            Self::ProcessingFiguresTables => 85,
            Self::CleaningUp => 95,
            Self::Complete => 100,
        }
    }
}

/// Progress information sent from the worker to the UI
#[allow(dead_code)]
struct ProgressInfo {
    stage: ProcessingStage,
    progress: f64,
    message: String,
    time: std::time::Instant,
}

/// Simulate processing a document with progress updates
fn process_document(
    pdf_path: &Path,
    progress_sender: Sender<ProgressInfo>,
    should_cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let filename = pdf_path.file_name().unwrap_or_default().to_string_lossy();

    // Helper for sending progress updates
    let send_progress = |stage: ProcessingStage, message: &str| {
        let _ = progress_sender.send(ProgressInfo {
            stage,
            progress: stage.percentage() as f64,
            message: message.to_string(),
            time: Instant::now(),
        });

        // Check for cancellation signal
        if should_cancel.load(Ordering::Relaxed) {
            return Err("Processing cancelled by user".to_string());
        }

        // Simulate processing time
        thread::sleep(Duration::from_millis(500));
        Ok(())
    };

    // Start processing with stage updates
    send_progress(
        ProcessingStage::Initializing,
        &format!("Starting to process {}", filename),
    )?;

    send_progress(
        ProcessingStage::PdfToXml,
        "Running pdfalto for XML conversion",
    )?;

    send_progress(
        ProcessingStage::SegmentingDocument,
        "Applying segmentation model",
    )?;

    send_progress(
        ProcessingStage::ProcessingHeader,
        "Extracting title, authors, and abstracts",
    )?;

    send_progress(
        ProcessingStage::ProcessingBody,
        "Processing main document content",
    )?;

    // Check cancellation one more time
    if should_cancel.load(Ordering::Relaxed) {
        return Err("Processing cancelled by user".to_string());
    }

    send_progress(
        ProcessingStage::ProcessingReferences,
        "Extracting and parsing references",
    )?;

    send_progress(
        ProcessingStage::ProcessingFiguresTables,
        "Processing figures and tables",
    )?;

    send_progress(ProcessingStage::CleaningUp, "Finalizing document")?;

    send_progress(ProcessingStage::Complete, "Processing complete")?;

    // Return simulated result
    Ok(format!("<TEI>\n  <text>\n    <body>\n      <p>Document {} processed successfully with progress reporting.</p>\n    </body>\n  </text>\n</TEI>", filename))
}

/// Display a simple progress bar in the terminal
fn display_progress_bar(percentage: u8, width: usize, description: &str) {
    let bar_width = width - 10; // Allow space for percentage text
    let filled_width = (bar_width as f32 * (percentage as f32 / 100.0)) as usize;
    let empty_width = bar_width - filled_width;

    print!("\r[");
    for _ in 0..filled_width {
        print!("=");
    }
    for _ in 0..empty_width {
        print!(" ");
    }
    print!("] {:3}% {}", percentage, description);
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
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

    // Verify paths
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

    // Initialize Grobid (in a real application)
    println!("Initializing Grobid from {}", grobid_home.display());
    // grobid_rs::init(grobid_home)?;

    // Set up progress channel
    let (tx, rx): (Sender<ProgressInfo>, Receiver<ProgressInfo>) = mpsc::channel();

    // Set up cancellation signal
    let should_cancel = Arc::new(AtomicBool::new(false));
    let should_cancel_clone = should_cancel.clone();

    // Set up cancellation handler in a separate thread
    thread::spawn(move || {
        println!("Press Ctrl+C to cancel processing");
        let _ = std::io::stdin().read_line(&mut String::new());
        should_cancel_clone.store(true, Ordering::Relaxed);
    });

    // Start processing in a separate thread
    let pdf_path_clone = pdf_path.to_path_buf();
    let processing_thread =
        thread::spawn(move || process_document(&pdf_path_clone, tx, should_cancel));

    // Main thread monitors progress
    let start_time = Instant::now();
    let mut last_percentage = 0;
    let progress_bar_width = 50;

    println!("\nProcessing: {}", pdf_path.display());
    println!("Progress:");

    for progress in rx {
        let percentage = progress.stage.percentage();
        if percentage != last_percentage {
            display_progress_bar(percentage, progress_bar_width, progress.stage.description());
            last_percentage = percentage;
        }

        if progress.stage == ProcessingStage::Complete {
            break;
        }
    }

    // Wait for processing to finish and get the result
    let result = match processing_thread.join() {
        Ok(Ok(output)) => {
            println!(
                "\n\nSuccess! Processing completed in {:.2} seconds",
                start_time.elapsed().as_secs_f32()
            );
            println!("\nOutput sample:");
            println!("{}", output);
            Ok(())
        }
        Ok(Err(e)) => {
            println!("\n\nError: {}", e);
            Err(Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
        }
        Err(_) => {
            println!("\n\nError: Processing thread panicked");
            Err(Box::new(std::io::Error::other("Thread panic")) as Box<dyn std::error::Error>)
        }
    };

    result
}
