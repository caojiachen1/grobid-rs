use grobid_rs::{init, is_initialized, shutdown, GrobidConfig, GrobidConfigBuilder};
use serial_test::serial;
use std::env;
use std::path::PathBuf;

// We use serial_test to ensure JVM tests don't run in parallel

fn get_test_grobid_path() -> PathBuf {
    // Use the path set by the build script
    PathBuf::from(env!("GROBID_RS_ASSETS_PATH"))
}

#[test]
#[serial]
fn test_basic_initialization() {
    // Make sure we're not initialized at the start of the test
    if is_initialized() {
        shutdown().expect("Failed to shutdown JVM");
    }
    assert!(
        !is_initialized(),
        "JVM should not be initialized at test start"
    );

    // Initialize with default config
    let config = GrobidConfig::default();
    let result = init(&config);

    // Check initialization succeeded
    assert!(
        result.is_ok(),
        "Failed to initialize with default config: {:?}",
        result
    );
    assert!(is_initialized(), "JVM should be initialized after init()");

    // Cleanup
    shutdown().expect("Failed to shutdown JVM");
    assert!(
        !is_initialized(),
        "JVM should not be initialized after shutdown"
    );
}

#[test]
#[serial]
fn test_custom_initialization() {
    // Make sure we're not initialized at the start of the test
    if is_initialized() {
        shutdown().expect("Failed to shutdown JVM");
    }

    // Create a custom config
    let config = GrobidConfigBuilder::default()
        .base_path(get_test_grobid_path())
        .max_memory("512m")
        .jvm_option("-Djava.awt.headless=true")
        .system_property("grobid.use_heuristics", "true")
        .build();

    let result = init(&config);

    // Check initialization succeeded
    assert!(
        result.is_ok(),
        "Failed to initialize with custom config: {:?}",
        result
    );
    assert!(is_initialized(), "JVM should be initialized");

    // Cleanup
    shutdown().expect("Failed to shutdown JVM");
}

#[test]
#[serial]
fn test_invalid_path_initialization() {
    // Make sure we're not initialized at the start of the test
    if is_initialized() {
        shutdown().expect("Failed to shutdown JVM");
    }

    // Create config with invalid path
    let config = GrobidConfigBuilder::default()
        .base_path(PathBuf::from("/path/that/definitely/does/not/exist"))
        .build();

    let result = init(&config);

    // Check initialization failed
    assert!(
        result.is_err(),
        "Initialization should fail with invalid path"
    );
    assert!(
        !is_initialized(),
        "JVM should not be initialized after failed init"
    );
}

#[test]
#[serial]
fn test_reinitialization() {
    // Make sure we're not initialized at the start of the test
    if is_initialized() {
        shutdown().expect("Failed to shutdown JVM");
    }

    // Initialize with default config
    let config = GrobidConfig::default();
    let result1 = init(&config);
    assert!(result1.is_ok(), "First initialization failed");
    assert!(
        is_initialized(),
        "JVM should be initialized after first init"
    );

    // Try initializing again - should succeed without error
    let result2 = init(&config);
    assert!(result2.is_ok(), "Re-initialization should succeed");
    assert!(
        is_initialized(),
        "JVM should still be initialized after re-init"
    );

    // Cleanup
    shutdown().expect("Failed to shutdown JVM");
}

#[test]
#[serial]
fn test_initialization_and_shutdown_cycle() {
    // Multiple init/shutdown cycles
    for i in 0..3 {
        // Make sure we're not initialized at the start of each iteration
        if is_initialized() {
            shutdown().expect("Failed to shutdown JVM");
        }
        assert!(
            !is_initialized(),
            "JVM should not be initialized at start of cycle {}",
            i
        );

        // Initialize with default config
        let config = GrobidConfig::default();
        let result = init(&config);
        assert!(
            result.is_ok(),
            "Failed to initialize in cycle {}: {:?}",
            i,
            result
        );
        assert!(is_initialized(), "JVM should be initialized in cycle {}", i);

        // Shutdown
        let shutdown_result = shutdown();
        assert!(
            shutdown_result.is_ok(),
            "Failed to shutdown in cycle {}: {:?}",
            i,
            shutdown_result
        );
        assert!(
            !is_initialized(),
            "JVM should not be initialized after shutdown in cycle {}",
            i
        );
    }
}
