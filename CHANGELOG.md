# Changelog

All notable changes to grobid-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

No unreleased changes yet.

## [0.1.0] - 2025-06-01

### Added

#### Core Library
- Implemented version checking mechanism to ensure compatibility between library and Grobid version
- Added proper error taxonomy using `thiserror` with specific error variants
- Created configuration builder pattern with `GrobidConfig::builder()`
- Fixed JNI method signature for GROBID 0.8.2 API compatibility
- Implemented batch processing mode with thread pool support
- Added streaming API for memory-efficient batch processing
- Implemented comprehensive caching layer with:
  - Cache key generation based on file characteristics
  - Automatic cache pruning to prevent unbounded growth
  - Statistics tracking for cache hits/misses
  - Environment variable configuration options

#### CLI
- Redesigned CLI with subcommands (Header, Fulltext, References) and output format flags
- Added appropriate default output formats for different commands
- Implemented comprehensive help text and usage examples

#### Performance
- Added thread pool support for parallel processing
- Implemented configurable thread count

#### CI/Build System
- Optimized CI pipeline with cargo-chef for dependency caching
- Configured fast linkers (mold on Linux, default on macOS, lld on Windows)
- Implemented smart test execution with cargo-nextest
- Added Grobid artifact sharing across CI jobs
- Configured GitHub Actions caching for faster builds (10-15× speedup)
- Added git hooks for code quality enforcement

### Changed
- Updated mozilla-actions/sccache-action configuration to use stable version
- Switched to GitHub Actions cache for sccache

### Fixed
- Fixed sccache stats reporting to prevent job failures
- Fixed macOS linker configuration for better compatibility

### Security
- Implemented proper JNI error handling to prevent JVM crashes
- Added validation of paths for security