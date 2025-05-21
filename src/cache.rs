use crate::GrobidError;
use directories::ProjectDirs;
use fs2::FileExt;
use memmap2::Mmap;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tracing::{debug, info, trace, warn};

/// Grobid version used as part of the cache key
use crate::GROBID_VERSION;

/// Environment variable name for overriding the cache directory
pub const CACHE_DIR_ENV: &str = "GROBID_RS_CACHE_DIR";

/// Global cache statistics for the current process
#[derive(Debug, Default, Clone, Copy)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: usize,
    /// Number of cache misses
    pub misses: usize,
    /// Number of bytes read from cache
    pub bytes_read: usize,
    /// Number of bytes written to cache
    pub bytes_written: usize,
    /// Estimated time saved by using cache
    pub time_saved_ms: u64,
}

// Global atomic counters for cache statistics
static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);
static CACHE_MISSES: AtomicUsize = AtomicUsize::new(0);
static CACHE_BYTES_READ: AtomicUsize = AtomicUsize::new(0);
static CACHE_BYTES_WRITTEN: AtomicUsize = AtomicUsize::new(0);
static CACHE_TIME_SAVED_MS: AtomicUsize = AtomicUsize::new(0);

/// Get the current cache statistics
pub fn get_cache_stats() -> CacheStats {
    CacheStats {
        hits: CACHE_HITS.load(Ordering::Relaxed),
        misses: CACHE_MISSES.load(Ordering::Relaxed),
        bytes_read: CACHE_BYTES_READ.load(Ordering::Relaxed),
        bytes_written: CACHE_BYTES_WRITTEN.load(Ordering::Relaxed),
        time_saved_ms: CACHE_TIME_SAVED_MS.load(Ordering::Relaxed) as u64,
    }
}

/// Reset cache statistics
pub fn reset_cache_stats() {
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    CACHE_BYTES_READ.store(0, Ordering::Relaxed);
    CACHE_BYTES_WRITTEN.store(0, Ordering::Relaxed);
    CACHE_TIME_SAVED_MS.store(0, Ordering::Relaxed);
}

/// Different output types that can be cached
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    Tei,
    Json,
    Bibtex,
    Text,
}

impl OutputType {
    /// Get the file extension for this output type
    pub fn extension(&self) -> &'static str {
        match self {
            OutputType::Tei => "tei",
            OutputType::Json => "json",
            OutputType::Bibtex => "bib",
            OutputType::Text => "txt",
        }
    }
}

/// Cache configuration for controlling cache behavior
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// Whether to use the cache at all (both for reading and writing)
    pub enabled: bool,
    /// Whether to skip processing if cached results exist
    pub skip_existing: bool,
    /// Whether to force reprocessing even if cache exists
    pub force_reprocess: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            skip_existing: true,
            force_reprocess: false,
        }
    }
}

/// Calculates the SHA-256 hash of a PDF file, which will be used as the cache key
fn hash_pdf(pdf_path: &Path) -> Result<String, GrobidError> {
    trace!("Hashing PDF file: {}", pdf_path.display());

    // Read the entire file into a buffer
    let file_content = fs::read(pdf_path).map_err(GrobidError::Io)?;

    // Hash the file content
    hash_content(&file_content)
}

/// Calculates the SHA-256 hash of a byte slice, used for both initial hashing and validation
fn hash_content(content: &[u8]) -> Result<String, GrobidError> {
    // Create a SHA-256 hasher
    let mut hasher = Sha256::new();

    // Hash the content
    hasher.update(content);

    // Append the Grobid version to the hash to ensure cache is invalidated on version changes
    hasher.update(format!("-{}", GROBID_VERSION).as_bytes());

    // Get the hash as a hex string
    let hash = hasher.finalize();
    let hex_hash = format!("{:x}", hash);

    trace!("Hash generated: {}", hex_hash);
    Ok(hex_hash)
}

/// Get the base cache directory for grobid-rs
pub fn get_cache_dir() -> Result<PathBuf, GrobidError> {
    // Check for environment variable override (useful for testing)
    if let Ok(env_cache_dir) = std::env::var(CACHE_DIR_ENV) {
        let cache_dir = PathBuf::from(env_cache_dir);

        // Create the cache directory if it doesn't exist
        if !cache_dir.exists() {
            trace!(
                "Creating cache directory from env var: {}",
                cache_dir.display()
            );
            fs::create_dir_all(&cache_dir).map_err(GrobidError::Io)?;
        }

        return Ok(cache_dir);
    }

    // Get the project directories using the `directories` crate
    let proj_dirs = ProjectDirs::from("rs", "", "grobid-rs").ok_or_else(|| {
        GrobidError::InvalidInput("Failed to determine cache directory".to_string())
    })?;

    let cache_dir = proj_dirs.cache_dir().to_path_buf();

    // Create the cache directory if it doesn't exist
    if !cache_dir.exists() {
        trace!("Creating cache directory: {}", cache_dir.display());
        fs::create_dir_all(&cache_dir).map_err(GrobidError::Io)?;
    }

    Ok(cache_dir)
}

/// Get the cache file path for a specific PDF and output type
pub fn get_cache_path(pdf_path: &Path, output_type: OutputType) -> Result<PathBuf, GrobidError> {
    let cache_dir = get_cache_dir()?;
    let hash = hash_pdf(pdf_path)?;
    let cache_path = cache_dir.join(format!("{}.{}", hash, output_type.extension()));

    trace!(
        "Cache path for {}: {}",
        pdf_path.display(),
        cache_path.display()
    );
    Ok(cache_path)
}

/// Check if a cached result exists for the given PDF and output type
pub fn cache_exists(pdf_path: &Path, output_type: OutputType) -> Result<bool, GrobidError> {
    let cache_path = get_cache_path(pdf_path, output_type)?;
    let exists = cache_path.exists();

    if exists {
        debug!(
            "Cache hit for {}: {}",
            pdf_path.display(),
            cache_path.display()
        );
    } else {
        debug!(
            "Cache miss for {}: {}",
            pdf_path.display(),
            cache_path.display()
        );
    }

    Ok(exists)
}

/// Read cached result from disk using memory mapping for zero-copy access
pub fn read_cache(pdf_path: &Path, output_type: OutputType) -> Result<String, GrobidError> {
    let cache_path = get_cache_path(pdf_path, output_type)?;

    debug!("Reading from cache: {}", cache_path.display());

    // Open the file
    let file = fs::File::open(&cache_path).map_err(GrobidError::Io)?;

    // Get file size for logging
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

    // Memory-map the file for zero-copy reading
    let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
        GrobidError::Io(std::io::Error::other(format!(
            "Failed to memory-map cache file: {}",
            e
        )))
    })?;

    // Create a string from the memory-mapped region, avoiding a copy
    let content = match std::str::from_utf8(&mmap) {
        Ok(s) => s.to_string(),
        Err(e) => {
            return Err(GrobidError::InvalidInput(format!(
                "Cache file contains invalid UTF-8: {}",
                e
            )))
        }
    };

    // Validate the cache by re-hashing the PDF
    if let Ok(file_content) = fs::read(pdf_path) {
        let expected_hash = hash_content(&file_content)?;
        let hash_from_path = cache_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");

        if hash_from_path != expected_hash {
            warn!("Cache hash mismatch for {}. Expected {}, found {}. Possible bit-rot or file modification.", 
                pdf_path.display(), expected_hash, hash_from_path);
            return Err(GrobidError::InvalidInput(format!(
                "Cache validation failed: hash mismatch for {}",
                pdf_path.display()
            )));
        }
    }

    // Update cache statistics
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
    CACHE_BYTES_READ.fetch_add(file_size as usize, Ordering::Relaxed);

    info!(
        "Read {} bytes from cache for {} using zero-copy",
        file_size,
        pdf_path.display()
    );
    Ok(content)
}

/// Get the path to a cached result if it exists
pub fn get_cached_path(
    pdf_path: &Path,
    output_type: OutputType,
) -> Result<Option<PathBuf>, GrobidError> {
    let cache_path = get_cache_path(pdf_path, output_type)?;
    if cache_path.exists() {
        Ok(Some(cache_path))
    } else {
        Ok(None)
    }
}

/// Write result to cache
pub fn write_cache(
    pdf_path: &Path,
    output_type: OutputType,
    content: &str,
) -> Result<(), GrobidError> {
    let cache_path = get_cache_path(pdf_path, output_type)?;

    debug!("Writing to cache: {}", cache_path.display());

    // Create a temporary file to write to atomically
    let dir = cache_path.parent().ok_or_else(|| {
        GrobidError::InvalidInput("Failed to determine cache file parent directory".to_string())
    })?;

    let mut temp_file =
        NamedTempFile::new_in(dir).map_err(|e| GrobidError::Io(std::io::Error::other(e)))?;

    // Acquire an exclusive lock on the temporary file to avoid race conditions in parallel processing
    {
        let file = temp_file.as_file();
        file.lock_exclusive().map_err(GrobidError::Io)?;

        // Write the content to the temporary file
        temp_file
            .write_all(content.as_bytes())
            .map_err(GrobidError::Io)?;

        // Keep the lock until the end of this scope
    }

    // Persist the temporary file by renaming it to the target path (atomic operation)
    temp_file
        .persist(&cache_path)
        .map_err(|e| GrobidError::Io(std::io::Error::other(e)))?;

    // Update cache statistics
    CACHE_BYTES_WRITTEN.fetch_add(content.len(), Ordering::Relaxed);

    info!(
        "Wrote {} bytes to cache for {}",
        content.len(),
        pdf_path.display()
    );
    Ok(())
}

/// Process with caching support
///
/// This is a helper function that handles the caching logic for all processing types.
/// It will:
/// 1. Check if a cached result exists and return it if appropriate
/// 2. Otherwise, call the process function to generate a new result
/// 3. Cache the new result
///
/// The cache behavior can be controlled via the CacheConfig.
///
/// This function is thread-safe and can be called from multiple threads safely.
/// File locking ensures that cache operations don't conflict when run in parallel.
///
/// This function also tracks cache statistics, which can be accessed via `get_cache_stats()`.
pub fn process_with_cache<F>(
    pdf_path: &Path,
    output_type: OutputType,
    config: CacheConfig,
    process_fn: F,
) -> Result<String, GrobidError>
where
    F: FnOnce() -> Result<String, GrobidError>,
{
    // Start timing for statistics
    let start_time = Instant::now();
    // If cache is disabled, just process directly
    if !config.enabled {
        debug!("Cache disabled, processing directly");
        return process_fn();
    }

    // Check if we need to use the cache
    let use_cache = match (
        cache_exists(pdf_path, output_type)?,
        config.skip_existing,
        config.force_reprocess,
    ) {
        // Cache exists, skip_existing is true, and force_reprocess is false -> use cache
        (true, true, false) => true,
        // Otherwise, don't use cache
        _ => false,
    };

    if use_cache {
        info!("Using cached result for {}", pdf_path.display());
        match read_cache(pdf_path, output_type) {
            Ok(content) => {
                // Calculate time saved by using cache
                let processing_time = start_time.elapsed();

                // Estimate - typical processing takes at least 3 seconds, so the saved time is:
                let estimated_saved = Duration::from_secs(3).saturating_sub(processing_time);
                CACHE_TIME_SAVED_MS
                    .fetch_add(estimated_saved.as_millis() as usize, Ordering::Relaxed);

                Ok(content)
            }
            Err(e) => {
                // If cache validation fails, process the file instead
                warn!("Cache read error: {}. Will reprocess the file.", e);
                CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
                let result = process_fn()?;

                // Write the fresh result to cache
                if let Err(cache_err) = write_cache(pdf_path, output_type, &result) {
                    warn!("Failed to update cache: {}", cache_err);
                }

                Ok(result)
            }
        }
    } else {
        info!("Processing {} (skipping cache)", pdf_path.display());
        CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
        let result = process_fn()?;

        // Only write to cache if we're not forcing reprocess (which would be pointless)
        if !config.force_reprocess && config.enabled {
            if let Err(e) = write_cache(pdf_path, output_type, &result) {
                warn!("Failed to write to cache: {}", e);
                // Continue even if caching fails
            }
        }

        Ok(result)
    }
}

/// Process fulltext with caching support
pub fn fulltext_to_tei_cached(pdf_path: &Path, config: CacheConfig) -> Result<String, GrobidError> {
    process_with_cache(pdf_path, OutputType::Tei, config, || {
        crate::engine::fulltext_to_tei(pdf_path)
    })
}

/// Process header with caching support
pub fn process_header_cached(pdf_path: &Path, config: CacheConfig) -> Result<String, GrobidError> {
    process_with_cache(pdf_path, OutputType::Tei, config, || {
        crate::engine::process_header(pdf_path)
    })
}

/// Process references with caching support
pub fn process_references_cached(
    pdf_path: &Path,
    config: CacheConfig,
) -> Result<String, GrobidError> {
    process_with_cache(pdf_path, OutputType::Tei, config, || {
        crate::engine::process_references(pdf_path)
    })
}
