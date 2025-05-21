use clap::Parser;
use grobid_rs::{init, is_initialized, reset_cache_stats, CacheConfig, GrobidConfig};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::process::ExitCode;
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli_modules;
use cli_modules::processor::{process_fulltext, process_header, process_references};
use cli_modules::types::{Args, CliExitCode, Commands};

fn main() -> ExitCode {
    let args = Args::parse();

    // Initialize tracing with appropriate log level
    let subscriber_level = match args.trace_level {
        cli_modules::types::CliLogLevel::Off => Level::ERROR,
        cli_modules::types::CliLogLevel::Error => Level::ERROR,
        cli_modules::types::CliLogLevel::Warn => Level::WARN,
        cli_modules::types::CliLogLevel::Info => Level::INFO,
        cli_modules::types::CliLogLevel::Debug => Level::DEBUG,
        cli_modules::types::CliLogLevel::Trace => Level::TRACE,
    };

    // Initialize the tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(subscriber_level)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    // Set the subscriber as the default
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Failed to set tracing subscriber: {}", e);
        return CliExitCode::GenericError.into();
    }

    debug!(
        "Grobid CLI initialized with trace level: {:?}",
        subscriber_level
    );

    // Set up Grobid configuration
    let mut config = GrobidConfig::new(args.grobid_base.clone())
        .with_max_memory(args.max_memory.clone())
        .with_log_level(args.log_level.into());

    // Add custom JVM options
    for option in &args.jvm_options {
        config = config.with_jvm_option(option);
    }

    // Add system properties
    for (key, value) in &args.properties {
        config = config.with_system_property(key, value);
    }

    // Reset cache statistics
    reset_cache_stats();

    // Create spinner for initialization
    let spinner = if std::io::stdout().is_terminal() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Initializing Grobid engine...");
        debug!("Using progress spinner for JVM initialization");
        Some(pb)
    } else {
        info!("Initializing Grobid engine...");
        None
    };

    // Initialize Grobid
    let init_result = init(&config);

    // Stop the spinner
    if let Some(pb) = spinner {
        if init_result.is_ok() {
            pb.finish_with_message("  Grobid engine initialized successfully.  ");
        } else {
            pb.finish_with_message("  Grobid engine initialization failed!  ");
        }
    }

    // Handle initialization errors
    if let Err(e) = init_result {
        error!("Failed to initialize Grobid: {}", e);
        eprintln!("Error: Failed to initialize Grobid: {}", e);
        return CliExitCode::JvmInitError.into();
    }

    // Verify that initialization was successful
    if !is_initialized() {
        error!("Grobid engine initialization did not complete properly");
        eprintln!("Error: Grobid engine initialization did not complete properly");
        return CliExitCode::NotInitialized.into();
    }

    // Set up cache configuration
    let cache_config = CacheConfig {
        enabled: !args.no_cache,
        skip_existing: args.skip_existing,
        force_reprocess: args.force_reprocess,
    };

    // Process the command
    let exit_code = match &args.command {
        Commands::Header {
            pdf_file,
            output_format,
        } => process_header(
            pdf_file,
            *output_format,
            &args.output_file,
            cache_config,
            args.stats,
            args.json_format,
        ),
        Commands::Fulltext {
            pdf_file,
            output_format,
        } => process_fulltext(
            pdf_file,
            *output_format,
            &args.output_file,
            cache_config,
            args.stats,
            args.json_format,
        ),
        Commands::References {
            pdf_file,
            output_format,
        } => process_references(
            pdf_file,
            *output_format,
            &args.output_file,
            cache_config,
            args.stats,
            args.json_format,
        ),
    };

    exit_code.into()
}
