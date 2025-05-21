# Grobid-rs Cache System Manual

## Overview

grobid-rs includes a built-in cache system that significantly improves performance when processing the same documents multiple times. When a PDF is processed, the results are stored in a local cache, allowing subsequent processing of the same document to be nearly instantaneous.

## Cache Location

The default cache location depends on the operating system:

- **Linux/macOS**: `$XDG_CACHE_HOME/grobid-rs` (typically `~/.cache/grobid-rs`)
- **Windows**: `%LOCALAPPDATA%\grobid-rs\cache`

You can override this location by setting the `GROBID_RS_CACHE_DIR` environment variable.

## Cache Keys

Each processed document is uniquely identified by:
- SHA-256 hash of the PDF content
- Grobid version (to invalidate cache when Grobid is updated)

This ensures that identical documents produce cache hits while any change to the document or Grobid version invalidates the cache.

## CLI Commands

### Basic usage flags

These flags can be added to any processing command:

```
--no-cache           Disable caching entirely
--force-reprocess    Force reprocessing even if cached result exists
--skip-existing      Skip processing if cached results exist (default: true)
--stats              Display cache statistics after processing
```

Example:
```
grobid-cli header document.pdf --stats
grobid-cli fulltext document.pdf --force-reprocess
grobid-cli references document.pdf --no-cache
```

### Cache management commands

The CLI provides specialized commands for cache management:

#### Display cache information
```
grobid-cli cache info
```

This command shows:
- Current cache directory location
- Total cache size and limits
- Number of cached files
- Auto-pruning status
- Current session statistics (hits, misses, hit rate)

#### Prune the cache
```
grobid-cli cache prune [SIZE_GB]
```

Removes the least recently accessed files to keep the cache under the specified size limit (in gigabytes). Default is 10GB if not specified.

Example:
```
grobid-cli cache prune 5  # Limit to 5GB
```

#### Clear the entire cache
```
grobid-cli cache clear
```

Removes all files from the cache.

## Environment Variables

The following environment variables control cache behavior:

- `GROBID_RS_CACHE_DIR`: Custom cache directory location
- `GROBID_RS_CACHE_MAX_SIZE`: Maximum cache size in bytes (default: 10GB)
- `GROBID_RS_CACHE_AUTO_PRUNE`: Enable/disable automatic pruning (`true`/`false`, default: `true`)

Example:
```bash
export GROBID_RS_CACHE_DIR="/tmp/grobid-cache"
export GROBID_RS_CACHE_MAX_SIZE=5368709120  # 5GB in bytes
export GROBID_RS_CACHE_AUTO_PRUNE=false
```

## Cache Statistics

When using the `--stats` flag, the following information is displayed:

- **Hits**: Number of times a result was retrieved from cache
- **Misses**: Number of times a document needed processing
- **Hit rate**: Percentage of requests served from cache
- **Bytes read**: Amount of data read from cache
- **Bytes written**: Amount of data written to cache
- **Estimated time saved**: Approximate processing time saved by using cache

## Cross-Process Safety

The cache implementation is thread-safe and process-safe, using file locking to prevent race conditions when multiple processes access the same cache files. This makes it safe to:

- Run multiple grobid-rs instances simultaneously
- Process documents in parallel with the `parallel` feature
- Share a cache directory between different users or applications

## Programmatic Usage

Rust library users can control cache behavior with the `CacheConfig` struct:

```rust
let cache_config = CacheConfig {
    enabled: true,          // Use cache at all
    skip_existing: true,    // Skip processing if cached result exists
    force_reprocess: false, // Force reprocessing even if cache exists
};

let result = grobid_rs::process_header_cached(&pdf_path, cache_config)?;
```

For cache maintenance in code:

```rust
// Prune to specific size
let (files_removed, bytes_freed) = grobid_rs::prune_cache(max_size_bytes)?;

// Clear entire cache
let (files_removed, bytes_freed) = grobid_rs::clear_cache()?;

// Get cache information
let size_bytes = grobid_rs::get_cache_size()?;
let size_human = grobid_rs::get_human_readable_cache_size()?;
let summary = grobid_rs::get_cache_summary()?;
```