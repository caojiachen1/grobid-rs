# grobid-rs Development Roadmap

This document outlines the development plan and roadmap for grobid-rs, a Rust library providing JNI bindings to Grobid.

## Project Goals

1.  Provide a robust, idiomatic Rust interface to Grobid.
2.  Ensure cross-platform compatibility (Linux, macOS, Windows).
3.  Optimize for both ease of use and performance.
4.  Maintain compatibility with Grobid's version evolution.
5.  Achieve distribution and ease-of-use parity with containerized solutions for end-users.

## Next Iteration Roadmap (Actionable Steps)

This section outlines a concrete “next-iteration” roadmap that touches both the public library API and the CLI UX, ranked by impact and complexity.

### P0 (High Priority / Quick Wins)

- [x] **CLI Usability: Sub-commands & Output Formats**
    *   **Completed:** Implemented CLI with subcommands (Header, Fulltext, References) and appropriate output format flags
    *   **Details:** Created an ergonomic CLI interface using clap with proper documentation and examples. Each command has contextually appropriate default output formats (e.g., BibTeX for References).

- [x] **Core: Version Guard**
    *   **Completed:** Implemented version check that verifies `grobid.properties.version` matches the expected version
    *   **Details:** Created `version_check.rs` module with dedicated error types and user-friendly error messages. Configuration validation now includes version compatibility checks.

### P1 (Medium Priority / Weekend Project)

- [x] **CLI UX: Progress Reporting**
    *   **Completed:** Added progress indicators for lengthy operations like downloads and processing.
    *   **Details:** Implemented indicatif progress bars in source_ops.rs for downloads and extractions, providing visual feedback during long-running tasks.

- [ ] **Core: Safe JNI Guard (`JniHandle`)**
    *   **Task:** Create a `JniHandle<'a> { env: JNIEnv<'a>, engine: JObject<'a> }` struct.
    *   **Why / Benefit:** Simplifies JNI calls and makes them safer. `attach()` method returns this handle. `Deref<Target=JNIEnv>` for easy access to `env`. RAII `Drop` implementation clears pending Java exceptions automatically. Reduces boilerplate and risk of forgetting `exception_clear()`.
    *   **Implementation:**
        ```rust
        // pub struct JniHandle<'a> {
        //     env: ManuallyDrop<JNIEnv<'a>>, // To control drop order with engine
        //     engine: ManuallyDrop<JObject<'a>>, // Or GlobalRef if engine is long-lived
        //     // Potentially other global refs like GrobidFactory instance
        // }
        // impl<'a> Drop for JniHandle<'a> {
        //     fn drop(&mut self) {
        //         unsafe {
        //             if self.env.exception_check().unwrap_or(false) {
        //                 self.env.exception_describe().unwrap_or_default(); // Log it
        //                 self.env.exception_clear().unwrap_or_default();
        //             }
        //             // Manually drop fields if necessary, in correct order
        //             ManuallyDrop::drop(&mut self.engine);
        //             ManuallyDrop::drop(&mut self.env);
        //         }
        //     }
        // }
        // impl<'a> Deref for JniHandle<'a> { /* target = JNIEnv */ }
        ```

### P2 (Lower Priority / Enhancements)

- [x] **Performance: Batch Mode & Thread Pool**
    *   **Completed:** Implemented configurable thread pool for parallel processing.
    *   **Details:** Added thread_count configuration option, rayon dependency, and parallel feature flag for efficient batch processing of multiple PDFs.

- [x] **Library API: Streaming API for Batch Processing**
    *   **Completed:** Created memory-efficient API for processing large batches of PDFs.
    *   **Details:** Implemented streaming interface that allows consuming results incrementally rather than loading everything into memory.

- [x] **Performance: Cache Layer**
    *   **Completed:** Implemented caching for processed outputs with automatic pruning.
    *   **Details:** Created cache.rs and cache_prune.rs modules with comprehensive features including hit/miss tracking, size management, and file-based storage of processing results.

- [ ] **Library API: Serde Structs for JSON Output**
    *   **Task:** Provide Serde structs for common Grobid outputs (header, citations) and functions to deserialize into them.
    *   **Why / Benefit:** Improves DX for Rust consumers; type-safe access to data.
    *   **Implementation:**
        *   Define Rust structs (e.g., `HeaderMetadata`, `Author`).
        *   Use Grobid's built-in JSON converters if available (e.g., `TEIConverter` on JVM side, or `HeaderResult.builder().withJson(true)`).
        *   Expose functions like `process_header_json(pdf_path) -> Result<HeaderMetadata, GrobidError>`.

- [x] **Observability: Logging Hooks (`tracing`)**
    *   **Completed:** Integrated tracing crate for structured logging in cache and other modules.
    *   **Details:** Added tracing::debug!, tracing::trace!, etc. in cache_prune.rs and other components. Provides better diagnostics for maintenance and debugging.

- [ ] **CLI: Standardized Exit Codes**
    *   **Task:** Map `GrobidError` variants to `std::process::ExitCode` for machine-readable scriptability.
    *   **Why / Benefit:** Allows scripts to reliably determine outcomes.
    *   **Implementation:** E.g., Invalid PDF = 100, JVM init error = 101, etc.

- [ ] **Distribution: Auto-download pre-built asset bundle on first run (CLI)**
    *   **Task:** If CLI detects missing assets (e.g., when installed via `cargo install` without a bundled release), offer to download a pre-built asset bundle.
    *   **Why / Benefit:** Improves "out-of-the-box" experience for users not using full release bundles. Mimics tools like `deno` or `rust-analyzer`.
    *   **Implementation:** Check for assets; if missing, prompt user (or use a flag), show spinner, download pre-built bundle (from GitHub Releases) to a standard cache location (e.g., `$XDG_CACHE_HOME/grobid-rs`).

### P3 (Future Polish / Advanced)

- [ ] **Packaging: CLI Completions, Man Pages, and System Packages**
    *   **Task:** Generate CLI completions (`clap_complete`), man pages (`clap_mangen`), and create system packages.
    *   **Why / Benefit:** Improves CLI usability, essential for polished packaging and wider adoption.
    *   **Implementation:**
        *   Use `build.rs` to emit completions and man pages.
        *   Investigate and implement packaging for `.deb` (Debian/Ubuntu), `.rpm` (Fedora/CentOS), Homebrew (macOS), Scoop (Windows).
- [ ] **Resilience: HTTP Fallback (Feature Gated)**
    *   **Task:** Optionally call a remote Grobid URL if local JVM/Grobid init fails.
    *   **Why / Benefit:** Fallback for unsupported architectures or users unable to set up local JVM.
    *   **Implementation:** `#[cfg(feature = "http-fallback")]` module using `reqwest`. Requires clear user consent.
- [ ] **Convenience: Self-Update Mechanism**
    *   **Task:** `grobid-cli upgrade [--nightly]` to fetch new binary + assets.
    *   **Why / Benefit:** Easier updates for users not familiar with `cargo install` or manual bundle downloads.
    *   **Implementation:** Use `self_update` crate. Requires hosting release binaries/assets.
- [x] **Packaging: Feature Flags for Slimmer Binaries**
    *   **Completed:** Implemented feature flags for modular functionality
    *   **Details:** Created features for 'cli', 'parallel', and other optional components. Allows users to build binaries with only needed functionality.
- [ ] **Core: Graceful Shutdown**
    *   **Task:** Provide `pub fn shutdown()` to detach threads and potentially signal JVM to exit.
    *   **Why / Benefit:** For embedded applications wanting to unload Grobid resources.
    *   **Implementation:** Detach threads. For JVM exit, consider `System.exit(0)` in a daemon thread (won't fully free all memory from host OS perspective but stops Grobid's pools).

- [x] **Distribution: Vendored Dependencies for Offline Builds**
    *   **Completed:** Implemented comprehensive vendoring system for offline builds
    *   **Details:** Created xtask/src/bin/vendor.rs utility to create minimal vendor bundles, implemented check_for_vendored_files() and use_vendored_files() in build.rs. Fully supports air-gapped environments.

## Broader Development Goals

### 1. Core Library & Build System Stability & Robustness

- [ ] **Code Structure: Modularize `lib.rs`**
    *   **Task:** Split monolithic `lib.rs` into smaller modules (e.g., `jvm.rs`, `engine.rs`, `api.rs`, `config.rs`, `errors.rs`).
    *   **Why / Benefit:** Improves maintainability, readability, and compile times.
- [ ] **JNI Interactions: Comprehensive Error Handling & Safety**
    *   **Task:** Systematically review all JNI call sites. Convert Java exceptions to specific `GrobidError` variants. Ensure Java exception stack traces are captured.
    *   - [x] **Fixed method signature mismatch in `processReferences`:** Updated the JNI call to match GROBID 0.8.2's method signature, changing from `processReferences(String, BiblioItem)` to `processReferences(File, int)` with correct return type conversion.
- [ ] **Memory Management: Audit JNI GlobalRefs**
    *   **Task:** Audit usage of JNI `GlobalRef`s, ensuring they are deleted when no longer needed.
- [ ] **Build System: Enhancements for Speed, Security, Portability, and Distribution**
    *   - [ ] **Simplify end-user setup by optionally downloading pre-built Grobid assets:** `build.rs` to detect if it should download a pre-compiled Grobid (one-jar + models + JRE) from CI artifacts instead of building from source, especially for users installing via `cargo install` or when a specific flag is set.
    *   - [ ] **Security of Gradle wrapper:** In `build_ops::run_gradle_build` verify `gradlew` checksum or use `gradle-wrapper-validation-action`.
    *   - [ ] **Gradle offline cache:** Set `GRADLE_USER_HOME=$ASSETS_DIR/.gradle_cache` and pass `--offline` on retries.
    *   - [ ] **Zip integrity check before SHA:** Use `zip::read::ZipArchive::new()` before hashing large zip files.
    *   - [ ] **Progress bar for Gradle build:** Pipe `gradlew` stdout and parse `> Task` lines for `indicatif`.
    *   - [ ] **Rpath portability:** Only emit `-Wl,-rpath,...` on Linux in `jni_config.rs`.
    *   - [x] **Dynamic lib look-up at runtime:** Set `DYLD_FALLBACK_LIBRARY_PATH` (macOS) / `PATH` (Windows) to include `jni_lib_path` at runtime.
    *   - [x] **Parallel unzip:** Switch to `zip_extract` crate for multithreaded decompression.
    *   - [x] **Resume partially extracted source:** Use a `.partial` marker file during unzip.
    *   - [ ] **Checksum cache for downloads:** Store `<zip>.sha256.ok` after successful verify; check mtime on next build.
    *   - [ ] **Graceful fallback for proxies in download:** Detect `HTTPS_PROXY` and configure `reqwest::Proxy::https()`.
    *   - [ ] **Validate jlink output:** After `jre_ops::build_jlink_runtime`, check `runtime/bin/java -version`.
    *   - [ ] **Feature-gated vendor path:** Gate `check_for_vendored_files` behind a Cargo feature `vendored`.
    *   - [ ] **Rustfmt & Clippy in build scripts:** Add `#![warn(clippy::all, rust_2018_idioms)]` to `build_modules/*.rs`.
- [ ] **CI/CD Pipeline:**
    *   - [ ] **CI builds Grobid (one-jar + grobid-home) for inclusion in release bundles.** This step pre-compiles Grobid, removing the need for end-users to run Gradle.
    *   - [ ] **CI produces ready-to-use release bundles (`grobid-rs-VERSION-${target}.tar.zst`)** containing stripped `grobid-cli`, jlink'd `runtime/`, and the pre-built `grobid/` (one-jar + models).
    *   - [ ] Automate building, testing (including canary PDF), and releasing on Linux, macOS (x86_64, ARM64), Windows.
    *   - [ ] Cache Maven and jlink layers in GitHub Actions (`actions/cache@v4`) keyed on `GROBID_VERSION` and OS to cut build times.
    *   - [ ] CI produces `{os}-{arch}.zip` release bundles (binary + assets) nightly/on-tag via `cargo xtask dist`.
- [x] **Configuration: Runtime Path Override for Assets**
    *   **Task:** Allow `GROBID_RS_RUNTIME_PATH` (or similar env var) to override the compile-time `JLINK_RUNTIME_PATH` and other asset paths.
    *   **Why / Benefit:** Flexibility for users who relocate assets after build. Use env var first, fall back to compile-time const.

### 2. Performance Optimization & Resource Management

- [x] **Configurable JVM Memory Settings:** Expose `-Xms`, `-Xmx` via `GrobidConfig` or env vars.
- [ ] **Benchmark and Optimize JNI Overhead:** Use `criterion.rs`. Profile hotspots.
- [ ] **Upgrade Mechanism for Bundled Grobid Version:** Streamline `build.rs` for new Grobid releases.
- [x] **Reduced Disk Footprint Options:** Investigate selective model inclusion, optimize jlink JRE. (Bundling a jlink'd JRE makes JDK installation unnecessary for end-users).

### 3. API Refinement & Usability

- [ ] **Support for All Core Grobid Processing Methods:** Systematically add functions for `processDate`, `processAuthor`, etc.
- [ ] **Official Async API Support (Experimental):**
    *   **Task:** Offer `async fn` versions of processing functions.
    *   **Why / Benefit:** Integration into async Rust applications.
    *   **Implementation:** Start with `tokio::task::spawn_blocking` wrapper as an experimental "async façade" to ship quickly and gauge demand.
- [ ] **Integration Guides for Common Rust Frameworks:** Examples for Actix, Axum, etc.

### 4. Security Hardening

- [ ] **Sandbox External Processes:** Run `pdfalto` (if used directly) with `seccomp` (Linux) or `sandbox-exec` (macOS) when available. Implement timeouts.
- [ ] **Secure Temporary Directory:** After JVM init, call `System.setProperty("java.io.tmpdir", "<project_cache_dir>/tmp")` so untrusted PDFs aren't unpacked in shared `/tmp`.

### 5. Documentation & Examples

- [ ] **Comprehensive API Documentation:** `rustdoc` comments with examples.
- [ ] **User Guide & Cookbook:**
    *   - [ ] Detailed README and/or separate guides, highlighting ease of use with pre-built bundles (no JDK needed).
    *   - [ ] Create `examples/minimal.rs` and use it for `cargo doc --open` landing page examples via `/// ```rust,ignore` doc test blocks.
    *   - [ ] Cookbook examples: "Parse one PDF (using pre-built bundle)", "Batch directory", "CLI flags".
- [ ] **Contribution Guidelines:** `CONTRIBUTING.md`.
- [ ] **Licensing Information:**
    *   - [ ] Add SPDX license headers (e.g., `//! SPDX-License-Identifier: Apache-2.0 OR MIT`) in every Rust file.
    *   - [ ] Bundle licenses of dependencies (e.g., pdfalto, OpenJDK) in a `licenses/` folder within release bundles.
    *   - [ ] Add `about --licenses` flag in CLI.
    *   - [ ] Embed SPDX expressions in `Cargo.toml` `[package.license]`.

## Previously Completed Milestones (for context)

- [x] **Fix JNI classloader issues**
- [x] **Implement thread pooling for concurrent document processing (basic version)**
- [x] **ARM64 support (Apple Silicon, ARM Linux) (basic runtime confirmed)**
- [x] **Automatic Grobid resource downloading/management (source-based)**
- [x] **Example applications (CLI, batch_processing.rs)**
- [x] **Fixed JNI method signature mismatch in `processReferences` for GROBID 0.8.2 compatibility**
- [x] **Optimized CI pipeline with multi-stage dependency caching**
- [x] **Added Git hooks for code quality (auto-formatting and workflow validation)**
- [x] **Improved Grobid artifact caching and sharing across CI jobs**
- [x] **Fixed cache pruning mechanism for reliable operation**
- [x] **Added fast linkers (mold on Linux) for quicker build times**

## Technical Challenges
*   JNI Stability & Complexity
*   Cross-Platform Compatibility & Build System for Bundles
*   Resource Management (for lean bundles)
*   Build Process & Dependencies (managing pre-built vs. source-built assets)

## Development Approach

### Testing Strategy

## Completed Tasks

### P0 (High Priority / Quick Wins - Completed)

- [x] **CI Performance: Build & Test Optimization**
    * **Completed:** Implemented advanced CI pipeline with cargo-chef, faster linkers, and efficient artifact caching
    * **Benefit:** Reduced CI build times from ~8-12 minutes to ~2-3 minutes
    * **Features added:**
      * Multi-stage caching with cargo-chef for dependency separation
      * Fast linkers (mold on Linux)
      * Smart test execution with cargo-nextest
      * Grobid artifact sharing across jobs
      * Git hooks for code quality
      * Updated sccache to use GitHub Actions cache provider (v0.1.3)

- [x] **JNI Method Signature Fix for GROBID 0.8.2**
    * **Task:** Fix JNI method signature mismatch in `processReferences` to match GROBID 0.8.2 API.
    * **Why / Benefit:** Resolves `NoSuchMethodError` by updating from `processReferences(String, BiblioItem)` to `processReferences(File, int)` and correctly handling the return type conversion from `List<BibDataSet>` to TEI string.

- [x] **Library API: Config Builder**
    * **Task:** Implement `GrobidConfig::builder()` exposing `GrobidAnalysisConfig` options safely.
    * **Why / Benefit:** Keeps high-level API stable, allows advanced configuration (coordinates, consolidation) without proliferating function arguments.
    * **Implementation:**
        * Builder methods for common `GrobidAnalysisConfig` fields (e.g., `consolidate_header`, `include_coordinates`, `segment_sentences`).
        * `GrobidConfigBuilder::finish()` method that creates and returns a `jni::objects::GlobalRef<JObject>` for the Java `GrobidAnalysisConfig` instance. This instance can be cached and reused for multiple calls within a session to save JNI object creation overhead (~4ms per call).

- [x] **Error Handling: Proper Error Taxonomy**
    * **Task:** Implement `GrobidError` using `thiserror`, with specific variants and `std::error::Error + Send + Sync`.
    * **Why / Benefit:** Clear, categorizable errors (e.g., `JvmError`, `GrobidProcessingError`, `Invali
- [x] **CI Testing Setup:** Optimized CI pipeline with cargo-nextest for faster test execution
- [x] **Git Hooks:** Pre-commit hooks for code quality (rustfmt, clippy, GitHub Actions workflow validation)
- [ ] **Unit Tests:** For individual Rust functions.
- [ ] **Integration Tests:**
    *   - [ ] **API Tests:** With diverse real PDFs.
    *   - [ ] **CLI Tests:** `assert_cmd` for `grobid-cli` (testing with bundled assets and potentially auto-downloaded assets).
    *   - [ ] **Golden File Testing:** `insta` crate for TEI/JSON/BibTeX outputs.
    *   - [ ] **Canary PDF Test:** A small, public-domain PDF checked into the repo, parsed on every CI run, asserting title/author via XPath or similar.
- [ ] **JNI-Specific Tests:** Concurrent access, error handling.
- [ ] **Cross-Platform Testing (CI):** Linux, macOS (x86_64/ARM64), Windows, testing the bundled distributions.
- [ ] **Fuzz Testing (Future Consideration):** For JNI boundaries.

*(Existing sections on Community Engagement, Compatibility Matrix, and Progress Tracking remain relevant)*