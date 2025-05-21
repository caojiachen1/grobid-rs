use grobid_rs::get_cache_stats;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::info;

/// Write output to file or stdout
pub fn write_output(output: &str, output_file: &Option<PathBuf>) -> Result<(), std::io::Error> {
    match output_file {
        Some(path) => {
            // Create progress bar for file writing
            let spinner = if std::io::stdout().is_terminal() {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                        .template("{spinner:.green} {msg}")
                        .unwrap(),
                );
                pb.set_message(format!("Writing output to file: {}", path.display()));
                Some(pb)
            } else {
                info!("Writing output to file: {}", path.display());
                None
            };

            // Write file
            let write_result = fs::write(path, output);

            // Finish progress bar
            if let Some(pb) = spinner {
                if write_result.is_ok() {
                    pb.finish_with_message(format!("Output written to: {}", path.display()));
                } else {
                    pb.finish_with_message("Failed to write output file.");
                }
            }

            write_result?;
            Ok(())
        }
        None => {
            info!("Printing output to stdout");
            println!("\n--- OUTPUT ---");
            println!("{}", output);
            Ok(())
        }
    }
}

/// Display cache statistics
pub fn display_cache_stats() {
    let stats = get_cache_stats();
    println!("\nCache Statistics:");
    println!("  Hits: {}", stats.hits);
    println!("  Misses: {}", stats.misses);
    println!("  Bytes read: {} KB", stats.bytes_read / 1024);
    println!("  Bytes written: {} KB", stats.bytes_written / 1024);
    println!("  Estimated time saved: {} ms", stats.time_saved_ms);
}

/// Create a progress spinner
pub fn create_spinner(message: &str) -> Option<ProgressBar> {
    if std::io::stdout().is_terminal() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        Some(pb)
    } else {
        info!("{}", message);
        None
    }
}
