use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, warn};
use crate::GrobidError;
use crate::cache::{get_cache_dir, get_cache_stats};

/// The default maximum cache size (10 GB)
pub const DEFAULT_MAX_CACHE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Environment variable for setting maximum cache size in bytes
pub const CACHE_MAX_SIZE_ENV: &str = "GROBID_RS_CACHE_MAX_SIZE";

/// Environment variable for enabling automatic cache pruning
pub const CACHE_AUTO_PRUNE_ENV: &str = "GROBID_RS_CACHE_AUTO_PRUNE";

/// Default check interval for background GC (1 hour)
pub const DEFAULT_GC_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Cache file info used for pruning decisions
struct CacheFileInfo {
    path: PathBuf,
    size: u64,
    last_accessed: SystemTime,
}

/// Gets the configured maximum cache size in bytes
pub fn get_max_cache_size() -> u64 {
    if let Ok(size_str) = std::env::var(CACHE_MAX_SIZE_ENV) {
        if let Ok(size) = size_str.parse::<u64>() {
            debug!("Using configured max cache size: {} bytes", size);
            return size;
        }
        warn!("Invalid {} value: '{}', using default", CACHE_MAX_SIZE_ENV, size_str);
    }
    DEFAULT_MAX_CACHE_SIZE
}

/// Check if auto-pruning is enabled
pub fn is_auto_prune_enabled() -> bool {
    match std::env::var(CACHE_AUTO_PRUNE_ENV) {
        Ok(val) => {
            match val.to_lowercase().as_str() {
                "true" | "1" | "yes" | "y" | "on" => true,
                _ => false,
            }
        },
        Err(_) => true, // Default to true if env var is not set
    }
}

/// Calculate the current size of the cache directory in bytes
pub fn get_cache_size() -> Result<u64, GrobidError> {
    let cache_dir = get_cache_dir()?;
    let mut total_size = 0;
    
    for entry in fs::read_dir(&cache_dir)
        .map_err(|e| GrobidError::Io(e))?
    {
        let entry = entry.map_err(|e| GrobidError::Io(e))?;
        let metadata = entry.metadata().map_err(|e| GrobidError::Io(e))?;
        if metadata.is_file() {
            total_size += metadata.len();
        }
    }
    
    Ok(total_size)
}

/// Get cache size as a human-readable string
pub fn get_human_readable_cache_size() -> Result<String, GrobidError> {
    let size_bytes = get_cache_size()?;
    
    if size_bytes < 1024 {
        return Ok(format!("{} B", size_bytes));
    } else if size_bytes < 1024 * 1024 {
        return Ok(format!("{:.2} KB", size_bytes as f64 / 1024.0));
    } else if size_bytes < 1024 * 1024 * 1024 {
        return Ok(format!("{:.2} MB", size_bytes as f64 / (1024.0 * 1024.0)));
    } else {
        return Ok(format!("{:.2} GB", size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)));
    }
}

/// Format bytes as a human-readable string
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// List all cache files
pub fn list_cache_files() -> Result<Vec<PathBuf>, GrobidError> {
    let cache_dir = get_cache_dir()?;
    debug!("Listing cache files in: {}", cache_dir.display());
    
    // First check if the directory exists
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)
            .map_err(|e| {
                warn!("Failed to create cache directory {}: {}", cache_dir.display(), e);
                GrobidError::Io(e)
            })?;
        debug!("Created cache directory: {}", cache_dir.display());
        return Ok(Vec::new());
    }
    
    let mut files = Vec::new();
    
    for entry in fs::read_dir(&cache_dir)
        .map_err(|e| {
            warn!("Failed to read cache directory {}: {}", cache_dir.display(), e);
            GrobidError::Io(e)
        })?
    {
        let entry = entry.map_err(|e| GrobidError::Io(e))?;
        let metadata = entry.metadata().map_err(|e| GrobidError::Io(e))?;
        if metadata.is_file() {
            debug!("Found cache file: {}", entry.path().display());
            files.push(entry.path());
        }
    }
    
    debug!("Found {} files in cache directory", files.len());
    Ok(files)
}

/// Get a summary of cache information
pub fn get_cache_summary() -> Result<String, GrobidError> {
    let stats = get_cache_stats();
    let total_size = get_cache_size()?;
    let max_size = get_max_cache_size();
    let files = list_cache_files()?;
    
    let mut summary = String::new();
    summary.push_str(&format!("Cache directory: {}\n", get_cache_dir()?.display()));
    summary.push_str(&format!("Total cache size: {} / {} ({:.1}%)\n", 
        format_bytes(total_size), 
        format_bytes(max_size),
        (total_size as f64 / max_size as f64) * 100.0));
    summary.push_str(&format!("Files: {}\n", files.len()));
    summary.push_str(&format!("Auto-pruning: {}\n", if is_auto_prune_enabled() { "enabled" } else { "disabled" }));
    summary.push_str("\nCache Statistics (current session):\n");
    summary.push_str(&format!("  Hits: {}\n", stats.hits));
    summary.push_str(&format!("  Misses: {}\n", stats.misses));
    summary.push_str(&format!("  Hit rate: {:.1}%\n", 
        if stats.hits + stats.misses > 0 { 
            (stats.hits as f64 / (stats.hits + stats.misses) as f64) * 100.0 
        } else { 
            0.0 
        }));
    summary.push_str(&format!("  Bytes read: {}\n", format_bytes(stats.bytes_read as u64)));
    summary.push_str(&format!("  Bytes written: {}\n", format_bytes(stats.bytes_written as u64)));
    summary.push_str(&format!("  Estimated time saved: {:.2} seconds\n", stats.time_saved_ms as f64 / 1000.0));
    
    Ok(summary)
}

/// Get a list of cache files sorted by access time (oldest first)
fn get_cache_files_by_age() -> Result<Vec<CacheFileInfo>, GrobidError> {
    let cache_dir = get_cache_dir()?;
    debug!("Finding cache files in: {}", cache_dir.display());
    let mut files = Vec::new();
    
    for entry in fs::read_dir(&cache_dir)
        .map_err(|e| { 
            warn!("Failed to read cache directory {}: {}", cache_dir.display(), e);
            GrobidError::Io(e)
        })?
    {
        let entry = entry.map_err(|e| GrobidError::Io(e))?;
        let path = entry.path();
        debug!("Examining cache entry: {}", path.display());
        
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(e) => {
                warn!("Failed to get metadata for {}: {}", path.display(), e);
                continue; // Skip this file if we can't get metadata
            }
        };
        
        if metadata.is_file() {
            // Get last access time, fallback to modified time if not available
            let last_accessed = metadata.accessed().unwrap_or_else(|_| {
                debug!("Could not get access time for {}, falling back to modified time", path.display());
                metadata.modified().unwrap_or_else(|_| {
                    debug!("Could not get modified time for {}, using current time", path.display());
                    SystemTime::now()
                })
            });
            
            let file_size = metadata.len();
            debug!("Adding cache file: {} (size: {}, last accessed: {:?})",
                path.display(), format_bytes(file_size), last_accessed);
                
            files.push(CacheFileInfo {
                path,
                size: file_size,
                last_accessed,
            });
        } else {
            debug!("Skipping non-file entry: {}", path.display());
        }
    }
    
    debug!("Found {} cache files to consider for pruning", files.len());
    
    // Sort by access time (oldest first)
    files.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));
    
    if !files.is_empty() {
        debug!("Oldest file: {} (accessed: {:?})", 
            files.first().unwrap().path.display(),
            files.first().unwrap().last_accessed);
        debug!("Newest file: {} (accessed: {:?})", 
            files.last().unwrap().path.display(),
            files.last().unwrap().last_accessed);
    }
    // Sort by access time (oldest first)
files.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));
    
Ok(files)
}

/// Prune the cache to stay under the specified size limit
/// 
/// Returns a tuple with the number of files removed and the number of bytes removed
pub fn prune_cache(max_size_bytes: u64) -> Result<(usize, u64), GrobidError> {
// Fresh lookup of cache directory each time to respect environment changes
let cache_dir = get_cache_dir()?;
debug!("Pruning cache directory: {}", cache_dir.display());
    
// List files in the directory to confirm it exists and has contents
match fs::read_dir(&cache_dir) {
    Ok(entries) => {
        let count = entries.count();
        debug!("Found {} entries in cache directory", count);
    },
    Err(e) => {
        warn!("Failed to read cache directory {}: {}", cache_dir.display(), e);
    }
}
    
let current_size = get_cache_size()?;
    
    // If we're already under the limit, nothing to do
    if current_size <= max_size_bytes {
        info!("Cache is already under the size limit ({} <= {})", 
            format_bytes(current_size), format_bytes(max_size_bytes));
        return Ok((0, 0));
    }
    
    info!("Pruning cache: current size {} exceeds limit {} (needs to remove at least {} bytes)", 
        format_bytes(current_size), format_bytes(max_size_bytes), 
        format_bytes(current_size.saturating_sub(max_size_bytes)));
    
    // Get list of files sorted by access time
    let files = get_cache_files_by_age()?;
    
    // Target size is slightly below the max to provide a buffer
    let target_size = (max_size_bytes as f64 * 0.9) as u64;
    let bytes_to_remove = current_size.saturating_sub(target_size);
    
    let mut bytes_removed = 0;
    let mut files_removed = 0;
    
    // Force removal of at least one file if we're over the limit
    // This prevents the function from returning without removing anything
    let mut removed_at_least_one = false;
    
    // Remove files until we're under the target size
    for file in files {
        // Always remove at least one file if we're over the limit
        if current_size - bytes_removed <= target_size && removed_at_least_one {
            break;
        }
        
        // Try to remove the file
        match fs::remove_file(&file.path) {
            Ok(_) => {
                debug!("Removed cache file: {} ({})", 
                    file.path.display(), format_bytes(file.size));
                
                bytes_removed += file.size;
                files_removed += 1;
                removed_at_least_one = true;
            },
            Err(e) => {
                warn!("Failed to remove cache file {}: {}", file.path.display(), e);
            }
        }
    }
    
    // If we didn't remove any files but need to, let's force removal of all files
    if files_removed == 0 && bytes_to_remove > 0 {
        info!("No files removed during normal pruning. Attempting to force-remove files.");
        let all_files = list_cache_files()?;
        
        if all_files.is_empty() {
            info!("No cache files found to force-remove. Cache directory may be empty.");
        } else {
            info!("Found {} files to consider for force removal", all_files.len());
        }
        
        for path in all_files {
            info!("Trying to force-remove: {}", path.display());
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.is_file() {
                    let size = metadata.len();
                    match fs::remove_file(&path) {
                        Ok(_) => {
                            info!("Force removed cache file: {} ({})", 
                                path.display(), format_bytes(size));
                            bytes_removed += size;
                            files_removed += 1;
                        },
                        Err(e) => {
                            warn!("Failed to force-remove cache file {}: {}", path.display(), e);
                        }
                    }
                } else {
                    debug!("Skipping non-file entry: {}", path.display());
                }
            } else {
                warn!("Failed to get metadata for file to force-remove: {}", path.display());
            }
        }
    }
    
    let new_size = current_size - bytes_removed;
    info!("Cache pruning complete: removed {} files ({}) - new size: {}", 
        files_removed, format_bytes(bytes_removed), format_bytes(new_size));
        
    Ok((files_removed, bytes_removed))
}

/// Remove all files from the cache directory
pub fn clear_cache() -> Result<(usize, u64), GrobidError> {
    let files = list_cache_files()?;
    let mut files_removed = 0;
    let mut bytes_removed = 0;
    
    for path in files {
        // Get file size before removing
        let size = match fs::metadata(&path) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };
        
        match fs::remove_file(&path) {
            Ok(_) => {
                debug!("Removed cache file: {} ({})", path.display(), format_bytes(size));
                files_removed += 1;
                bytes_removed += size;
            },
            Err(e) => {
                warn!("Failed to remove cache file {}: {}", path.display(), e);
            }
        }
    }
    
    info!("Cache cleared: removed {} files ({})", files_removed, format_bytes(bytes_removed));
    Ok((files_removed, bytes_removed))
}

/// Background garbage collection task
/// 
/// This function is meant to be called from a separate thread and will run
/// indefinitely, checking and pruning the cache at regular intervals.
pub fn background_gc_task(interval: Duration) {
    loop {
        // Sleep before first check to avoid immediate pruning on startup
        std::thread::sleep(interval);
        
        if !is_auto_prune_enabled() {
            debug!("Auto-pruning is disabled, skipping background GC cycle");
            continue;
        }
        
        debug!("Running background cache garbage collection");
        
        match prune_cache(get_max_cache_size()) {
            Ok((files_removed, bytes_removed)) => {
                if files_removed > 0 {
                    debug!("Background GC removed {} files ({})", 
                        files_removed, format_bytes(bytes_removed));
                } else {
                    debug!("Background GC: cache is within size limits");
                }
            },
            Err(e) => {
                error!("Background cache GC failed: {}", e);
            }
        }
    }
}

/// Start a background garbage collection thread
pub fn start_background_gc() {
    let interval = DEFAULT_GC_INTERVAL;
    
    // Spawn a thread that never returns
    if let Err(e) = std::thread::Builder::new()
        .name("grobid-cache-gc".to_string())
        .spawn(move || {
            debug!("Started background cache GC thread (interval: {:?})", interval);
            background_gc_task(interval);
        }) {
            error!("Failed to spawn background GC thread: {}", e);
        }
}

/// Check if the cache needs pruning and prune if necessary
/// 
/// This is a convenience function that can be called periodically to
/// ensure the cache stays under the size limit. It checks the current
/// size and prunes if needed.
pub fn check_and_prune_if_needed() -> Result<(), GrobidError> {
    if !is_auto_prune_enabled() {
        return Ok(());
    }
    
    let current_size = get_cache_size()?;
    let max_size = get_max_cache_size();
    
    if current_size > max_size {
        debug!("Cache size {} exceeds limit {}, pruning", 
            format_bytes(current_size), format_bytes(max_size));
        prune_cache(max_size)?;
    }
    
    Ok(())
}