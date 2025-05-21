use std::env;
use std::path::{Path, PathBuf};

// Mock implementation for tests - in a real project, this would typically
// be behind a #[cfg(test)] feature flag in the main lib.rs code

#[derive(Debug, PartialEq, Clone)] // Added Clone
enum MockResult {
    Init(PathBuf),
    ProcessHeader(PathBuf),
    FulltextToTei(PathBuf),
    ProcessReferences(PathBuf),
    Error,
}

// Mock function to track what would be called, using thread_local for test isolation
#[cfg(test)]
thread_local! {
    static MOCK_RESULTS: std::cell::RefCell<Vec<MockResult>> = std::cell::RefCell::new(Vec::new());
}

// Reset the mock results
#[cfg(test)]
fn reset_mocks() {
    MOCK_RESULTS.with(|results| {
        results.borrow_mut().clear();
    });
}

// Get the current mock results
#[cfg(test)]
fn get_mock_results() -> Vec<MockResult> {
    MOCK_RESULTS.with(|results| results.borrow().clone())
}

// Mock implementation of the actual library functions for testing
#[cfg(test)]
mod mock_impl {
    use super::*;
    use crate::GrobidError;

    pub fn init(path: &Path) -> Result<(), GrobidError> {
        if path.exists() && path.is_dir() {
            MOCK_RESULTS.with(|results| {
                results
                    .borrow_mut()
                    .push(MockResult::Init(path.to_path_buf()));
            });
            Ok(())
        } else {
            MOCK_RESULTS.with(|results| {
                results.borrow_mut().push(MockResult::Error);
            });
            Err(GrobidError::JvmInitialization("Invalid path".to_string()))
        }
    }

    pub fn process_header(path: &Path) -> Result<String, GrobidError> {
        if path.exists() && path.is_file() {
            MOCK_RESULTS.with(|results| {
                results
                    .borrow_mut()
                    .push(MockResult::ProcessHeader(path.to_path_buf()));
            });
            Ok("<tei>Mock header result</tei>".to_string())
        } else {
            MOCK_RESULTS.with(|results| {
                results.borrow_mut().push(MockResult::Error);
            });
            Err(GrobidError::Java("File not found".to_string()))
        }
    }

    pub fn fulltext_to_tei(path: &Path) -> Result<String, GrobidError> {
        if path.exists() && path.is_file() {
            MOCK_RESULTS.with(|results| {
                results
                    .borrow_mut()
                    .push(MockResult::FulltextToTei(path.to_path_buf()));
            });
            Ok("<tei>Mock fulltext result</tei>".to_string())
        } else {
            MOCK_RESULTS.with(|results| {
                results.borrow_mut().push(MockResult::Error);
            });
            Err(GrobidError::Java("File not found".to_string()))
        }
    }

    pub fn process_references(path: &Path) -> Result<String, GrobidError> {
        if path.exists() && path.is_file() {
            MOCK_RESULTS.with(|results| {
                results
                    .borrow_mut()
                    .push(MockResult::ProcessReferences(path.to_path_buf()));
            });
            Ok("<tei>Mock references result</tei>".to_string())
        } else {
            MOCK_RESULTS.with(|results| {
                results.borrow_mut().push(MockResult::Error);
            });
            Err(GrobidError::Java("File not found".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GrobidError;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_DIR_ID: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let id = TEST_DIR_ID.fetch_add(1, Ordering::SeqCst);
            let pid = process::id();
            let suite_base_dir = env::temp_dir().join("grobid_rs_test_suite");

            fs::create_dir_all(&suite_base_dir)
                .expect("Failed to create suite base temp directory");

            let specific_tmp_dir = suite_base_dir.join(format!("pid_{}_run_{}", pid, id));

            if specific_tmp_dir.exists() {
                fs::remove_dir_all(&specific_tmp_dir).expect(&format!(
                    "Pre-cleaning: Failed to remove old temp dir {:?}",
                    specific_tmp_dir
                ));
            }

            fs::create_dir_all(&specific_tmp_dir).expect(&format!(
                "Failed to create specific temp dir {:?}",
                specific_tmp_dir
            ));
            TempDir {
                path: specific_tmp_dir,
            }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            if self.path.exists() {
                fs::remove_dir_all(&self.path).unwrap_or_else(|e| {
                    eprintln!("Cleanup: Failed to remove temp dir {:?}: {}", self.path, e)
                });
            }
        }
    }

    // Helper function to create a mock PDF file in a specific directory
    fn create_mock_pdf_in_dir(base_dir: &Path) -> PathBuf {
        let pdf_path = base_dir.join("test.pdf");
        fs::write(&pdf_path, b"%PDF-mock content").expect("Failed to write mock PDF");
        assert!(
            pdf_path.exists(),
            "Mock PDF file was not created successfully"
        );
        pdf_path
    }

    #[test]
    fn test_initialization() {
        reset_mocks();
        let temp_dir_guard = TempDir::new();
        let dir_path = temp_dir_guard.path();

        // Test successful initialization
        let result = mock_impl::init(dir_path);
        assert!(result.is_ok());

        // Test that the correct path was passed
        let mock_results = get_mock_results();
        assert_eq!(mock_results.len(), 1);
        assert_eq!(mock_results[0], MockResult::Init(dir_path.to_path_buf()));

        // Test error handling with non-existent directory
        reset_mocks();
        let invalid_dir = PathBuf::from("/path/that/does/not/exist");
        let result = mock_impl::init(&invalid_dir);
        assert!(result.is_err());

        if let Err(GrobidError::JvmInitialization(msg)) = result {
            assert!(msg.contains("Invalid path"));
        } else {
            panic!("Expected JvmInitialization error");
        }
    }

    #[test]
    fn test_process_header() {
        reset_mocks();
        let temp_dir_guard = TempDir::new();
        let dir_path = temp_dir_guard.path();
        let pdf = create_mock_pdf_in_dir(dir_path);
        println!("Created mock PDF at: {}", pdf.display());

        // Test successful processing
        let result = mock_impl::process_header(&pdf);
        assert!(result.is_ok(), "process_header failed for existing file");
        assert_eq!(result.unwrap(), "<tei>Mock header result</tei>");

        // Check that the correct function was called
        let mock_results = get_mock_results();
        assert_eq!(mock_results.len(), 1);
        assert_eq!(mock_results[0], MockResult::ProcessHeader(pdf.clone()));

        // Test error handling with non-existent file
        reset_mocks();
        let invalid_pdf = PathBuf::from("/path/to/nonexistent.pdf");
        let result = mock_impl::process_header(&invalid_pdf);
        assert!(result.is_err());

        if let Err(GrobidError::Java(msg)) = result {
            assert!(msg.contains("File not found"));
        } else {
            panic!("Expected Java error matching mock_impl's return for file not found");
        }
    }

    #[test]
    fn test_fulltext_to_tei() {
        reset_mocks();
        let temp_dir_guard = TempDir::new();
        let dir_path = temp_dir_guard.path();
        let pdf = create_mock_pdf_in_dir(dir_path);

        // Test successful processing
        let result = mock_impl::fulltext_to_tei(&pdf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<tei>Mock fulltext result</tei>");

        // Check that the correct function was called
        let mock_results = get_mock_results();
        assert_eq!(mock_results.len(), 1);
        assert_eq!(mock_results[0], MockResult::FulltextToTei(pdf.clone()));

        // Test error handling with non-existent file
        reset_mocks();
        let invalid_pdf = PathBuf::from("/path/to/nonexistent.pdf");
        let result = mock_impl::fulltext_to_tei(&invalid_pdf);
        assert!(result.is_err());
        if let Err(GrobidError::Java(msg)) = result {
            assert!(msg.contains("File not found"));
        } else {
            panic!("Expected Java error");
        }
    }

    #[test]
    fn test_process_references() {
        reset_mocks();
        let temp_dir_guard = TempDir::new();
        let dir_path = temp_dir_guard.path();
        let pdf = create_mock_pdf_in_dir(dir_path);

        // Test successful processing
        let result = mock_impl::process_references(&pdf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<tei>Mock references result</tei>");

        // Check that the correct function was called
        let mock_results = get_mock_results();
        assert_eq!(mock_results.len(), 1);
        assert_eq!(mock_results[0], MockResult::ProcessReferences(pdf.clone()));

        // Test error handling with non-existent file
        reset_mocks();
        let invalid_pdf = PathBuf::from("/path/to/nonexistent.pdf");
        let result = mock_impl::process_references(&invalid_pdf);
        assert!(result.is_err());
        if let Err(GrobidError::Java(msg)) = result {
            assert!(msg.contains("File not found"));
        } else {
            panic!("Expected Java error");
        }
    }

    #[test]
    fn test_processing_workflow() {
        reset_mocks();
        let temp_dir_guard = TempDir::new();
        let dir_path = temp_dir_guard.path();
        println!("Created temp dir at: {}", dir_path.display());
        assert!(dir_path.exists(), "Temp directory doesn't exist");
        assert!(dir_path.is_dir(), "Temp path is not a directory");

        let pdf = create_mock_pdf_in_dir(dir_path);
        println!("Created mock PDF at: {}", pdf.display());
        assert!(pdf.exists(), "Mock PDF doesn't exist");
        assert!(pdf.is_file(), "Mock PDF path is not a file");

        // Test a complete workflow
        let init_result = mock_impl::init(dir_path);
        assert!(
            init_result.is_ok(),
            "Failed to initialize: {:?}",
            init_result
        );

        let header_result = mock_impl::process_header(&pdf);
        assert!(
            header_result.is_ok(),
            "Failed to process header: {:?}",
            header_result
        );

        let fulltext_result = mock_impl::fulltext_to_tei(&pdf);
        assert!(
            fulltext_result.is_ok(),
            "Failed to process fulltext: {:?}",
            fulltext_result
        );

        let refs_result = mock_impl::process_references(&pdf);
        assert!(
            refs_result.is_ok(),
            "Failed to process references: {:?}",
            refs_result
        );

        // Check that all functions were called in order
        let mock_results = get_mock_results();
        assert_eq!(mock_results.len(), 4);
        assert_eq!(mock_results[0], MockResult::Init(dir_path.to_path_buf()));
        assert_eq!(mock_results[1], MockResult::ProcessHeader(pdf.clone()));
        assert_eq!(mock_results[2], MockResult::FulltextToTei(pdf.clone()));
        assert_eq!(mock_results[3], MockResult::ProcessReferences(pdf.clone()));
    }

    // test_cleanup is no longer needed as TempDir handles cleanup via Drop.
}
