# grobid-rs

A Rust library providing JNI bindings to [Grobid](https://github.com/kermitt2/grobid) - a machine learning library for extracting, parsing, and structuring raw scholarly documents.

## Overview

grobid-rs enables Rust applications to leverage Grobid's powerful document processing capabilities through a type-safe, idiomatic Rust API. The library handles the complex JNI integration, resource management, and cross-platform compatibility issues, making it straightforward to extract structured information from PDFs in your Rust applications.

## Features

- Full access to core Grobid functionality (header extraction, full-text processing, reference parsing)
- Automatic management of Grobid resources and JVM lifecycle 
- Builder pattern for flexible configuration (memory, logging, system properties)
- Cross-platform support (Linux, macOS, Windows) with tested CI pipeline
- Comprehensive error handling with proper error taxonomy
- Powerful CLI with intuitive subcommands and multiple output formats
- Fast parallel processing with configurable thread pool
- Performance-optimized caching system with automatic pruning
- Support for offline builds with vendored dependencies
- Optimized build system with parallel downloads and streaming extraction
- Version compatibility checking to prevent cryptic JNI errors

## Installation

Add grobid-rs to your Cargo.toml:

```toml
[dependencies]
grobid-rs = "0.1.0"
```

### Features

The library provides several optional features:

- **cli**: Include the command-line interface tools
- **parallel**: Enable parallel processing support (recommended for batch processing)
- **default**: Basic functionality with single-threaded processing

```toml
# For CLI with parallel processing support
[dependencies]
grobid-rs = { version = "0.1.0", features = ["cli", "parallel"] }
```

### System Requirements

- Rust toolchain 1.65+
- JDK 11+ (with JAVA_HOME correctly set, only needed at build time)
- ~500MB disk space for Grobid resources
- ~1GB RAM minimum for processing

## Usage

### Basic Example

```rust
use grobid_rs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with configuration builder pattern
    let config = grobid_rs::GrobidConfig::builder()
        .base_path(Path::new("/path/to/grobid-resources"))
        .max_memory("2G")
        .log_level(grobid_rs::LogLevel::Info)
        .thread_count(4)  // For parallel processing
        .build()?;
    
    grobid_rs::init_with_config(&config)?;
    
    // Process a PDF file
    let pdf_path = Path::new("path/to/document.pdf");
    let header_xml = grobid_rs::process_header(pdf_path)?;
    
    // The result is TEI XML containing the extracted header information
    println!("Extracted header: {}", header_xml);
    
    Ok(())
}
```

### Using the Cache System

```rust
// Configure the cache
let cache_config = grobid_rs::CacheConfig {
    enabled: true,
    skip_existing: true,
    force_reprocess: false,
};

// Process with caching
let cached_result = grobid_rs::process_with_cache(
    pdf_path,
    grobid_rs::OutputType::Tei,
    cache_config,
    || grobid_rs::process_header(pdf_path)
)?;

// Manage cache size to prevent unbounded growth
grobid_rs::prune_cache(1024 * 1024 * 100)?; // Limit to 100MB

// Get cache usage information
let summary = grobid_rs::get_cache_summary()?;
println!("Cache status: {}", summary);
```

## CLI Usage

With the `cli` feature enabled, you can use the intuitive command-line interface:

```bash
# Process document header (outputs TEI XML by default)
grobid-cli header document.pdf

# Extract references as BibTeX (default format for references)
grobid-cli references document.pdf

# Process full text and output as JSON
grobid-cli fulltext document.pdf --output-format json

# Save output to a file
grobid-cli fulltext document.pdf --output-format json -o document.json

# Advanced options
grobid-cli --grobid-base /path/to/grobid-resources \
  --max-memory 2G \
  --log-level debug \
  -D http.proxyHost=proxy.example.com \
  -D http.proxyPort=8080 \
  header document.pdf
```

For help on available commands and options:

```bash
grobid-cli --help
grobid-cli header --help
```

## Project Structure

- `src/` - Main library code
  - `lib.rs` - Core JNI integration and Grobid API
  - `bin/` - CLI implementation
- `build_modules/` - Build script modules for Grobid resource management
- `build.rs` - Build script that handles JNI setup and resource acquisition
- `xtask/` - Development and maintenance utilities
  - `clean_runtime` - Tool to clean and force rebuild of the JRE
  - `vendor` - Tool to vendor dependencies for offline builds
- `vendor/` - Minimal vendored dependencies for offline builds
  - `grobid/` - Minimal Grobid components
  - `jre/` - Minimal JRE components

## Documentation

- [GitHub Actions CI Caching](docs/CI_CACHING.md) - Learn about our performance-optimized CI pipeline
- [Git Hooks Setup](docs/GIT_HOOKS.md) - Automated code formatting and workflow validation
- [Changelog](CHANGELOG.md) - Detailed list of changes and features

API documentation:

```bash
# Generate and open API documentation
cargo doc --open
```

## Building from Source

Prerequisites:
- Rust toolchain (1.65+)
- JDK 11+ with JAVA_HOME correctly set
- Internet connection (for downloading Grobid assets during build, unless using vendored files)
- At least 1GB of free RAM (for Grobid processing)
- About 500MB of disk space for Grobid resources

```bash
# Clone the repository
git clone https://github.com/username/grobid-rs.git
cd grobid-rs

# Build the library with default features
cargo build

# Build with CLI and parallel support
cargo build --features cli,parallel

# Run tests
cargo test

# Install the CLI tool
cargo install --path . --features cli
```

### Offline Builds with Vendored Files

This project supports offline builds using minimal vendored dependencies:

```bash
# Build using vendored files if available
cargo build

# Force a complete rebuild, ignoring vendored files
FORCE_GROBID_REBUILD=true cargo build

# Clean just the JRE runtime to force a rebuild with different modules
cargo run --package xtask --bin clean_runtime
```

### Build System Features

- Caching of Grobid resources for faster rebuilds
- Parallel downloading with range-based requests
- Automatic download resumption for interrupted builds
- Memory-efficient streaming ZIP extraction
- Proper cleanup and rebuild when JRE configuration changes
- Version compatibility checking

### Development Tools

#### Git Hooks

Set up git hooks to automatically format code and run clippy checks before commits:

```bash
# Run the included installation script
./scripts/install-hooks.sh
```

See [Git Hooks Documentation](docs/GIT_HOOKS.md) for details and advanced configurations.

#### Vendoring Dependencies

To vendor the minimal necessary files for offline builds:

```bash
# First, ensure you have a complete build
FORCE_GROBID_REBUILD=true cargo build

# Then run the vendor task to copy minimal files to the vendor directory
cargo run --package xtask --bin vendor

# After vendoring, these files can be committed to the repository
```

#### Environment Variables

Control the build and runtime behavior:

- `GROBID_RS_ASSETS_PATH`: Set custom path for Grobid assets
- `FORCE_GROBID_REBUILD`: Force rebuild of Grobid assets
- `GROBID_RS_CACHE_DIR`: Set custom path for cache directory
- `GROBID_RS_CACHE_MAX_SIZE`: Set maximum cache size
- `GROBID_RS_CACHE_AUTO_PRUNE`: Enable/disable auto-pruning

## License

This project is licensed under [MIT/Apache-2.0] - see the LICENSE files for details.

Note: This project packages various components with different licenses:
- Grobid: Apache License 2.0
- pdfalto: GNU General Public License v3.0
- OpenJDK components: GPLv2 with Classpath Exception

## Acknowledgments

- [Grobid](https://github.com/kermitt2/grobid) - The core document processing engine
- [pdfalto](https://github.com/kermitt2/pdfalto) - PDF to ALTO XML converter
- [jni-rs](https://github.com/jni-rs/jni-rs) - Rust JNI bindings