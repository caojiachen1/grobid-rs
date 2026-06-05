# grobid-rs Implementation Tasks

This document tracks the implementation tasks required to complete the grobid-rs project, organized by priority and component.

## Priority 1: JSON Output Implementation (High Impact)

The JSON output feature provides the highest developer experience improvement for API users.

### Core JSON Models and Converters

- [ ] Create basic model structure in `models/` directory
  - [ ] Define `Document` struct with all required fields
  - [ ] Implement `Header` struct with metadata fields
  - [ ] Create `BibEntry` struct for citation representation
  - [ ] Add `Author` and `Affiliation` supporting structures
  - [ ] Implement proper enum types for controlled vocabularies
  - [ ] Ensure all structs derive `Serialize`, `Deserialize`, `Debug`

### TEI Conversion Layer

- [ ] Implement TEI-to-JSON converters
  - [ ] Create XML parser for header documents
  - [ ] Add citation extraction and normalization
  - [ ] Implement fulltext structure extraction
  - [ ] Handle figures, tables, and other document elements
  - [ ] Add proper error handling for malformed XML

### Public API Extensions

- [ ] Expose JSON interfaces in public API
  - [ ] Add `process_header_json()` method
  - [ ] Implement `process_fulltext_json()` method
  - [ ] Create `process_references_json()` method
  - [ ] Add `process_*_structured()` variants returning Rust structs
  - [ ] Ensure all methods handle errors consistently

### CLI Integration

- [ ] Add JSON output support to CLI
  - [ ] Implement `--output-format json` flag
  - [ ] Maintain TEI XML as default format
  - [ ] Add pretty-printing option for JSON output

### Testing

- [ ] Create comprehensive test suite for JSON functionality
  - [ ] Unit tests for model serialization/deserialization
  - [ ] Round-trip testing for TEI→JSON→TEI conversion
  - [ ] Integration tests with snapshot comparisons
  - [ ] Edge case handling for malformed documents
  - [ ] Performance benchmarks for conversion overhead

## Priority 2: Asset Pipeline Improvements

Focusing on improving first-time user experience and installation flow.

### Downloadable Assets

- [ ] Implement pre-built asset download path
  - [ ] Detect `$CARGO_INSTALL` environment in build.rs
  - [ ] Add logic to download pre-built assets when no local JDK
  - [ ] Create GitHub release with grobid-0.9.1.tar.zst bundle
  - [ ] Implement checksum verification for downloads
  - [ ] Add fallback mechanism for download failures

### Asset Optimization

- [ ] Reduce asset size and improve loading performance
  - [ ] Strip unnecessary files from Grobid models
  - [ ] Implement optional model pruning for specific use cases
  - [ ] Add progressive loading for models based on usage

## Priority 3: JNI Safety Improvements

Ensuring robustness and preventing memory leaks or crashes.

### JNI Handle Safety

- [ ] Implement RAII pattern for JNI references
  - [ ] Create `JniHandle` wrapper with proper Drop implementation
  - [ ] Ensure exception_clear is called in Drop
  - [ ] Add debug assertions for reference validity
  - [ ] Update all JNI code to use the new safety wrapper

### Thread Safety

- [ ] Ensure thread-safety across JNI boundary
  - [ ] Run stress tests with multiple threads
  - [ ] Verify no deadlocks or race conditions
  - [ ] Add loom tests for concurrency verification
  - [ ] Document thread-safety guarantees

## Priority 4: CLI Polish

Enhancing user experience for command-line users.

### Shell Completions

- [ ] Generate shell completion scripts
  - [ ] Implement completion generation using clap_complete
  - [ ] Support Bash, Zsh, Fish, and PowerShell
  - [ ] Include completion scripts in distribution bundle
  - [ ] Add documentation on enabling completions

### Error Handling

- [ ] Improve CLI error messages and exit codes
  - [ ] Map GrobidError variants to specific exit codes
  - [ ] Document exit codes in help text
  - [ ] Add detailed error messages with suggestions
  - [ ] Implement --verbose flag for debug information

### User Experience

- [ ] Enhance overall CLI experience
  - [ ] Add progress indicators for long-running operations
  - [ ] Implement color output for better readability
  - [ ] Add examples to help text
  - [ ] Support config files for recurring options

## Priority 5: Release Automation

Streamlining the release process for maintainers and users.

### Distribution Task

- [ ] Implement xtask dist command
  - [ ] Create build script for release artifacts
  - [ ] Strip binaries for size optimization
  - [ ] Bundle runtime/, grobid/, completions, and README
  - [ ] Compress with zstd for efficient distribution
  - [ ] Generate checksums for verification

### CI Integration

- [ ] Configure CI for automated releases
  - [ ] Set up matrix builds for all supported platforms
  - [ ] Upload build artifacts on successful runs
  - [ ] Promote artifacts to releases on tag push
  - [ ] Add signing for official releases

## Priority 6: Documentation and Examples

Making the library more accessible to new users.

### API Documentation

- [ ] Improve API documentation
  - [ ] Convert examples into doctests
  - [ ] Add runnable examples for common use cases
  - [ ] Document all public API methods thoroughly
  - [ ] Create a quick-start guide

### User Guides

- [ ] Create comprehensive user guides
  - [ ] Write installation and setup instructions
  - [ ] Create tutorial for basic usage
  - [ ] Add advanced usage examples
  - [ ] Document known limitations and workarounds

## Future Enhancements (Post-MVP)

These items are important but not blocking for initial release.

### HTTP Server Implementation

- [ ] Implement HTTP API compatible with Grobid
  - [ ] Create Axum/Actix router mirroring servlet routes
  - [ ] Develop OpenAPI specification
  - [ ] Implement authentication and rate limiting
  - [ ] Add metrics and monitoring endpoints

### Daemon Mode

- [ ] Add background service capabilities
  - [ ] Implement service wrapper for various platforms
  - [ ] Create systemd, launchd, and Windows service units
  - [ ] Add configuration options for memory limits
  - [ ] Implement proper logging and rotation

### TUI Application ("papers/research")

- [ ] Develop terminal user interface
  - [ ] Create document browser interface
  - [ ] Implement search functionality
  - [ ] Add annotation capabilities
  - [ ] Support paper organization and tagging

---

## Milestones

### v0.1.0-beta (MVP Bundle)
- Core JNI engine complete
- JSON output implementation
- Downloadable assets support
- Basic CLI functionality
- Essential documentation

### v0.1.0 (First Stable Release)
- All JNI safety improvements
- Complete CLI polish with completions
- Comprehensive documentation
- Release automation

### v0.2.0
- HTTP server implementation
- Daemon mode
- Advanced error handling
- Performance optimizations

### v1.0.0
- Complete feature parity with Java Grobid
- TUI application
- Comprehensive test coverage
- Enterprise-ready deployment options