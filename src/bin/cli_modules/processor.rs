use crate::cli_modules::output::{create_spinner, display_cache_stats, write_output};
use crate::cli_modules::types::{CliExitCode, JsonFormat, OutputFormat};
use grobid_rs::format::FormatConverter;
use grobid_rs::{
    fulltext_to_tei_cached, process_header_cached, process_references_cached, CacheConfig,
};
use std::path::Path;
use tracing::error;

/// Process a document header
pub fn process_header(
    pdf_file: &Path,
    output_format: OutputFormat,
    output_file: &Option<std::path::PathBuf>,
    cache_config: CacheConfig,
    show_stats: bool,
    json_format: JsonFormat,
) -> CliExitCode {
    // Create progress spinner
    let message = format!("Processing header from PDF: {}", pdf_file.display());
    let spinner = create_spinner(&message);

    // Process header with caching
    let tei_result = process_header_cached(pdf_file, cache_config);

    // Finish progress bar
    if let Some(pb) = spinner {
        if tei_result.is_ok() {
            pb.finish_with_message("Header processing complete.");
        } else {
            pb.finish_with_message("Header processing failed.");
        }
    }

    let tei_result = match tei_result {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to process header: {}", e);
            eprintln!("Error: Failed to process header: {}", e);
            return e.into();
        }
    };

    // Convert to requested format
    let output = match output_format {
        OutputFormat::Tei => tei_result,
        OutputFormat::Json => {
            let pretty = matches!(json_format, JsonFormat::Pretty);
            match FormatConverter::header_to_json_with_options(&tei_result, pretty) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to convert header TEI to JSON: {}", e);
                    eprintln!("Error: Failed to convert header TEI to JSON: {}", e);
                    return CliExitCode::FormatConversionError;
                }
            }
        }
        OutputFormat::Text => match FormatConverter::tei_to_text(&tei_result) {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to convert TEI to text: {}", e);
                eprintln!("Error: Failed to convert TEI to text: {}", e);
                return CliExitCode::FormatConversionError;
            }
        },
        OutputFormat::Bibtex => {
            error!("BibTeX output not applicable for headers");
            eprintln!("Error: BibTeX output not applicable for headers. Use --output-format=tei, json, or text instead.");
            return CliExitCode::InvalidInput;
        }
    };

    match write_output(&output, output_file) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to write output: {}", e);
            eprintln!("Error: Failed to write output: {}", e);
            return CliExitCode::IoError;
        }
    }

    // Show cache statistics if requested
    if show_stats {
        display_cache_stats();
    }

    CliExitCode::Success
}

/// Process a document's full text
pub fn process_fulltext(
    pdf_file: &Path,
    output_format: OutputFormat,
    output_file: &Option<std::path::PathBuf>,
    cache_config: CacheConfig,
    show_stats: bool,
    json_format: JsonFormat,
) -> CliExitCode {
    // Create progress spinner
    let message = format!("Processing full text from PDF: {}", pdf_file.display());
    let spinner = create_spinner(&message);

    // Process fulltext with caching
    let tei_result = fulltext_to_tei_cached(pdf_file, cache_config);

    // Finish progress bar
    if let Some(pb) = spinner {
        if tei_result.is_ok() {
            pb.finish_with_message("Full text processing complete.");
        } else {
            pb.finish_with_message("Full text processing failed.");
        }
    }

    let tei_result = match tei_result {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to process full text: {}", e);
            eprintln!("Error: Failed to process full text: {}", e);
            return e.into();
        }
    };

    // Convert to requested format
    let output = match output_format {
        OutputFormat::Tei => tei_result,
        OutputFormat::Json => {
            let pretty = matches!(json_format, JsonFormat::Pretty);
            match FormatConverter::references_to_json_with_options(&tei_result, pretty) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to convert references TEI to JSON: {}", e);
                    eprintln!("Error: Failed to convert references TEI to JSON: {}", e);
                    return CliExitCode::FormatConversionError;
                }
            }
        }
        OutputFormat::Text => match FormatConverter::tei_to_text(&tei_result) {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to convert TEI to text: {}", e);
                eprintln!("Error: Failed to convert TEI to text: {}", e);
                return CliExitCode::FormatConversionError;
            }
        },
        OutputFormat::Bibtex => {
            error!("BibTeX output not applicable for full text");
            eprintln!("Error: BibTeX output not applicable for full text. Use --output-format=tei, json, or text instead.");
            return CliExitCode::InvalidInput;
        }
    };

    match write_output(&output, output_file) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to write output: {}", e);
            eprintln!("Error: Failed to write output: {}", e);
            return CliExitCode::IoError;
        }
    }

    // Show cache statistics if requested
    if show_stats {
        display_cache_stats();
    }

    CliExitCode::Success
}

/// Process document references
pub fn process_references(
    pdf_file: &Path,
    output_format: OutputFormat,
    output_file: &Option<std::path::PathBuf>,
    cache_config: CacheConfig,
    show_stats: bool,
    json_format: JsonFormat,
) -> CliExitCode {
    // Create progress spinner
    let message = format!("Extracting references from PDF: {}", pdf_file.display());
    let spinner = create_spinner(&message);

    // Process references with caching
    let tei_result = process_references_cached(pdf_file, cache_config);

    // Finish progress bar
    if let Some(pb) = spinner {
        if tei_result.is_ok() {
            pb.finish_with_message("References extraction complete.");
        } else {
            pb.finish_with_message("References extraction failed.");
        }
    }

    let tei_result = match tei_result {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to extract references: {}", e);
            eprintln!("Error: Failed to extract references: {}", e);
            return e.into();
        }
    };

    // Convert to requested format
    let output = match output_format {
        OutputFormat::Tei => tei_result,
        OutputFormat::Json => {
            let pretty = matches!(json_format, JsonFormat::Pretty);
            match FormatConverter::tei_to_json_with_options(&tei_result, pretty) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to convert TEI to JSON: {}", e);
                    eprintln!("Error: Failed to convert TEI to JSON: {}", e);
                    return CliExitCode::FormatConversionError;
                }
            }
        }
        OutputFormat::Text => match FormatConverter::tei_to_text(&tei_result) {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to convert TEI to text: {}", e);
                eprintln!("Error: Failed to convert TEI to text: {}", e);
                return CliExitCode::FormatConversionError;
            }
        },
        OutputFormat::Bibtex => match FormatConverter::tei_refs_to_bibtex(&tei_result) {
            Ok(bibtex) => bibtex,
            Err(e) => {
                error!("Failed to convert TEI references to BibTeX: {}", e);
                eprintln!("Error: Failed to convert TEI references to BibTeX: {}", e);
                return CliExitCode::FormatConversionError;
            }
        },
    };

    match write_output(&output, output_file) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to write output: {}", e);
            eprintln!("Error: Failed to write output: {}", e);
            return CliExitCode::IoError;
        }
    }

    // Show cache statistics if requested
    if show_stats {
        display_cache_stats();
    }

    CliExitCode::Success
}
