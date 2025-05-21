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

### P0 (High Priority / Quick Wins: Estimated 1-2 days development time)

- [ ] **CLI Usability: Sub-commands & Output Formats**
    *   **Task:** Refactor CLI to use positional verbs for processing types and flags for output formats.
    *   **Why / Benefit:** Shorter, more intuitive commands (e.g., `grobid-cli header <PDF> --json`). Verbs can determine default output formats (e.g., `refs` defaults to BibTeX). Aligns with Grobid's own service endpoints.
    *   **Quick sketch (using `clap`):**
        ```rust
        // #[derive(Parser)]
        // struct Cli {
        //     #[clap(subcommand)]
        //     command: Commands,
        //     // ... other global options like assets path, GROBID_RS_RUNTIME_PATH override
        // }

        // #[derive(Subcommand)]
        // enum Commands {
        //     Header { pdf_file: PathBuf, #[clap(long, default_value_t = OutputFormat::Tei)] output_format: OutputFormat },
        //     Fulltext { pdf_file: PathBuf, #[clap(long, default_value_t = OutputFormat::Tei)] output_format: OutputFormat },
        //     References { pdf_file: PathBuf, #[clap(long, default_value_t = OutputFormat::Bibtex)] output_format: OutputFormat },
        //     // Process { // Alternative: single process command with type and output_format flags
        //     //     #[clap(long, value_enum, default_value_t = ProcessType::Fulltext)]
        //     //     r#type: ProcessType,
        //     //     pdf_file: PathBuf,
        //     //     #[clap(long, value_enum, default_value_t = OutputFormat::Tei)]
        //     //     output_format: OutputFormat,
        //     // },
        // }
        // #[derive(ValueEnum, Clone, Debug)]
        // enum OutputFormat { Tei, Json, Bibtex }
        // // enum ProcessType { Fulltext, Header, References, PatentCitation }
        ```
- [x] **JNI Method Signature Fix for GROBID 0.8.2**
    *   **Task:** Fix JNI method signature mismatch in `processReferences` to match GROBID 0.8.2 API.
    *   **Why / Benefit:** Resolves `NoSuchMethodError` by updating from `processReferences(String, BiblioItem)` to `processReferences(File, int)` and correctly handling the return type conversion from `List<BibDataSet>` to TEI string.
- [x] **Library API: Config Builder**
    *   **Task:** Implement `GrobidConfig::builder()` exposing `GrobidAnalysisConfig` options safely.
    *   **Why / Benefit:** Keeps high-level API stable, allows advanced configuration (coordinates, consolidation) without proliferating function arguments.
    *   **Implementation:**
        *   Builder methods for common `GrobidAnalysisConfig` fields (e.g., `consolidate_header`, `include_coordinates`, `segment_sentences`).
        *   `GrobidConfigBuilder::finish()` method that creates and returns a `jni::objects::GlobalRef<JObject>` for the Java `GrobidAnalysisConfig` instance. This instance can be cached and reused for multiple calls within a session to save JNI object creation overhead (~4ms per call).
    *   **Quick sketch:**
        ```rust
        // // In lib.rs/config.rs
        // pub struct GrobidConfig { /* internal fields, potentially the GlobalRef to GrobidAnalysisConfig */ }
        // pub struct GrobidConfigBuilder { /* fields for options */ }
        // impl GrobidConfigBuilder {
        //     pub fn consolidate_header(mut self, val: bool) -> Self { /* ... */ }
        //     pub fn include_coordinates(mut self, val: bool) -> Self { /* ... */ }
        //     pub fn build(self) -> Result<GrobidConfig, GrobidError> { /* Creates Java GrobidAnalysisConfig, stores as GlobalRef */ }
        // }
        // // Usage:
        // // let config = GrobidConfig::builder().consolidate_header(true).build()?;
        // // grobid_rs::process_fulltext(pdf_path, &config, OutputFormat::Tei);
        ```
- [x] **Error Handling: Proper Error Taxonomy**
    *   **Task:** Implement `GrobidError` using `thiserror`, with specific variants and `std::error::Error + Send + Sync`.
    *   **Why / Benefit:** Clear, categorizable errors (e.g., `JvmError`, `GrobidProcessingError`, `InvalidInputError`). Smooth interop with `anyhow`.
    *   **Implementation:**
        *   Define top-level `GrobidError` enum.
        *   Re-export typed sub-errors (e.g., `JvmError`, `JavaError`, `PdfAltoError`) that also implement `std::error::Error`. This allows callers to match specific error types while still using the umbrella `GrobidError`.
    *   **Quick sketch (in `lib.rs`/`errors.rs`):**
        ```rust
        // #[derive(Debug, thiserror::Error)]
        // pub enum JvmError { /* ... */ }
        // #[derive(Debug, thiserror::Error)]
        // pub enum GrobidProcessingError { /* ... */ }

        // #[derive(Debug, thiserror::Error)]
        // pub enum GrobidError {
        //     #[error("JVM interaction error: {0}")]
        //     Jvm(#[from] JvmError),
        //     #[error("Grobid engine processing failed: {0}")]
        //     Processing(#[from] GrobidProcessingError),
        //     #[error("Invalid input: {0}")]
        //     Input(String),
        //     #[error("IO error: {0}")]
        //     Io(#[from] std::io::Error),
        //     // ... other error variants
        // }
        ```
- [ ] **Core: Version Guard**
    *   **Task:** At runtime, check `grobid.properties.version` in `grobid-home` against compile-time `GROBID_VERSION`.
    *   **Why / Benefit:** Prevents cryptic Java errors if mismatched `grobid-core` and `grobid-home` are used. Error out early with a clear message.

### P1 (Medium Priority / Weekend Project)

- [ ] **CLI UX: Progress Reporting**
    *   **Task:** Add progress bar/spinner in CLI (via `indicatif`) for JVM init and per-PDF processing.
    *   **Why / Benefit:** Visual feedback for long operations (Grobid warm-up, large PDFs), avoids perception of a hung application.
    *   **Implementation:** Wrap `grobid_rs::init()` and PDF processing calls with `indicatif::ProgressBar` or `ProgressBar::spinner()` when stdin is a TTY.
- [x] **Performance: Batch Mode & Thread Pool**
    *   **Task:** Implement `grobid-cli process ... dir/ --jobs N` and library support for parallel processing.
    *   **Why / Benefit:** Significant speed-up for processing multiple PDFs. Grobid models are generally thread-safe.
    *   **Implementation:**
        *   Use `rayon::ThreadPoolBuilder::new().num_threads(cfg.thread_count).stack_size(4 << 20).build_global()` in `grobid_rs::init_with_config` or similar central place. Default `--jobs` to `num_cpus / 2` to avoid full saturation.
        *   Provide `parallel_fulltext(dir)` (and similar for other operations) convenience functions in the library.
        *   Ensure JNI context (`JNIEnv`) is correctly handled per thread (usually by attaching/detaching threads if they are spawned outside of JNI's knowledge, or by ensuring each Rayon task gets its own `JNIEnv`).
- [x] **Library API: Streaming API for Batch Processing**
    *   **Task:** Implement `fn process_batch<P: AsRef<Path>>(paths: impl Iterator<Item = P>, config: &GrobidConfig, ...) -> impl Iterator<Item = Result<(P, String), GrobidError>>`.
    *   **Why / Benefit:** Memory-friendly for very large batches; allows consuming results incrementally.
    *   **Implementation:** Internally use the thread pool.
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

- [x] **Performance: Cache Layer**
    *   **Task:** Implement caching for processed outputs (`--skip-existing`, `--force-reprocess`).
    *   **Why / Benefit:** Huge speed-up on re-runs (CI, development).
    *   **Implementation:**
        *   Cache key: SHA-256 of PDF content + Grobid version. Don't bake full config into key to maximize cache hits.
        *   Store outputs per-kind: `<hash>.tei`, `<hash>.json`.
        *   If config flags differ significantly, user passes `--force-reprocess`.
        *   Library API: `fn fulltext_cached<P: AsRef<Path>>(pdf: P, cache_dir: P) -> Result<PathBuf>` that writes TEI to `cache_dir/<sha>.tei`.
        *   ✅ Added cache pruning functionality to prevent unbounded cache growth.
- [ ] **Library API: Serde Structs for JSON Output**
    *   **Task:** Provide Serde structs for common Grobid outputs (header, citations) and functions to deserialize into them.
    *   **Why / Benefit:** Improves DX for Rust consumers; type-safe access to data.
    *   **Implementation:**
        *   Define Rust structs (e.g., `HeaderMetadata`, `Author`).
        *   Use Grobid's built-in JSON converters if available (e.g., `TEIConverter` on JVM side, or `HeaderResult.builder().withJson(true)`).
        *   Expose functions like `process_header_json(pdf_path) -> Result<HeaderMetadata, GrobidError>`.
- [ ] **Observability: Logging Hooks (`tracing`)**
    *   **Task:** Integrate `tracing` crate for structured logging in the library.
    *   **Why / Benefit:** Essential for diagnosing JNI issues, Grobid behavior.
    *   **Implementation:** Add `tracing::info!`, `tracing::debug!` at key points. CLI uses `tracing-subscriber` to set verbosity (`RUST_LOG=grobid_rs=info`).
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
- [ ] **Packaging: Feature Flags for Slimmer Binaries**
    *   **Task:** Introduce Cargo features like `pdfalto` (default), `http-fallback`, `json-output`, `benchmark`.
    *   **Why / Benefit:** Allows users to build smaller binaries if they don't need all functionality.
    *   **Implementation:** Use `cargo bloat` to measure impact.
- [ ] **Core: Graceful Shutdown**
    *   **Task:** Provide `pub fn shutdown()` to detach threads and potentially signal JVM to exit.
    *   **Why / Benefit:** For embedded applications wanting to unload Grobid resources.
    *   **Implementation:** Detach threads. For JVM exit, consider `System.exit(0)` in a daemon thread (won't fully free all memory from host OS perspective but stops Grobid's pools).

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

## Technical Challenges
*   JNI Stability & Complexity
*   Cross-Platform Compatibility & Build System for Bundles
*   Resource Management (for lean bundles)
*   Build Process & Dependencies (managing pre-built vs. source-built assets)

## Development Approach

### Testing Strategy
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