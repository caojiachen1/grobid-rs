use std::path::PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use std::process::ExitCode;

/// Log verbosity levels for Grobid CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliLogLevel {
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

/// Exit codes for the CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliExitCode {
    /// Operation completed successfully
    Success = 0,
    /// Generic error
    GenericError = 1,
    /// Grobid not initialized
    NotInitialized = 2,
    /// JNI error
    JniError = 3,
    /// JVM initialization error
    JvmInitError = 4,
    /// Java exception
    JavaException = 5,
    /// PdfAlto error
    PdfAltoError = 6,
    /// Invalid input error
    InvalidInput = 7,
    /// Configuration error
    ConfigError = 8,
    /// I/O error
    IoError = 9,
    /// Version mismatch error
    VersionMismatch = 10,
    /// CLI argument error
    CliArgError = 120,
    /// Format conversion error
    FormatConversionError = 127,
    /// Unknown error
    Unknown = 255,
}

impl From<grobid_rs::GrobidError> for CliExitCode {
    fn from(error: grobid_rs::GrobidError) -> Self {
        match error {
            grobid_rs::GrobidError::NotInitialised => CliExitCode::NotInitialized,
            grobid_rs::GrobidError::Jni(_) => CliExitCode::JniError,
            grobid_rs::GrobidError::JvmInitialization(_) => CliExitCode::JvmInitError,
            grobid_rs::GrobidError::Java(_) => CliExitCode::JavaException,
            grobid_rs::GrobidError::PdfAlto(_) => CliExitCode::PdfAltoError,
            grobid_rs::GrobidError::InvalidInput(_) => CliExitCode::InvalidInput,
            grobid_rs::GrobidError::Configuration(_) => CliExitCode::ConfigError,
            grobid_rs::GrobidError::Io(_) => CliExitCode::IoError,
            grobid_rs::GrobidError::VersionMismatch { .. } => CliExitCode::VersionMismatch,
        }
    }
}

impl From<CliExitCode> for ExitCode {
    fn from(code: CliExitCode) -> Self {
        ExitCode::from(code as u8)
    }
}

/// Supported output formats for Grobid results
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// TEI XML format (native Grobid format)
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
    after_help = "OUTPUT FORMAT SUPPORT:\n  - TEI (tei): Native XML format from Grobid (all commands)\n  - JSON (json): Structured data extracted from TEI (all commands)\n  - Text (text): Plain text extraction from TEI (all commands)\n  - BibTeX (bibtex): Citation format (references command only)\n\nCACHE OPTIONS:\n  --skip-existing       Skip processing if cached results exist (default: true)\n  --force-reprocess     Force reprocessing even if cached results exist\n  --no-cache            Disable caching entirely\n  --stats               Display cache statistics after processing\n\nEXAMPLES:\n  # Process a document header and output as TEI XML\n  grobid-cli header paper.pdf\n\n  # Extract references as BibTeX\n  grobid-cli references paper.pdf\n\n  # Process full text and save as JSON to a file\n  grobid-cli fulltext paper.pdf --output-format=json -o paper.json\n\n  # Use custom Grobid base directory and more memory\n  grobid-cli --grobid-base=/path/to/grobid --max-memory=2G header paper.pdf\n\n  # Force reprocessing (ignore cache)\n  grobid-cli fulltext paper.pdf --force-reprocess\n\n  # Show cache statistics\n  grobid-cli --stats fulltext paper.pdf"
)]
pub struct Args {
    /// Path to the Grobid base directory (containing grobid/ and runtime/).
    #[clap(short, long, value_parser, default_value_os_t = PathBuf::from(env!("GROBID_RS_ASSETS_PATH")))]
    pub grobid_base: PathBuf,

    /// Maximum memory allocation for JVM (e.g. "1G", "2G", etc.)
    #[clap(long, default_value = "1G")]
    pub max_memory: String,

    /// Log verbosity level for Grobid JVM
    #[clap(long, value_enum, default_value = "info")]
    pub log_level: CliLogLevel,

    /// Trace verbosity level for CLI and library code
    #[clap(long, value_enum, default_value = "info")]
    pub trace_level: CliLogLevel,

    /// Add system property (format: key=value)
    #[clap(short = 'D', long = "property", value_parser = parse_key_val)]
    pub properties: Vec<(String, String)>,

    /// Add JVM option
    #[clap(short = 'J', long = "jvm-option")]
    pub jvm_options: Vec<String>,

    /// Skip processing if cached results exist (default: true)
    #[clap(long, default_value_t = true)]
    pub skip_existing: bool,

    /// Force reprocessing even if cached results exist
    #[clap(long, default_value_t = false)]
    pub force_reprocess: bool,

    /// Disable the cache entirely
    #[clap(long, default_value_t = false)]
    pub no_cache: bool,

    /// Display cache statistics after processing
    #[clap(long, default_value_t = false)]
    pub stats: bool,

    /// Output file path (if not provided, output is printed to stdout)
    #[clap(
        short = 'o',
        long = "output",
        help = "Write output to a file instead of stdout"
    )]
    pub output_file: Option<PathBuf>,

    /// The operation to perform
    #[clap(subcommand)]
    pub command: Commands,
}

/// Available Grobid processing commands
#[derive(Subcommand, Debug)]
pub enum Commands {
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

/// Parse a key-value pair in the format "key=value"
pub fn parse_key_val(s: &str) -> Result<(String, String), String> {
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