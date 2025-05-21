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

    /// Cache-related errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// XML parsing errors (general)
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Unexpected end of file during XML parsing
    #[error("Unexpected end of file while parsing {0}")]
    UnexpectedEof(String),

    /// XML parsing error with context
    #[error("XML parsing error in {context}: {message}")]
    XmlParseError { message: String, context: String },

    /// Malformed XML error
    #[error("Malformed XML: {message}. Expected {expected}, found {found}")]
    MalformedXml {
        message: String,
        expected: String,
        found: String,
    },

    /// JSON serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// JSON deserialization errors
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Generic conversion errors
    #[error("Conversion error: {0}")]
    Conversion(String),
}

impl From<Box<dyn std::error::Error>> for GrobidError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        GrobidError::InvalidInput(err.to_string())
    }
}

impl From<serde_json::Error> for GrobidError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_syntax() || err.is_data() {
            GrobidError::DeserializationError(err.to_string())
        } else {
            GrobidError::SerializationError(err.to_string())
        }
    }
}

impl From<quick_xml::Error> for GrobidError {
    fn from(err: quick_xml::Error) -> Self {
        GrobidError::ParseError(err.to_string())
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
