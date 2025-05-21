use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use crate::GrobidError;

/// Environment variable for controlling cache directory location
pub const CACHE_DIR_ENV: &str = "GROBID_RS_CACHE_DIR";

/// Default cache directory name
pub const DEFAULT_CACHE_DIR_NAME: &str = ".grobid-rs-cache";

/// Cache configuration options
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Skip processing if valid cache file exists
    pub skip_existing: bool,
    /// Force reprocessing even if cache exists
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

/// Output format type for cache file naming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// TEI XML format
    Tei,
    /// JSON format
    Json,
    /// BibTeX format
    Bibtex,
}

impl OutputType {
    /// Convert output type to file extension
    pub fn to_extension(&self) -> &'static str {
        match self {
            Self::Tei => "tei.xml",
            Self::Json => "json",
            Self::Bibtex => "bib",
        }
    }
}

/// Cache operation statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: usize,
    /// Number of cache misses
    pub misses: usize,
    /// Bytes read from cache
    pub bytes_read: usize,
    /// Bytes written to cache
    pub bytes_written: usize,
    /// Estimated time saved in milliseconds
    pub time_saved_ms: u64,
}

// Global stats counters
static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static BYTES_READ: AtomicU64 = AtomicU64::new(0);
static BYTES_WRITTEN: AtomicU64 = AtomicU64::new(0);
static TIME_SAVED_MS: AtomicU64 = AtomicU64::new(0);

/// Get the current cache statistics
pub fn get_cache_stats() -> CacheStats {
    CacheStats {
        hits: CACHE_HITS.load(Ordering::Relaxed) as usize,
        misses: CACHE_MISSES.load(Ordering::Relaxed) as usize,
        bytes_read: BYTES_READ.load(Ordering::Relaxed) as usize,
        bytes_written: BYTES_WRITTEN.load(Ordering::Relaxed) as usize,
        time_saved_ms: TIME_SAVED_MS.load(Ordering::Relaxed),
    }
}

/// Record a cache hit
fn record_hit(bytes: usize, time_saved_ms: u64) {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed);
    BYTES_READ.fetch_add(bytes as u64, Ordering::Relaxed);
    TIME_SAVED_MS.fetch_add(time_saved_ms, Ordering::Relaxed);
}

/// Record a cache miss
fn record_miss(bytes_written: usize) {
    CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
    BYTES_WRITTEN.fetch_add(bytes_written as u64, Ordering::Relaxed);
}

/// Get the cache directory path
pub fn get_cache_dir() -> Result<PathBuf, GrobidError> {
    // First check if cache directory is set via environment variable
    if let Ok(dir) = env::var(CACHE_DIR_ENV) {
        let path = PathBuf::from(dir);
        return Ok(path);
    }
    
    // Otherwise use default location in user's home directory
    if let Some(home_dir) = dirs::home_dir() {
        return Ok(home_dir.join(DEFAULT_CACHE_DIR_NAME));
    }
    
    // If home directory can't be determined, fall back to system temp directory
    if let Some(temp_dir) = env::temp_dir().to_str() {
        return Ok(PathBuf::from(temp_dir).join(DEFAULT_CACHE_DIR_NAME));
    }
    
    Err(GrobidError::Cache("Failed to determine cache directory location".to_string()))
}

/// Ensure the cache directory exists
pub fn ensure_cache_dir() -> Result<(), GrobidError> {
    let dir = get_cache_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| GrobidError::Io(e))?;
    }
    Ok(())
}

/// Generate a unique cache key for a PDF
fn generate_cache_key(pdf_path: &Path) -> Result<String, GrobidError> {
    // Get file metadata for uniqueness
    let metadata = fs::metadata(pdf_path)
        .map_err(|e| GrobidError::Io(e))?;
    
    // Generate a cache key based on:
    // 1. Absolute path (normalized)
    // 2. File size
    // 3. Last modified time
    let canonical_path = pdf_path.canonicalize()
        .map_err(|e| GrobidError::Io(e))?;
    
    let file_size = metadata.len();
    
    // Get modified time or use current time if unavailable
    let modified = metadata.modified()
        .unwrap_or_else(|_| SystemTime::now());
    
    // Convert modified time to seconds since epoch
    let modified_secs = match modified.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    };
    
    // Create a key combining these elements
    let key = format!("{}_{}_{}",
        canonical_path.to_string_lossy(),
        file_size,
        modified_secs
    );
    
    // Hash the key to create a fixed-length identifier
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    
    Ok(format!("{:016x}", hash))
}

/// Get the cache file path for a PDF and output type
pub fn get_cache_path(pdf_path: &Path, output_type: OutputType) -> Result<PathBuf, GrobidError> {
    let cache_dir = get_cache_dir()?;
    let cache_key = generate_cache_key(pdf_path)?;
    let extension = output_type.to_extension();
    
    Ok(cache_dir.join(format!("{}.{}", cache_key, extension)))
}

/// Process a PDF using the cache
/// 
/// This function takes a PDF path and a processing function. It will:
/// 1. Check if a valid cache entry exists (if enabled and not forced to reprocess)
/// 2. Return the cached result if found
/// 3. Otherwise, call the processing function and cache the result
pub fn process_with_cache<F>(
    pdf_path: &Path, 
    output_type: OutputType,
    config: CacheConfig,
    process_fn: F
) -> Result<String, GrobidError> 
where
    F: FnOnce() -> Result<String, GrobidError>
{
    // If cache is disabled, just call the processor function
    if !config.enabled {
        return process_fn();
    }
    
    ensure_cache_dir()?;
    let cache_path = get_cache_path(pdf_path, output_type)?;
    
    // Check if we can use the cache
    if !config.force_reprocess && config.skip_existing && cache_path.exists() {
        // Read from cache
        let start_time = SystemTime::now();
        let content = fs::read_to_string(&cache_path)
            .map_err(|e| GrobidError::Io(e))?;
        
        // Calculate time saved
        let elapsed = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_else(|_| std::time::Duration::from_millis(0));
        
        // Estimate time saved (assume processing takes at least 500ms)
        let time_saved = std::time::Duration::from_millis(500)
            .checked_sub(elapsed)
            .unwrap_or_else(|| std::time::Duration::from_millis(0));
        
        // Record cache hit
        record_hit(content.len(), time_saved.as_millis() as u64);
        
        return Ok(content);
    }
    
    // Process the file
    let result = process_fn()?;
    
    // Write to cache
    if let Err(e) = fs::write(&cache_path, &result) {
        // Non-fatal error, just log and continue
        eprintln!("Failed to write to cache file {}: {}", cache_path.display(), e);
    } else {
        // Record cache miss with successful write
        record_miss(result.len());
    }
    
    Ok(result)
}