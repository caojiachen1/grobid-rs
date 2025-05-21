use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
    Tei,
    Json,
    Bibtex,
}

/// A CLI tool to interact with Grobid for processing scholarly documents.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
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
        #[clap(long, value_enum, default_value_t = OutputFormat::Tei)]
        output_format: OutputFormat,
    },
    
    /// Process full text of the document
    Fulltext {
        /// Path to the PDF file to process
        pdf_file: PathBuf,
        
        /// Output format for the processing results
        #[clap(long, value_enum, default_value_t = OutputFormat::Tei)]
        output_format: OutputFormat,
    },
    
    /// Extract and process references/citations
    References {
        /// Path to the PDF file to process
        pdf_file: PathBuf,
        
        /// Output format for the processing results
        #[clap(long, value_enum, default_value_t = OutputFormat::Bibtex)]
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Set up Grobid configuration
    let mut config = grobid_rs::GrobidConfig::new(args.grobid_base)
        .with_max_memory(args.max_memory)
        .with_log_level(args.log_level.into());
    
    // Add system properties
    for (key, value) in args.properties {
        config = config.with_system_property(key, value);
    }
    
    // Add JVM options
    for option in args.jvm_options {
        config = config.with_jvm_option(option);
    }

    println!("Initializing Grobid with config: {:?}", config);
    grobid_rs::init_with_config(&config)
        .map_err(|e| anyhow::anyhow!("Grobid initialization failed: {}", e))?;

    // Process according to the command
    match args.command {
        Commands::Header { pdf_file, output_format } => {
            println!("Processing header for: {}", pdf_file.display());
            let result = grobid_rs::process_header(&pdf_file)
                .map_err(|e| anyhow::anyhow!("Header processing failed: {}", e))?;
            
            // For now, only TEI output is supported in the library, so we'll just print it
            // In the future, this would handle the different output formats
            match output_format {
                OutputFormat::Tei => println!("\n--- Header TEI ---\n{}", result),
                OutputFormat::Json => println!("JSON output not yet implemented for headers"),
                OutputFormat::Bibtex => println!("BibTeX output not applicable for headers"),
            }
        },
        
        Commands::Fulltext { pdf_file, output_format } => {
            println!("Processing full text for: {}", pdf_file.display());
            let result = grobid_rs::fulltext_to_tei(&pdf_file)
                .map_err(|e| anyhow::anyhow!("Full text processing failed: {}", e))?;
            
            match output_format {
                OutputFormat::Tei => println!("\n--- Full Text TEI ---\n{}", result),
                OutputFormat::Json => println!("JSON output not yet implemented for full text"),
                OutputFormat::Bibtex => println!("BibTeX output not applicable for full text"),
            }
        },
        
        Commands::References { pdf_file, output_format } => {
            println!("Processing references for: {}", pdf_file.display());
            let result = grobid_rs::process_references(&pdf_file)
                .map_err(|e| anyhow::anyhow!("References processing failed: {}", e))?;
            
            match output_format {
                OutputFormat::Tei => println!("\n--- References TEI ---\n{}", result),
                OutputFormat::Json => println!("JSON output not yet implemented for references"),
                OutputFormat::Bibtex => println!("BibTeX conversion not yet implemented"),
            }
        },
    }

    Ok(())
}