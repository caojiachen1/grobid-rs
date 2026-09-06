# grobid-rs Usage Guide

## Table of Contents

- [Installation](#installation)
- [Initialization](#initialization)
- [API Quick Reference](#api-quick-reference)
- [Detailed API Reference](#detailed-api-reference)
  - [Initialization & Lifecycle](#1-initialization--lifecycle)
  - [Low-Level Engine API (TEI XML)](#2-low-level-engine-api-tei-xml)
  - [High-Level API](#3-high-level-api)
  - [Cache System](#4-cache-system)
  - [Data Models](#5-data-models)
  - [Error Handling](#6-error-handling)
- [Complete Examples](#complete-examples)

---

## Installation

In `Cargo.toml`:

```toml
[dependencies]
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs" }

# If you only need the cache feature (enabled by default):
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs" }

# Pick features as needed (these are the current defaults; omitting them is fine):
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs", features = ["cache", "format", "cli", "parallel"] }
```

| Feature | Default | Description |
|---------|:----:|------|
| `cache`  | ✅ | SHA-256 based PDF caching to avoid duplicate processing |
| `format` | ✅ | TEI XML → JSON / Text / BibTeX conversion |
| `cli`    | ✅ | The `grobid-cli` CLI binary |
| `parallel` | ✅ | Multi-threaded batch processing based on `rayon` |

### System Requirements

| Requirement | Description |
|------|------|
| Rust 1.65+ | Stable toolchain is sufficient |
| JDK 11+ | **Only required at build time**, used by `jlink` to generate the JRE. Set `JAVA_HOME` |
| Disk space | Build artifacts are about 1.3 GB (models + JRE + JAR) |
| Memory | At least 1 GB RAM to process PDFs |

---

## Initialization

`grobid-rs` requires calling `init()` before use. The JVM and the Grobid engine start as singletons; only one call per process is needed.

```rust
use std::path::Path;

// === Option 1: Default configuration (auto-detects the build artifact path) ===
let config = grobid_rs::GrobidConfig::builder().build();
grobid_rs::init(&config)?;

// === Option 2: Manually specify the resource path (recommended for packaging/distribution) ===
let config = grobid_rs::GrobidConfig::builder()
    .base_path("/path/to/grobid-assets")  // contains the JAR + grobid-home/ + runtime/
    .max_memory("2G")
    .thread_count(4)
    .log_level(grobid_rs::LogLevel::Debug)
    .system_property("my.key", "my.value")
    .jvm_option("-XX:+UseG1GC")
    .analysis_config()                    // switch to the analysis config builder
        .consolidate_header(true)
        .consolidate_citations(true)
        .include_coordinates(false)
        .segment_sentences(false)
        .generate_raw_citations(true)
        .done()                           // returns to the main builder
    .build();

grobid_rs::init(&config)?;

// === Option 3: Minimal (new + chained calls) ===
grobid_rs::init(
    &grobid_rs::GrobidConfig::new("/path/to/grobid-assets")
        .with_max_memory("4G")
        .with_thread_count(8)
        .with_log_level(grobid_rs::LogLevel::Info)
)?;
```

### Lifecycle Functions

```rust
// Initialize (idempotent, safe to call multiple times)
grobid_rs::init(&config)?;

// Check whether it has been initialized
if grobid_rs::is_initialized() {
    println!("Grobid is ready");
}

// Shut down (cleans up engine references)
grobid_rs::shutdown()?;
```

---
## API Quick Reference

| Processing Type | TEI XML (raw) | JSON | Structured Rust Types |
|---------|----------------|------|----------------|
| **Full text** | `grobid_rs::fulltext_to_tei()` | `grobid_rs::fulltext_to_json()` | `grobid_rs::fulltext_to_structured()` → `GrobidDocument` |
| **Header** | `grobid_rs::process_header()` | `grobid_rs::process_header_json()` | `grobid_rs::process_header_structured()` → `DocumentMetadata` |
| **References** | `grobid_rs::process_references()` | `grobid_rs::process_references_json()` | `grobid_rs::process_references_structured()` → `Vec<Reference>` |
| **Custom** | — | — | `grobid_rs::parse_tei_str()` → `ParsedTei` (parses arbitrary TEI) |

---

## Detailed API Reference

### 1. Initialization & Lifecycle

#### `GrobidConfig`

```rust
pub struct GrobidConfig {
    pub base_path: PathBuf,          // Resource directory (contains the JAR + grobid-home/ + runtime/)
    pub max_memory: String,          // JVM max heap "-Xmx", default "1G"
    pub jvm_options: Vec<String>,    // Extra JVM options
    pub thread_count: usize,         // Parallel thread count, default 1
    pub system_properties: HashMap<String, String>,  // Custom -D system properties
    pub log_level: LogLevel,         // Log level, default Info
    pub prefer_vendored: bool,       // Whether to prefer bundled files
    pub analysis_config: Option<GrobidAnalysisConfig>,  // Analysis configuration
}
```

Constructors:

```rust
// GrobidConfig::new(base_path) — quick creation
let config = grobid_rs::GrobidConfig::new("/path/to/assets");

// GrobidConfig::builder() — full builder
let config = grobid_rs::GrobidConfig::builder()
    .base_path("/path/to/assets")
    .max_memory("2G")
    .thread_count(4)
    .log_level(grobid_rs::LogLevel::Debug)
    .jvm_option("-XX:+UseG1GC")
    .system_property("key", "val")
    .prefer_vendored(true)
    .build();

// Chained modification (after new)
let config = grobid_rs::GrobidConfig::new("/path")
    .with_max_memory("4G")
    .with_thread_count(8)
    .with_log_level(grobid_rs::LogLevel::Trace)
    .with_jvm_option("-XX:+UseZGC")
    .with_system_property("key", "val")
    .with_prefer_vendored(true)
    .with_analysis_config(my_analysis_config);
```

#### `GrobidAnalysisConfig`

```rust
pub struct GrobidAnalysisConfig {
    pub consolidate_header: bool,       // Consolidate the header with an external service, default false
    pub consolidate_citations: bool,    // Consolidate citations with an external service, default false
    pub include_coordinates: bool,      // Include coordinate information, default false
    pub segment_sentences: bool,        // Segment sentences, default false
    pub generate_raw_citations: bool,   // Generate raw citations, default true
}
```

Using the builder:

```rust
let analysis = grobid_rs::GrobidAnalysisConfig::builder()
    .consolidate_header(true)
    .consolidate_citations(true)
    .include_coordinates(false)
    .segment_sentences(true)
    .generate_raw_citations(true)
    .done()  // ← Note: returns GrobidConfigBuilder, not the final Config
    .build(); // ← Finally builds the GrobidConfig
```

#### `LogLevel`

```rust
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
```

#### Constants

```rust
pub const GROBID_VERSION: &str = "0.9.1";
```

---

### 2. Low-Level Engine API (TEI XML)

These three functions return Grobid's native TEI XML (strings) and are the lowest-level API.

#### `grobid_rs::process_header`

```rust
pub fn process_header(pdf: &Path) -> Result<String, GrobidError>
```

Input: PDF file path
Output: TEI XML string (header metadata: title, authors, abstract, etc.)

```rust
let tei = grobid_rs::process_header(Path::new("paper.pdf"))?;
println!("{}", tei); // ← TEI XML
```

#### `grobid_rs::fulltext_to_tei`

```rust
pub fn fulltext_to_tei(pdf: &Path) -> Result<String, GrobidError>
```

Input: PDF file path
Output: TEI XML string (full text + header + references)

```rust
let tei = grobid_rs::fulltext_to_tei(Path::new("paper.pdf"))?;
```

#### `grobid_rs::process_references`

```rust
pub fn process_references(pdf: &Path) -> Result<String, GrobidError>
```

Input: PDF file path
Output: TEI XML string (references list only)

```rust
let tei = grobid_rs::process_references(Path::new("paper.pdf"))?;
```

#### `grobid_rs::run_pdfalto`

```rust
pub fn run_pdfalto(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError>
```

Runs pdfalto (a PDF → ALTO XML tool) and returns the path to the generated ALTO XML file.

```rust
let alto_path = grobid_rs::run_pdfalto(
    Path::new("paper.pdf"),
    Path::new("/path/to/grobid-home"),
)?;
// Generates paper.alto.xml
```

---
### 3. High-Level API

Each processing type provides three output formats:

| Suffix | Purpose |
|------|------|
| `_json()` | Returns a JSON string |
| `_json_with_options(pdf_path, pretty)` | Returns JSON, controlling pretty-printing |
| `_structured()` | Returns strongly typed Rust structs |

#### Header API

```rust
// Header JSON (pretty-printed)
let json = grobid_rs::process_header_json(Path::new("paper.pdf"))?;

// Header JSON (compact)
let json = grobid_rs::process_header_json_with_options(
    Path::new("paper.pdf"),
    false,  // pretty = false → compact
)?;

// Header structured
let metadata: grobid_rs::DocumentMetadata =
    grobid_rs::process_header_structured(Path::new("paper.pdf"))?;

println!("Title: {:?}", metadata.title);
println!("Number of authors: {}", metadata.authors.len());
println!("Abstract: {:?}", metadata.abstract_text);
```

#### Full-Text API

```rust
// Full-text JSON (pretty-printed)
let json = grobid_rs::fulltext_to_json(Path::new("paper.pdf"))?;

// Full-text JSON (compact)
let json = grobid_rs::fulltext_to_json_with_options(
    Path::new("paper.pdf"),
    false,
)?;

// Full text structured
let doc: grobid_rs::GrobidDocument =
    grobid_rs::fulltext_to_structured(Path::new("paper.pdf"))?;

println!("Source: {}", doc.source);
println!("Version: {}", doc.version);
println!("Number of references: {}", doc.references.len());

if let Some(full_text) = &doc.full_text {
    println!("Number of sections: {}", full_text.sections.len());
    println!("Number of figures: {}", full_text.figures.len());
    println!("Number of tables: {}", full_text.tables.len());
}
```

#### References API

```rust
// References JSON (pretty-printed)
let json = grobid_rs::process_references_json(Path::new("paper.pdf"))?;

// References JSON (compact)
let json = grobid_rs::process_references_json_with_options(
    Path::new("paper.pdf"),
    false,
)?;

// References structured
let refs: Vec<grobid_rs::Reference> =
    grobid_rs::process_references_structured(Path::new("paper.pdf"))?;

for r in &refs {
    println!("  [{}] {:?}", r.id.as_deref().unwrap_or("?"), r.title.as_deref().unwrap_or("?"));
    println!("    Authors: {}", r.authors.join("; "));
    println!("    DOI: {:?}", r.doi);
}
```

#### `parse_tei_str` — Parse Arbitrary TEI XML

```rust
pub fn parse_tei_str(tei: &str) -> Result<ParsedTei, GrobidError>
```

```rust
let tei_xml = grobid_rs::fulltext_to_tei(Path::new("paper.pdf"))?;

match grobid_rs::parse_tei_str(&tei_xml)? {
    grobid_rs::ParsedTei::Header(metadata) => {
        println!("Header only: {:?}", metadata.title);
    }
    grobid_rs::ParsedTei::References(refs) => {
        println!("References only: {} entries", refs.len());
    }
    grobid_rs::ParsedTei::Full(doc) => {
        println!("Full document: {} references", doc.references.len());
    }
}
```

---

### 4. Cache System

#### `CacheConfig`

```rust
pub struct CacheConfig {
    pub enabled: bool,           // Whether caching is enabled, default true
    pub skip_existing: bool,     // Whether to skip processing on cache hit, default true
    pub force_reprocess: bool,   // Whether to force reprocessing, default false
}
```

#### `OutputType`

```rust
pub enum OutputType { Tei, Json, Bibtex, Text }

impl OutputType {
    pub fn extension(&self) -> &'static str {
        // Tei → "tei", Json → "json", Bibtex → "bib", Text → "txt"
    }
}
```

#### `CacheStats`

```rust
pub struct CacheStats {
    pub hits: usize,           // Number of cache hits
    pub misses: usize,         // Number of cache misses
    pub bytes_read: usize,     // Bytes read from the cache
    pub bytes_written: usize,  // Bytes written to the cache
    pub time_saved_ms: u64,    // Estimated time saved by the cache (milliseconds)
}
```

#### Cached Processing API

```rust
// Generic cached processing
let result = grobid_rs::process_with_cache(
    Path::new("paper.pdf"),
    grobid_rs::OutputType::Json,  // Cached output type
    grobid_rs::CacheConfig {
        enabled: true,
        skip_existing: true,
        force_reprocess: false,
    },
    || grobid_rs::fulltext_to_tei(Path::new("paper.pdf")),  // Processing function
)?;

// All-in-one convenience functions
let tei = grobid_rs::fulltext_to_tei_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;

let tei = grobid_rs::process_header_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;

let tei = grobid_rs::process_references_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;
```

#### Low-Level Cache Operations

```rust
// Get the cache directory
let cache_dir = grobid_rs::get_cache_dir()?;

// Get the cache path for a given PDF + output type
let cache_path = grobid_rs::get_cache_path(Path::new("paper.pdf"), grobid_rs::OutputType::Json)?;

// Get the path of the cached file (if it exists)
let path = grobid_rs::get_cached_path(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// Check whether the cache exists
let exists = grobid_rs::cache_exists(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// Read cache content
let cached = grobid_rs::read_cache(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// Write to the cache
grobid_rs::write_cache(Path::new("paper.pdf"), grobid_rs::OutputType::Tei, "TEI content")?;

// Get cache statistics
let stats = grobid_rs::get_cache_stats();
println!("Hits: {}, Misses: {}, Saved: {}ms",
    stats.hits, stats.misses, stats.time_saved_ms);

// Reset cache statistics
grobid_rs::reset_cache_stats();
```

#### Cache Management

```rust
// Prune the cache (limit the maximum size)
let (removed_files, removed_bytes) = grobid_rs::prune_cache(1024 * 1024 * 500)?;  // Limit 500 MB
println!("Removed {} files, freed {} bytes", removed_files, removed_bytes);

// Clear all cache
let (removed, bytes) = grobid_rs::clear_cache()?;

// Get the cache size
let size_bytes = grobid_rs::get_cache_size()?;

// Get a human-readable cache size
let size_str = grobid_rs::get_human_readable_cache_size()?;
println!("Cache size: {}", size_str);

// List all cached files
let files = grobid_rs::list_cache_files()?;

// Cache summary
let summary = grobid_rs::get_cache_summary()?;
println!("{}", summary);

// Start the background GC thread (checks and prunes every hour)
grobid_rs::start_background_gc();

// Manually trigger a check and prune
grobid_rs::check_and_prune_if_needed()?;

// Ensure the cache directory exists
grobid_rs::ensure_cache_dir()?;
```

**Environment variables:**

| Variable | Description | Default |
|------|------|------|
| `GROBID_RS_CACHE_DIR` | Overrides the cache directory | System default cache directory |
| `GROBID_RS_CACHE_MAX_SIZE` | Maximum cache size (bytes) | 10 GB |
| `GROBID_RS_CACHE_AUTO_PRUNE` | Enables automatic pruning | Depends on the implementation |

---
### 5. Data Models

#### `GrobidDocument`

```rust
pub struct GrobidDocument {
    pub source: String,                          // Always "grobid-rs"
    pub version: String,                         // Library version (from Cargo.toml)
    pub metadata: DocumentMetadata,              // Document metadata
    pub full_text: Option<FullText>,             // Full text (optional, only in fulltext mode)
    pub references: Vec<Reference>,              // References
}
```

All fields automatically derive `Serialize` + `Deserialize` and can be serialized directly to JSON.

#### `DocumentMetadata`

```rust
pub struct DocumentMetadata {
    pub title: Option<String>,                   // Document title
    pub authors: Vec<Author>,                    // Author list
    pub abstract_text: Option<String>,           // Abstract
    pub date: Option<Date>,                      // Publication date
    pub doi: Option<String>,                     // DOI
    pub venue: Option<Venue>,                    // Publishing journal/conference
    pub keywords: Vec<String>,                   // Keywords
    pub other: HashMap<String, String>,          // Other metadata
}
```

#### `Author`

```rust
pub struct Author {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub affiliation: Option<String>,             // Affiliation
    pub identifier: Option<String>,              // ORCID, etc.
}
```

#### `Date`

```rust
pub struct Date {
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub raw: Option<String>,                     // Raw date string
}
```

#### `Venue`

```rust
pub struct Venue {
    pub name: Option<String>,                    // Journal/conference name
    pub volume: Option<String>,                  // Volume
    pub issue: Option<String>,                   // Issue
    pub pages: Option<String>,                   // Page range
    pub publisher: Option<String>,
}
```

#### `FullText`

```rust
pub struct FullText {
    pub sections: Vec<Section>,                  // Sections
    pub figures: Vec<Figure>,                    // Figures
    pub tables: Vec<Table>,                      // Tables
    pub equations: Vec<Equation>,                // Equations
}
```

#### `Section`

```rust
pub struct Section {
    pub title: Option<String>,
    pub level: u8,                                // Heading level (1 = top-level heading)
    pub content: String,                          // Body text
    pub subsections: Vec<Section>,                // Subsections
}
```

#### `Figure` / `Table` / `Equation`

```rust
pub struct Figure {
    pub id: Option<String>,
    pub caption: Option<String>,
    pub description: Option<String>,
}

pub struct Table {
    pub id: Option<String>,
    pub caption: Option<String>,
    pub content: Option<String>,
}

pub struct Equation {
    pub id: Option<String>,
    pub content: String,                          // MathML or LaTeX
    pub description: Option<String>,
}
```

#### `Reference`

```rust
pub struct Reference {
    pub id: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<String>,                     // Author names (flat list of strings)
    pub date: Option<Date>,
    pub venue: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub publisher: Option<String>,
    pub doi: Option<String>,
    pub raw: Option<String>,                      // Raw citation text
}
```

#### `ParsedTei`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ParsedTei {
    Header(DocumentMetadata),       // Header only
    References(Vec<Reference>),     // References only
    Full(GrobidDocument),           // Full document
}
```

---

### 6. Error Handling

All fallible functions return `Result<_, GrobidError>`.

#### `GrobidError` Variants

```rust
pub enum GrobidError {
    NotInitialised,                // init() was not called
    Jni(JniError),                 // JNI interaction error
    JvmInitialization(String),     // JVM startup failed
    Java(String),                  // Exception on the Java side
    PdfAlto(String),               // pdfalto tool error
    InvalidInput(String),          // Invalid input arguments (e.g. path does not exist)
    Configuration(String),         // Configuration error
    Io(std::io::Error),            // File I/O error
    VersionMismatch { expected, found },  // Grobid version mismatch
    Cache(String),                 // Cache error
    ParseError(String),            // XML parsing error
    UnexpectedEof(String),         // Unexpected end of file
    XmlParseError { message, context },  // XML parsing context error
    MalformedXml { message, expected, found },  // Malformed XML
    SerializationError(String),    // JSON serialization error
    DeserializationError(String),  // JSON deserialization error
    Conversion(String),            // Type conversion error
}
```

#### Convenience Constructors

```rust
// File not found
GrobidError::file_not_found("path/to/file.pdf");

// Invalid input
GrobidError::invalid_input("file is empty");

// Version mismatch
GrobidError::version_mismatch("0.9.1", "0.8.2");
```

#### Typical Error Handling Pattern

```rust
use grobid_rs::GrobidError;

fn process_pdf(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(path);

    // Match specific errors
    match grobid_rs::process_header(path) {
        Ok(tei) => println!("Success: {}", tei),
        Err(GrobidError::NotInitialised) => {
            eprintln!("Call init() first");
        }
        Err(GrobidError::InvalidInput(msg)) => {
            eprintln!("Invalid input: {}", msg);
        }
        Err(e) => {
            eprintln!("Processing failed: {}", e);
        }
    }

    // Or propagate errors with ?
    let metadata = grobid_rs::process_header_structured(path)?;
    Ok(())
}
```

---
## Complete Examples

### Basic: Extracting Header + Full Text + References

```rust
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize
    grobid_rs::init(&grobid_rs::GrobidConfig::builder().build())?;

    let pdf = Path::new("paper.pdf");

    // 2. Extract the header (structured)
    let meta = grobid_rs::process_header_structured(pdf)?;
    println!("Title: {:?}", meta.title);
    println!("Authors: {}", meta.authors.iter()
        .filter_map(|a| a.full_name.as_deref())
        .collect::<Vec<_>>()
        .join(", "));
    println!("Abstract: {:?}", meta.abstract_text.map(|s| s.chars().take(200).collect::<String>()));

    // 3. Extract the full text (JSON output)
    let json = grobid_rs::fulltext_to_json(pdf)?;
    std::fs::write("paper.json", json)?;

    // 4. Extract references
    let refs = grobid_rs::process_references_structured(pdf)?;
    for (i, r) in refs.iter().enumerate() {
        println!("[{}] {} — {}", i + 1,
            r.title.as_deref().unwrap_or("(no title)"),
            r.authors.join("; "));
    }

    Ok(())
}
```

### Production Use Case with Caching

```rust
use std::path::Path;
use grobid_rs::{CacheConfig, process_with_cache, OutputType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    grobid_rs::init(&grobid_rs::GrobidConfig::builder().build())?;

    let pdf = Path::new("paper.pdf");

    let result = process_with_cache(
        pdf,
        OutputType::Json,
        CacheConfig {
            enabled: true,
            skip_existing: true,    // Cache exists → skip processing
            force_reprocess: false, // Do not force reprocessing
        },
        || grobid_rs::fulltext_to_json(pdf),
    )?;

    println!("{}", result);

    // Inspect cache statistics
    let stats = grobid_rs::get_cache_stats();
    println!("Cache: {} hits / {} misses, saved {}ms",
        stats.hits, stats.misses, stats.time_saved_ms);

    Ok(())
}
```

### Custom Configuration

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = grobid_rs::GrobidConfig::builder()
        .max_memory("4G")
        .thread_count(8)
        .log_level(grobid_rs::LogLevel::Info)
        .analysis_config()
            .consolidate_header(true)
            .consolidate_citations(true)
            .include_coordinates(false)
            .segment_sentences(true)
            .done()
        .build();

    grobid_rs::init(&config)?;

    let meta = grobid_rs::process_header_structured(Path::new("paper.pdf"))?;

    // Serialize to JSON
    println!("{}", serde_json::to_string_pretty(&meta)?);

    Ok(())
}
```

### Cache Management

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the background auto-pruning (checks every hour)
    grobid_rs::start_background_gc();

    // Or prune manually
    let (removed, freed) = grobid_rs::prune_cache(500 * 1024 * 1024)?; // Limit 500MB
    println!("Removed {} files, freed {} MB", removed, freed / 1024 / 1024);

    // View the cache summary
    println!("{}", grobid_rs::get_cache_summary()?);

    // Clear the cache
    grobid_rs::clear_cache()?;

    Ok(())
}
```

### Batch Processing Multiple PDFs

```rust
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = grobid_rs::GrobidConfig::builder()
        .thread_count(4)
        .build();

    grobid_rs::init(&config)?;

    let pdfs = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];

    // Batch processing with caching
    for pdf_name in &pdfs {
        let pdf = Path::new(pdf_name);
        match grobid_rs::process_header_cached(pdf, grobid_rs::CacheConfig::default()) {
            Ok(tei) => println!("{} → Success ({} bytes)", pdf_name, tei.len()),
            Err(e) => eprintln!("{} → Failed: {}", pdf_name, e),
        }
    }

    Ok(())
}
```

---

## Integration in Tauri

See [Tauri Resource Bundling](https://v2.tauri.app/develop/resources/):

```json
// tauri.conf.json
{
  "bundle": {
    "resources": ["grobid-assets/**"]
  }
}
```

```rust
// Initialize on the Rust side
let resource_dir = app.path().resource_dir()?;
let grobid_base = resource_dir.join("grobid-assets");

let config = grobid_rs::GrobidConfig::builder()
    .base_path(&grobid_base)
    .build();

grobid_rs::init(&config)?;
```

Or download to the user data directory on first launch:

```rust
let app_data = app.path().app_data_dir()?;
let grobid_dir = app_data.join("grobid-assets");

if !grobid_dir.exists() {
    // Download and extraction logic
    download_and_extract("https://releases.example.com/grobid-assets.tar.zst", &grobid_dir)?;
}

let config = grobid_rs::GrobidConfig::builder()
    .base_path(&grobid_dir)
    .build();

grobid_rs::init(&config)?;
```

---

> For complete example code, see the [`examples/`](examples/) directory in the repository.
