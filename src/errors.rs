use jni::errors::Error as JniError;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Grobid operations.
#[derive(Error, Debug)]
pub enum GrobidError {
    /// Grobid engine has not been initialized
    #[error("Grobid not initialised")]
    NotInitialised,

    /// JNI-related errors
    #[error("JNI error: {0}")]
    Jni(#[from] JniError),

    /// JVM initialization errors
    #[error("JVM initialization error: {0}")]
    JvmInitialization(String),

    /// Java exception errors
    #[error("Java exception: {0}")]
    Java(String),

    /// PdfAlto-related errors
    #[error("pdfalto failed: {0}")]
    PdfAlto(String),

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Version mismatch errors
    #[error("Grobid version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

impl From<Box<dyn std::error::Error>> for GrobidError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        GrobidError::InvalidInput(err.to_string())
    }
}

// Convenience methods for creating specific error types
impl GrobidError {
    /// Create a new invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        GrobidError::InvalidInput(msg.into())
    }

    /// Create a new file not found error
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        GrobidError::InvalidInput(format!("File not found: {}", path_buf.display()))
    }

    /// Create a new version mismatch error
    pub fn version_mismatch(expected: impl Into<String>, found: impl Into<String>) -> Self {
        GrobidError::VersionMismatch {
            expected: expected.into(),
            found: found.into(),
        }
    }
}
