//! # GROBID-RS
//!
//! A Rust implementation of GROBID for extracting structured information from research papers.
//!
//! This library provides a Rust interface to the GROBID (GeneRation Of BIbliographic Data)
//! library, which extracts and parses information from research papers in PDF format.
//!
//! ## Features
//!
//! - Header metadata extraction (title, authors, abstract)
//! - Bibliographic references extraction
//! - Full-text extraction with section structure
//! - Conversion to JSON, TEI, and plain text formats
//! - Document caching for performance
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::path::Path;
//! use grobid_rs::GrobidConfigBuilder;
//!
//! // Initialize GROBID with default configuration
//! grobid_rs::init(GrobidConfigBuilder::default().build().unwrap()).unwrap();
//!
//! // Process a PDF file to extract header metadata as JSON
//! let pdf_path = Path::new("path/to/paper.pdf");
//! let json = grobid_rs::process_header_json(&pdf_path).unwrap();
//! println!("Extracted metadata: {}", json);
//!
//! // Process a PDF file to extract structured bibliographic references
//! let references = grobid_rs::process_references_structured(&pdf_path).unwrap();
//! println!("Found {} references", references.len());
//! ```

// Export our modules
mod api;
mod cache;
mod config;
mod engine;
mod errors;
pub mod format;
mod jni_handle;
pub mod models;

// Re-export the main types for users
pub use api::{
    // Common API functions
    common::parse_tei_str,

    // Full-text processing API
    fulltext::{fulltext_to_json, fulltext_to_json_with_options, fulltext_to_structured},
    // Header processing API
    header::{process_header_json, process_header_json_with_options, process_header_structured},

    // References processing API
    references::{
        process_references_json, process_references_json_with_options,
        process_references_structured,
    },
};

// Re-export the format module's ParsedTei type
pub use format::tei::ParsedTei;

// Re-export models for direct access
pub use models::{
    Author, Date, DocumentMetadata, Equation, Figure, FullText, GrobidDocument, Reference, Section,
    Table, Venue,
};

// Re-export configuration types
pub use config::{
    GrobidAnalysisConfig, GrobidAnalysisConfigBuilder, GrobidConfig, GrobidConfigBuilder,
};

// Re-export error type
pub use errors::GrobidError;

// Re-export the JNI handle for advanced users
pub use jni_handle::JniHandle;

// Export core engine functions
pub use engine::{fulltext_to_tei, process_header, process_references, run_pdfalto};

/// Log verbosity levels for Grobid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

// Expected Grobid version - must match the version bundled with the library
pub const GROBID_VERSION: &str = "0.8.2";

// JVM and Engine globals
use jni::{objects::*, JavaVM};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static JVM: OnceCell<JavaVM> = OnceCell::new();
static ENGINE: OnceCell<Mutex<GlobalRef>> = OnceCell::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Boot JVM + Grobid with the provided configuration.
///
/// The configuration's `base_path` should point to a directory containing `runtime/` and `grobid/`.
/// The `runtime` directory is expected to have a subdirectory named after the OS
/// (e.g., "linux-latest", "macos-14", "windows-latest") which is created by the build script.
///
/// # Examples
///
/// ```no_run
/// use grobid_rs::{GrobidConfig, init, is_initialized};
///
/// // Create configuration
/// let config = GrobidConfig::default();
///
/// // Initialize
/// init(&config).expect("Failed to initialize Grobid");
/// assert!(is_initialized());
/// ```
pub fn init(config: &config::GrobidConfig) -> Result<(), errors::GrobidError> {
    // Validate configuration
    config.validate()?;

    // If already initialized, just return success
    if is_initialized() {
        return Ok(());
    }

    // Get path to runtime
    let runtime_path = config.base_path.join("runtime");
    if !runtime_path.exists() {
        return Err(errors::GrobidError::Configuration(format!(
            "Runtime directory not found: {}",
            runtime_path.display()
        )));
    }

    // Get path to grobid
    let grobid_path = config.base_path.join("grobid");
    if !grobid_path.exists() {
        return Err(errors::GrobidError::Configuration(format!(
            "Grobid directory not found: {}",
            grobid_path.display()
        )));
    }

    // Prepare JVM arguments
    let mut jvm_args = Vec::new();

    // Set classpath
    let grobid_home = grobid_path.join("grobid-home");
    let grobid_core = grobid_path.join("grobid-core");
    let grobid_core_jar = grobid_core.join("build/libs/grobid-core-0.8.2-onejar.jar");

    if !grobid_core_jar.exists() {
        return Err(errors::GrobidError::Configuration(format!(
            "Grobid JAR not found: {}",
            grobid_core_jar.display()
        )));
    }

    let classpath = format!("-Djava.class.path={}", grobid_core_jar.display());
    jvm_args.push(classpath);

    // Set system properties
    jvm_args.push(format!("-Dgrobid.home={}", grobid_home.display()));

    // Add custom system properties
    for (key, value) in &config.system_properties {
        jvm_args.push(format!("-D{}={}", key, value));
    }

    // Set memory limits
    jvm_args.push(format!("-Xmx{}", config.max_memory));

    // Add other JVM options
    for option in &config.jvm_options {
        jvm_args.push(option.clone());
    }

    // Create JVM arguments for jni crate
    let options_str = jvm_args.join(" ");
    let jni_args = jni::InitArgsBuilder::new()
        .version(jni::JNIVersion::V8)
        .option(&options_str)
        .build()
        .map_err(|e| {
            errors::GrobidError::JvmInitialization(format!("Failed to build JVM args: {}", e))
        })?;

    // Create the JVM
    let jvm = jni::JavaVM::new(jni_args).map_err(|e| {
        errors::GrobidError::JvmInitialization(format!("Failed to create JVM: {}", e))
    })?;

    // Store JVM in global
    if JVM.set(jvm).is_err() {
        return Err(errors::GrobidError::JvmInitialization(
            "Failed to store JVM in global state".to_string(),
        ));
    }

    // Get JNI environment
    let jvm = JVM.get().unwrap(); // Safe because we just set it
    let mut env = jvm
        .attach_current_thread()
        .map_err(errors::GrobidError::Jni)?;

    // Initialize Grobid engine
    let engine_class = env
        .find_class("org/grobid/core/engines/Engine")
        .map_err(errors::GrobidError::Jni)?;

    // Get singleton instance
    let engine_obj = env
        .call_static_method(
            engine_class,
            "getEngine",
            "()Lorg/grobid/core/engines/Engine;",
            &[],
        )
        .map_err(errors::GrobidError::Jni)?
        .l()
        .map_err(errors::GrobidError::Jni)?;

    // Convert to global reference
    let engine_global = env
        .new_global_ref(engine_obj)
        .map_err(errors::GrobidError::Jni)?;

    // Store in global
    if ENGINE.set(Mutex::new(engine_global)).is_err() {
        return Err(errors::GrobidError::JvmInitialization(
            "Failed to store engine reference in global state".to_string(),
        ));
    }

    // Mark as initialized
    INITIALIZED.store(true, Ordering::SeqCst);

    Ok(())
}

/// Boot JVM + Grobid with the provided configuration.
///
/// This is a compatibility alias for `init()`.
///
/// # Deprecated
/// This function is deprecated in favor of `init`. It will be removed in a future version.
#[deprecated(since = "0.1.0", note = "Use init instead")]
pub fn init_with_config(config: &config::GrobidConfig) -> Result<(), errors::GrobidError> {
    init(config)
}

/// Shuts down the JVM and frees resources.
///
/// This function should be called when the application is shutting down to properly
/// clean up JVM resources. It's not strictly necessary as the JVM will be shut down
/// when the process exits, but it's good practice to call it explicitly.
///
/// # Examples
///
/// ```no_run
/// use grobid_rs::{GrobidConfig, init, shutdown};
///
/// // Initialize
/// let config = GrobidConfig::default();
/// init(&config).expect("Failed to initialize Grobid");
///
/// // Use Grobid...
///
/// // Shutdown when done
/// shutdown().expect("Failed to shut down Grobid");
/// ```
pub fn shutdown() -> Result<(), errors::GrobidError> {
    // Nothing to do if not initialized
    if !is_initialized() {
        return Ok(());
    }

    // First mark as not initialized to prevent new operations
    INITIALIZED.store(false, Ordering::SeqCst);

    // Clear the engine reference first
    if let Some(engine) = ENGINE.get() {
        // Try to acquire the lock - if we can't, something has gone wrong
        if let Ok(engine_ref) = engine.try_lock() {
            // Release any JNI resources by dropping the reference
            // Note: we can't actually reset the OnceCell, but we can mark it as uninitialized
            drop(engine_ref);
        }
    }

    // For now, we don't attempt to destroy the JVM, as this can cause crashes
    // if there are still outstanding JNI references or attached threads.
    // Instead, we just mark it as uninitialized and allow it to be garbage collected
    // when the process terminates.

    // In a real implementation with proper JVM shutdown, we would:
    // 1. Ensure all threads are detached from the JVM
    // 2. Call JVM.destroy() if available
    // 3. Reset our references

    // For now we just mark as uninitialized
    Ok(())
}

/// Returns true if the JVM and Grobid engine have been initialized.
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst) && JVM.get().is_some() && ENGINE.get().is_some()
}

// Re-export cache management functions
pub use cache::{
    cache_exists, fulltext_to_tei_cached, get_cache_dir, get_cache_path, get_cache_stats,
    get_cached_path, process_header_cached, process_references_cached, process_with_cache,
    read_cache, reset_cache_stats, write_cache, CacheConfig, CacheStats, OutputType,
};

// Import the cache_prune module
mod cache_prune;

// Re-export cache pruning functions
pub use cache_prune::{
    background_gc_task, check_and_prune_if_needed, clear_cache, get_cache_size, get_cache_summary,
    get_human_readable_cache_size, list_cache_files, prune_cache, start_background_gc,
};

// Helper function for ensuring cache directory exists
pub fn ensure_cache_dir() -> Result<(), errors::GrobidError> {
    let _dir = get_cache_dir()?;
    Ok(())
}

// Include tests module when testing
#[cfg(test)]
mod tests;
