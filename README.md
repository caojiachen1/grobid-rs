# grobid-rs

A Rust library providing JNI bindings to [Grobid](https://github.com/kermitt2/grobid) - a machine learning library for extracting, parsing, and structuring raw scholarly documents.

## Overview

grobid-rs enables Rust applications to leverage Grobid's powerful document processing capabilities through a type-safe, idiomatic Rust API. The library handles the complex JNI integration, resource management, and cross-platform compatibility issues, making it straightforward to extract structured information from PDFs in your Rust applications.

## Features

- Full access to core Grobid functionality (header extraction, full-text processing, reference parsing)
- Automatic management of Grobid resources and JVM lifecycle
- Flexible configuration options for JVM memory, logging, and system properties
- Cross-platform support (Linux, macOS, Windows)
- Comprehensive error handling
- Optional CLI integration
- Support for parallel processing with the `parallel` feature
- Support for offline builds with vendored dependencies
- Optimized build system with parallel downloads and streaming extraction
- Performance-optimized caching with automatic pruning for faster re-processing

## Installation

Add grobid-rs to your Cargo.toml:

```toml
[dependencies]
grobid-rs = "0.1.0"
```

For CLI features, use:

```toml
[dependencies]
grobid-rs = { version = "0.1.0", features = ["cli"] }
```

For parallel processing support, use:

```toml
[dependencies]
grobid-rs = { version = "0.1.0", features = ["parallel"] }
```

Or combine features:

```toml
[dependencies]
grobid-rs = { version = "0.1.0", features = ["cli", "parallel"] }
```

## Usage

Basic example of extracting header information from a PDF:

```rust
use grobid_rs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple initialization with default configuration
    let base_path = Path::new("/path/to/grobid-resources");
    grobid_rs::init(base_path)?;
    
    // Or use advanced configuration options
    let config = grobid_rs::GrobidConfig::new(base_path)
        .with_max_memory("2G")
        .with_log_level(grobid_rs::LogLevel::Debug)
        .with_system_property("http.proxyHost", "proxy.example.com")
        .with_system_property("http.proxyPort", "8080")
        .with_jvm_option("-Xss2m");
    
    grobid_rs::init_with_config(&config)?;
    
    // Process a PDF file
        let pdf_path = Path::new("path/to/document.pdf");
        let header_xml = grobid_rs::process_header(pdf_path)?;
    
        // The result is TEI XML containing the extracted header information
        println!("Extracted header: {}", header_xml);
    
        // Use caching for faster reprocessing
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
    
        Ok(())
    }
    ```

## CLI Usage

If built with the `cli` feature, you can use the included command-line tool:

```bash
# Basic usage
grobid-cli --pdf-file /path/to/document.pdf --grobid-base /path/to/grobid-resources

# Advanced options
grobid-cli --pdf-file /path/to/document.pdf --grobid-base /path/to/grobid-resources \
  --max-memory 2G \
  --log-level debug \
  -D http.proxyHost=proxy.example.com \
  -D http.proxyPort=8080 \
  -J "-Xss2m"

# Caching options
grobid-cli --pdf-file /path/to/document.pdf --grobid-base /path/to/grobid-resources \
  --cache-enabled \
  --skip-existing \
  --max-cache-size 100M
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

Detailed documentation is available in the `docs/` directory:

- [Core Rust-Grobid JNI Integration](docs/1_INTEGRATION.md)
- [Managing Grobid Resources](docs/2_RESOURCES.md)
- [Packaging and Distribution](docs/3_DISTRIBUTION.md)
- [Debugging and Advanced Topics](docs/4_ADVANCED.md)
- [GitHub Actions CI Caching](docs/CI_CACHING.md)
- [Git Hooks Setup](docs/GIT_HOOKS.md)
- [Development Roadmap](docs/PLAN.md)

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

# Build the library
cargo build

# Build with CLI support
cargo build --features cli
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

The build system includes several optimizations:
- Parallel downloading with range-based requests for faster downloads
- Automatic download resumption for interrupted builds
- Memory-efficient streaming ZIP extraction
- Proper cleanup and rebuild when JRE configuration changes
- GitHub Actions caching for faster CI builds (10-15× speedup)

### Maintenance Tasks

### Git Hooks

We recommend setting up git hooks to automatically format code with rustfmt before commits:

```bash
# Install rustfmt if you don't have it
rustup component add rustfmt

# Set up a pre-commit hook to run rustfmt
mkdir -p .git/hooks
curl -o .git/hooks/pre-commit https://raw.githubusercontent.com/agustif/grobid-rs/master/scripts/pre-commit.sh
chmod +x .git/hooks/pre-commit
```

See [Git Hooks Documentation](docs/GIT_HOOKS.md) for more options and advanced configurations.

### Vendoring Dependencies

To vendor the minimal necessary files for offline builds:

```bash
# First, ensure you have a complete build
FORCE_GROBID_REBUILD=true cargo build

# Then run the vendor task to copy minimal files to the vendor directory
cargo run --package xtask --bin vendor

# After vendoring, these files can be committed to the repository
# Future builds will use these files instead of downloading dependencies
```

## License

This project is licensed under [MIT/Apache-2.0] - see the LICENSE files for details.

Note: This project packages various components with different licenses:
- Grobid: Apache License 2.0
- pdfalto: GNU General Public License v3.0
- OpenJDK components: GPLv2 with Classpath Exception

See [LICENSING.md](docs/LICENSING.md) for a comprehensive overview of all dependency licenses.

## Acknowledgments

- [Grobid](https://github.com/kermitt2/grobid) - The core document processing engine
- [pdfalto](https://github.com/kermitt2/pdfalto) - PDF to ALTO XML converter
- [jni-rs](https://github.com/jni-rs/jni-rs) - Rust JNI bindings