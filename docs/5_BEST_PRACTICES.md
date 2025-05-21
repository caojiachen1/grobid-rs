# Best Practices for grobid-rs

## 1. Introduction

This document outlines recommended best practices for working with the grobid-rs library. Following these guidelines will help you create robust, maintainable, and performant applications that efficiently integrate with Grobid's document processing capabilities.

## 2. Development Setup

### 2.1. Environment Setup

- **JDK Version:** Use JDK 11 or higher with a properly set `JAVA_HOME` environment variable
- **Rust Toolchain:** Use the stable Rust toolchain (1.65.0 or higher)
- **Development Tools:**
  - Install `cargo-expand` to debug procedural macros
  - Use `cargo clippy` for linting
  - Enable Rust Analyzer in your IDE for better JNI type checking

### 2.2. Project Structure

- Maintain a clear separation between JNI code and business logic
- Use feature flags to separate CLI functionality from the core library
- Keep build scripts (`build.rs`) focused only on resource acquisition and JNI setup

## 3. API Design Principles

### 3.1. Public Interface

- **Minimize JNI Exposure:** Don't expose JNI types in your public API
- **Idiomatic Rust:** Use Rust types and error handling patterns
- **Type Safety:** Prefer strong typing over generic string returns
- **Consistency:** Maintain consistent naming and parameter patterns

```rust
// Prefer this:
pub fn process_header(pdf_path: &Path) -> Result<HeaderData, GrobidError>

// Over this:
pub fn process_header(pdf_path: &str) -> Result<String, GrobidError>
```

### 3.2. Configuration Options

- Use a builder pattern for Grobid configuration options
- Provide sensible defaults for all configuration parameters
- Document all configuration options clearly
- Validate configuration values early

```rust
// Builder pattern example
pub fn fulltext_to_tei(pdf_path: &Path, config: GrobidConfig) -> Result<TeiDocument, GrobidError> {
    // ...
}

let result = grobid.fulltext_to_tei(
    &path,
    GrobidConfig::builder()
        .consolidate_citations(true)
        .include_raw_citations(false)
        .build()?
);
```

### 3.3. Return Types

- Return structured types rather than raw XML strings where possible
- Use types that facilitate further processing, like `serde`-compatible structures
- Provide methods to access both structured data and raw XML when needed

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct TeiDocument {
    title: String,
    authors: Vec<Author>,
    abstract_text: Option<String>,
    body_text: Vec<Section>,
    // Additional fields...
    
    // Raw XML accessor
    pub fn as_xml(&self) -> &str {
        &self.raw_xml
    }
}
```

## 4. Error Handling

### 4.1. Error Types

- Define a comprehensive error type that covers all failure scenarios
- Include context in error messages to facilitate debugging
- Wrap underlying errors rather than losing information

```rust
#[derive(thiserror::Error, Debug)]
pub enum GrobidError {
    #[error("Grobid not initialized")]
    NotInitialized,
    
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
    
    #[error("JVM initialization error: {0}")]
    JvmInitialization(String),
    
    #[error("Java exception: {0}")]
    Java(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("pdfalto error: {0}")]
    PdfAlto(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
}
```

### 4.2. Exception Handling

- Always check for Java exceptions after JNI calls
- Clear exceptions before continuing execution
- Convert Java exceptions to appropriate Rust errors
- Include Java stack traces when debugging is enabled

### 4.3. Recovery Strategies

- Implement retries for transient failures
- Create cleanup routines to handle partially completed operations
- Log errors at appropriate levels (debug, info, warning, error)
- Provide clear guidance to users on how to resolve common errors

## 5. Performance Optimization

### 5.1. JNI Overhead Reduction

- Cache class and method references to reduce lookups
- Minimize string conversions between Java and Rust
- Batch operations when calling Java methods
- Use direct buffers for large data transfers

### 5.2. Parallel Processing

- Process multiple documents concurrently when appropriate
- Ensure thread safety when accessing JVM resources
- Use a thread pool with sensible defaults based on available cores
- Provide control over concurrency levels

```rust
pub fn process_batch(
    pdfs: &[PathBuf], 
    config: &GrobidConfig,
    max_threads: Option<usize>,
) -> Result<Vec<TeiDocument>, GrobidError> {
    let threads = max_threads.unwrap_or_else(|| std::cmp::min(8, num_cpus::get()));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()?;
    
    pool.install(|| {
        pdfs.par_iter()
            .map(|pdf| fulltext_to_tei(pdf, config.clone()))
            .collect()
    })
}
```

### 5.3. Memory Management

- Set appropriate JVM heap size based on document size and batch processing needs
- Monitor and limit memory usage during large batch processes
- Clean up temporary files after processing
- Provide memory usage indicators or progress feedback for long-running operations

## 6. Testing

### 6.1. Test Coverage

- Unit test all Rust functionality
- Integration test JNI interactions with sample PDFs
- Test error handling and edge cases
- Test resource cleanup
- Test cross-platform compatibility

### 6.2. Test Resources

- Include small test PDFs in your test resources
- Mock Java interactions for unit tests
- Create fixtures for testing different document types
- Add performance benchmarks

### 6.3. CI/CD

- Run tests on all supported platforms (Linux, macOS, Windows)
- Include security scanning in CI/CD pipeline
- Automate release testing
- Generate documentation as part of the build process

## 7. Documentation

### 7.1. API Documentation

- Document all public functions with examples
- Include parameter descriptions and return value explanations
- Document possible error cases
- Use doc tests to verify examples

### 7.2. Architecture Documentation

- Explain the JNI integration approach
- Document the relationship between Rust and Java components
- Include sequence diagrams for complex interactions
- Document memory management considerations

### 7.3. User Guides

- Provide getting started guides
- Include troubleshooting sections
- Document common workflows and use cases
- Add performance tuning recommendations

## 8. Deployment

### 8.1. Resource Bundling

- Create a reproducible build process for all platforms
- Generate platform-specific bundles with correct native libraries
- Verify checksums of all downloaded or included resources
- Document installation requirements clearly

### 8.2. Version Management

- Use semantic versioning
- Document compatibility with Grobid versions
- Provide migration guides for major version updates
- Include changelogs with each release

### 8.3. Environment Configuration

- Document required environment variables
- Provide sample configurations for different environments
- Include default logging configuration
- Offer guidance on production deployment

## 9. Security

### 9.1. Input Validation

- Validate all user-provided inputs before processing
- Set size limits for PDF files
- Verify file format before processing
- Sanitize file paths to prevent path traversal attacks

### 9.2. Resource Protection

- Sandbox external tool execution (like pdfalto)
- Clean up temporary files securely
- Don't expose sensitive configuration in error messages
- Use secure temporary directories

### 9.3. Dependency Management

- Keep dependencies updated
- Use `cargo audit` to check for security vulnerabilities
- Pin dependency versions for reproducible builds
- Verify integrity of downloaded resources

## 10. Community and Contribution

### 10.1. Code Style

- Follow Rust community conventions
- Use `rustfmt` and `clippy` on all code
- Write descriptive commit messages
- Add test cases for all new features and fixes

### 10.2. Issue Management

- Use issue templates for bug reports and feature requests
- Tag issues appropriately (bug, enhancement, documentation)
- Link pull requests to related issues
- Document decision-making processes

### 10.3. Contribution Process

- Provide clear contribution guidelines
- Review all pull requests promptly
- Recognize and acknowledge contributions
- Ensure all code meets quality and security standards

## 11. Conclusion

Following these best practices will help you create robust, performant, and maintainable applications with grobid-rs. As you gain experience with the library, you may develop additional patterns and practices that work well for your specific use cases.

Remember that these are guidelines, not strict rules. Adapt them to your project's specific needs while maintaining the core principles of reliability, performance, and maintainability.