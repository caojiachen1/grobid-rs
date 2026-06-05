use std::path::{Path, PathBuf};
use std::fs;

/// Expected Grobid version that this library is built against
pub const EXPECTED_GROBID_VERSION: &str = "0.9.1";

/// Error type for version checking
#[derive(Debug, thiserror::Error)]
pub enum VersionCheckError {
    #[error("Grobid properties file not found at {0}")]
    PropertiesFileNotFound(PathBuf),
    
    #[error("Failed to read Grobid properties file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Version information not found in Grobid properties")]
    VersionNotFound,
    
    #[error("Version mismatch: expected {expected}, found {actual}")]
    VersionMismatch {
        expected: String,
        actual: String,
    },
}

/// Extracts the Grobid version from the properties file
pub fn extract_grobid_version(grobid_home: &Path) -> Result<String, VersionCheckError> {
    let properties_path = grobid_home.join("config/grobid.properties");
    
    if !properties_path.exists() {
        return Err(VersionCheckError::PropertiesFileNotFound(properties_path));
    }
    
    let content = fs::read_to_string(&properties_path)?;
    
    // Look for version line in format: grobid.version=X.Y.Z
    content
        .lines()
        .find(|line| line.starts_with("grobid.version="))
        .and_then(|line| line.strip_prefix("grobid.version="))
        .map(|version| version.trim().to_string())
        .ok_or(VersionCheckError::VersionNotFound)
}

/// Checks if the Grobid installation is compatible with this library
pub fn check_grobid_version(grobid_home: &Path) -> Result<(), VersionCheckError> {
    let actual_version = extract_grobid_version(grobid_home)?;
    
    if actual_version != EXPECTED_GROBID_VERSION {
        return Err(VersionCheckError::VersionMismatch {
            expected: EXPECTED_GROBID_VERSION.to_string(),
            actual: actual_version,
        });
    }
    
    Ok(())
}

/// Verifies Grobid version compatibility, returning a user-friendly error message
/// if versions don't match
pub fn verify_version_compatibility(grobid_home: &Path) -> Result<(), String> {
    match check_grobid_version(grobid_home) {
        Ok(_) => Ok(()),
        Err(VersionCheckError::VersionMismatch { expected, actual }) => {
            Err(format!(
                "Grobid version mismatch: this library expects version {}, but found {}. \
                This may cause subtle bugs or crashes. Please either upgrade Grobid to {}, \
                or use a compatible version of this library.",
                expected, actual, expected
            ))
        }
        Err(e) => Err(format!("Failed to verify Grobid version: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    
    #[test]
    fn test_version_extraction() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        
        let properties_path = config_dir.join("grobid.properties");
        let mut file = fs::File::create(&properties_path).unwrap();
        
        // Create a test properties file
        writeln!(file, "# Grobid properties\ngrobid.version=0.9.1\nsome.other.property=value").unwrap();
        
        // Test successful extraction
        let version = extract_grobid_version(temp_dir.path()).unwrap();
        assert_eq!(version, "0.9.1");
        
        // Test version check
        assert!(check_grobid_version(temp_dir.path()).is_ok());
        
        // Test version mismatch
        let mut file = fs::File::create(&properties_path).unwrap();
        writeln!(file, "# Grobid properties\ngrobid.version=0.7.0\nsome.other.property=value").unwrap();
        
        let result = check_grobid_version(temp_dir.path());
        assert!(result.is_err());
        match result {
            Err(VersionCheckError::VersionMismatch { expected, actual }) => {
                assert_eq!(expected, EXPECTED_GROBID_VERSION);
                assert_eq!(actual, "0.7.0");
            }
            _ => panic!("Expected VersionMismatch error"),
        }
    }
}