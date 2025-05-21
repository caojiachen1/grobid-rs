use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;

mod format_converter;
use format_converter::FormatConverter;

/// Log verbosity levels for Grobid CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliLogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<CliLogLevel> for grobid_rs::LogLevel {
    fn from(level: CliLogLevel) -> Self {
        match level {
            CliLogLevel::Off => grobid_rs::LogLevel::Off,
            CliLogLevel::Error => grobid_rs::LogLevel::Error,
            CliLogLevel::Warn => grobid_rs::LogLevel::Warn,
            CliLogLevel::Info => grobid_rs::LogLevel::Info,
            CliLogLevel::Debug => grobid_rs::LogLevel::Debug,
            CliLogLevel::Trace => grobid_rs::LogLevel::Trace,
        }
    }
}

/// Output format options for Grobid processing results
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// TEI XML format (default for most operations)
    Tei,
    /// JSON format (for structured data)
    Json,
    /// BibTeX format (for bibliographic entries)
    Bibtex,
    /// Plain text format
    Text,
}

/// A CLI tool to interact with Grobid for processing scholarly documents.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(
    after_help = "OUTPUT FORMAT SUPPORT:\n  - TEI (tei): Native XML format from Grobid (all commands)\n  - JSON (json): Structured data extracted from TEI (all commands)\n  - Text (text): Plain text extraction from TEI (all commands)\n  - BibTeX (bibtex): Citation format (references command only)\n\nEXAMPLES:\n  # Process a document header and output as TEI XML\n  grobid-cli header paper.pdf\n\n  # Extract references as BibTeX\n  grobid-cli references paper.pdf\n\n  # Process full text and save as JSON to a file\n  grobid-cli fulltext paper.pdf --output-format=json -o paper.json\n\n  # Use custom Grobid base directory and more memory\n  grobid-cli --grobid-base=/path/to/grobid --max-memory=2G header paper.pdf"
)]
struct Args {
    /// Path to the Grobid base directory (containing grobid/ and runtime/).
    #[clap(short, long, value_parser, default_value_os_t = PathBuf::from(env!("GROBID_RS_ASSETS_PATH")))]
    grobid_base: PathBuf,

    /// Maximum memory allocation for JVM (e.g. "1G", "2G", etc.)
    #[clap(long, default_value = "1G")]
    max_memory: String,

    /// Log verbosity level
    #[clap(long, value_enum, default_value = "info")]
    log_level: CliLogLevel,

    /// Add system property (format: key=value)
    #[clap(short = 'D', long = "property", value_parser = parse_key_val)]
    properties: Vec<(String, String)>,

    /// Add JVM option
    #[clap(short = 'J', long = "jvm-option")]
    jvm_options: Vec<String>,

    /// Output file path (if not provided, output is printed to stdout)
    #[clap(
        short = 'o',
        long = "output",
        help = "Write output to a file instead of stdout"
    )]
    output_file: Option<PathBuf>,

    /// The operation to perform
    #[clap(subcommand)]
    command: Commands,
}

/// Available Grobid processing commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Process document headers (title, authors, abstract, etc.)
    Header {
        /// Path to the PDF file to process
        pdf_file: PathBuf,

        /// Output format for the processing results
        #[clap(long, value_enum, default_value_t = OutputFormat::Tei, help = "Output format: TEI (default), JSON, or Text (BibTeX not supported)")]
        output_format: OutputFormat,
    },

    /// Process full text of the document
    Fulltext {
        /// Path to the PDF file to process
        pdf_file: PathBuf,

        /// Output format for the processing results
        #[clap(long, value_enum, default_value_t = OutputFormat::Tei, help = "Output format: TEI (default), JSON, or Text (BibTeX not supported)")]
        output_format: OutputFormat,
    },

    /// Extract and process references/citations
    References {
        /// Path to the PDF file to process
        pdf_file: PathBuf,

        /// Output format for the processing results
        #[clap(long, value_enum, default_value_t = OutputFormat::Bibtex, help = "Output format: BibTeX (default), TEI, JSON, or Text")]
        output_format: OutputFormat,
    },
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("System property must be in format key=value".to_string());
    }

    let key = parts[0].trim().to_string();
    let val = parts[1].trim().to_string();

    if key.is_empty() {
        return Err("Key cannot be empty".to_string());
    }

    Ok((key, val))
}

/// Write output to either file or stdout based on user preference
/// Write output to either file or stdout based on user preference
fn write_output(output: &str, output_file: &Option<PathBuf>) -> anyhow::Result<()> {
    match output_file {
        Some(path) => {
            println!("Writing output to file: {}", path.display());
            fs::write(path, output)?;
        }
        None => {
            println!("\n--- OUTPUT ---");
            println!("{}", output);
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Set up Grobid configuration
    let mut config = grobid_rs::GrobidConfig::new(args.grobid_base.clone())
        .with_max_memory(args.max_memory.clone())
        .with_log_level(args.log_level.into());

    // Add system properties
    for (key, value) in &args.properties {
        config = config.with_system_property(key.clone(), value.clone());
    }

    // Add JVM options
    for option in &args.jvm_options {
        config = config.with_jvm_option(option.clone());
    }

    println!("Initializing Grobid engine...");
    grobid_rs::init_with_config(&config)
        .map_err(|e| anyhow::anyhow!("Grobid initialization failed: {}", e))?;

    // Process according to the command
    match &args.command {
        Commands::Header {
            pdf_file,
            output_format,
        } => {
            println!("Processing header from PDF: {}", pdf_file.display());
            let tei_result = grobid_rs::process_header(pdf_file)
                .map_err(|e| anyhow::anyhow!("Header processing failed: {}", e))?;

            // Handle different output formats
            let output = match output_format {
                OutputFormat::Tei => tei_result,
                OutputFormat::Json => FormatConverter::tei_to_json(&tei_result)?,
                OutputFormat::Text => FormatConverter::tei_to_text(&tei_result)?,
                OutputFormat::Bibtex => {
                    return Err(anyhow::anyhow!("BibTeX output not applicable for headers. Use --output-format=tei, json, or text instead."))
                },
            };

            write_output(&output, &args.output_file)?;
        }

        Commands::Fulltext {
            pdf_file,
            output_format,
        } => {
            println!("Processing full text from PDF: {}", pdf_file.display());
            let tei_result = grobid_rs::fulltext_to_tei(pdf_file)
                .map_err(|e| anyhow::anyhow!("Full text processing failed: {}", e))?;

            // Handle different output formats
            let output = match output_format {
                OutputFormat::Tei => tei_result,
                OutputFormat::Json => FormatConverter::tei_to_json(&tei_result)?,
                OutputFormat::Text => FormatConverter::tei_to_text(&tei_result)?,
                OutputFormat::Bibtex => {
                    return Err(anyhow::anyhow!("BibTeX output not applicable for full text. Use --output-format=tei, json, or text instead."))
                },
            };

            write_output(&output, &args.output_file)?;
        }

        Commands::References {
            pdf_file,
            output_format,
        } => {
            println!("Extracting references from PDF: {}", pdf_file.display());
            let tei_result = grobid_rs::process_references(pdf_file)
                .map_err(|e| anyhow::anyhow!("References processing failed: {}", e))?;

            // Handle different output formats
            let output = match output_format {
                OutputFormat::Tei => tei_result,
                OutputFormat::Json => FormatConverter::tei_to_json(&tei_result)?,
                OutputFormat::Text => FormatConverter::tei_to_text(&tei_result)?,
                OutputFormat::Bibtex => FormatConverter::tei_refs_to_bibtex(&tei_result)?,
            };

            write_output(&output, &args.output_file)?;
        }
    }

    Ok(())
}
