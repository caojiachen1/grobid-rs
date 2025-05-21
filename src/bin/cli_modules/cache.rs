use crate::cli_modules::types::{CliExitCode, CacheCommands};
use grobid_rs::{
    prune_cache, clear_cache, get_cache_summary, get_max_cache_size,
    get_cache_size, get_human_readable_cache_size,
    DEFAULT_MAX_CACHE_SIZE, CACHE_MAX_SIZE_ENV, CACHE_AUTO_PRUNE_ENV
};
use std::process::ExitCode;
use tracing::{debug, error, info};

/// Process cache commands
pub fn process_cache_command(command: &CacheCommands) -> CliExitCode {
    match command {
        CacheCommands::Info => display_cache_info(),
        CacheCommands::Prune { max_size_gb } => prune_cache_command(*max_size_gb),
        CacheCommands::Clear => clear_cache_command(),
    }
}

/// Display information about the cache
fn display_cache_info() -> CliExitCode {
    debug!("Displaying cache information");
    
    // Get cache information
    match get_cache_summary() {
        Ok(summary) => {
            println!("\nCache Information\n=================\n");
            println!("{}", summary);
            
            // Display environment variables
            println!("\nConfiguration");
            println!("-------------");
            println!("Environment variables:");
            println!("  {}=<bytes>  (Current: {})", 
                CACHE_MAX_SIZE_ENV, 
                std::env::var(CACHE_MAX_SIZE_ENV).unwrap_or_else(|_| format!("<not set, default: {} bytes>", DEFAULT_MAX_CACHE_SIZE)));
            println!("  {}=<true|false>  (Current: {})", 
                CACHE_AUTO_PRUNE_ENV, 
                std::env::var(CACHE_AUTO_PRUNE_ENV).unwrap_or_else(|_| "<not set, default: true>".to_string()));
            
            CliExitCode::Success
        },
        Err(e) => {
            error!("Failed to get cache information: {}", e);
            eprintln!("Error: Failed to get cache information: {}", e);
            e.into()
        }
    }
}

/// Prune the cache to stay under the specified size limit
fn prune_cache_command(max_size_gb: f64) -> CliExitCode {
    let max_size_bytes = (max_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;
    
    debug!("Pruning cache to {} bytes ({:.2} GB)", max_size_bytes, max_size_gb);
    
    // Get current cache size for comparison
    let current_size = match get_cache_size() {
        Ok(size) => size,
        Err(e) => {
            error!("Failed to get current cache size: {}", e);
            eprintln!("Error: Failed to get current cache size: {}", e);
            return e.into();
        }
    };
    
    let human_readable_size = match get_human_readable_cache_size() {
        Ok(size) => size,
        Err(e) => {
            error!("Failed to get human-readable cache size: {}", e);
            format!("{} bytes", current_size)
        }
    };
    
    println!("Current cache size: {} / {:.2} GB", human_readable_size, max_size_gb);
    
    // Check if pruning is needed
    if current_size <= max_size_bytes {
        println!("Cache is already under the specified size limit. No pruning needed.");
        return CliExitCode::Success;
    }
    
    println!("Pruning cache...");
    
    // Prune the cache
    match prune_cache(max_size_bytes) {
        Ok((files_removed, bytes_removed)) => {
            // Get new size after pruning
            let new_size = match get_cache_size() {
                Ok(size) => size,
                Err(_) => current_size - bytes_removed,
            };
            
            let new_human_readable_size = format_bytes(new_size);
            let freed_human_readable = format_bytes(bytes_removed);
            
            println!("Cache pruned successfully:");
            println!("  Files removed: {}", files_removed);
            println!("  Bytes freed: {}", freed_human_readable);
            println!("  New cache size: {} / {:.2} GB", new_human_readable_size, max_size_gb);
            
            CliExitCode::Success
        },
        Err(e) => {
            error!("Failed to prune cache: {}", e);
            eprintln!("Error: Failed to prune cache: {}", e);
            e.into()
        }
    }
}

/// Clear the entire cache
fn clear_cache_command() -> CliExitCode {
    debug!("Clearing entire cache");
    
    // Get current cache size for reporting
    let current_size = match get_cache_size() {
        Ok(size) => size,
        Err(e) => {
            error!("Failed to get current cache size: {}", e);
            eprintln!("Error: Failed to get current cache size: {}", e);
            return e.into();
        }
    };
    
    let human_readable_size = match get_human_readable_cache_size() {
        Ok(size) => size,
        Err(e) => {
            error!("Failed to get human-readable cache size: {}", e);
            format!("{} bytes", current_size)
        }
    };
    
    println!("Current cache size: {}", human_readable_size);
    println!("Clearing entire cache...");
    
    // Clear the cache
    match clear_cache() {
        Ok((files_removed, bytes_removed)) => {
            let freed_human_readable = format_bytes(bytes_removed);
            
            println!("Cache cleared successfully:");
            println!("  Files removed: {}", files_removed);
            println!("  Bytes freed: {}", freed_human_readable);
            
            CliExitCode::Success
        },
        Err(e) => {
            error!("Failed to clear cache: {}", e);
            eprintln!("Error: Failed to clear cache: {}", e);
            e.into()
        }
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