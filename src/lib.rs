use jni::{objects::*, InitArgsBuilder, JNIVersion, JavaVM};
use once_cell::sync::OnceCell;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use tracing::{debug, error, info, warn};

mod cache;
mod config;
mod engine;
mod errors;
pub mod format;
mod jni_handle;

pub use cache::{
    cache_exists, fulltext_to_tei_cached, get_cache_dir, get_cache_path, get_cache_stats,
    process_header_cached, process_references_cached, read_cache, reset_cache_stats, write_cache,
    CacheConfig, CacheStats, OutputType,
};
pub use config::{
    GrobidAnalysisConfig, GrobidAnalysisConfigBuilder, GrobidConfig, GrobidConfigBuilder,
};
pub use engine::{fulltext_to_tei, process_header, process_references, run_pdfalto};
pub use errors::GrobidError;
pub use format::FormatConverter;
pub use jni_handle::JniHandle;

/// Log verbosity levels for Grobid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

// GrobidError and GrobidConfig are now defined in the errors.rs and config.rs modules
// and re-exported at the top of this file

// Default implementation for GrobidConfig is now in config.rs

static JVM: OnceCell<JavaVM> = OnceCell::new();
static ENGINE: OnceCell<Mutex<GlobalRef>> = OnceCell::new();
// Expected Grobid version - must match the version bundled with the library
pub const GROBID_VERSION: &str = "0.8.2";

/// Boot JVM + Grobid.
///
/// # Deprecated
/// This function is deprecated in favor of `init_with_config`. It will be removed in a future version.
#[deprecated(since = "0.1.0", note = "Use init_with_config instead")]
pub fn init(base: &Path) -> Result<(), GrobidError> {
    init_with_config(&GrobidConfig::new(base))
}

/// Boot JVM + Grobid with the provided configuration.
///
/// The configuration's `base_path` should point to a directory containing `runtime/` and `grobid/`.
/// The `runtime` directory is expected to have a subdirectory named after the OS
/// (e.g., "linux-latest", "macos-14", "windows-latest") which is created by the build script.
pub fn init_with_config(config: &GrobidConfig) -> Result<(), GrobidError> {
    info!("Initializing Grobid with configuration: {:?}", config);
    if JVM.get().is_some() {
        debug!("JVM already initialized, reusing existing instance");
        return Ok(());
    }

    // Validate the configuration
    debug!("Validating configuration");
    config.validate()?;

    // Check Grobid version compatibility
    debug!("Checking Grobid version compatibility");
    check_grobid_version(&config.base_path)?;

    // ---------- paths ----------
    // Use the JLink runtime path provided at compile time
    let runtime_dir = PathBuf::from(env!("JLINK_RUNTIME_PATH"));
    let jvm_lib = match std::env::consts::OS {
        "windows" => runtime_dir.join("bin/server/jvm.dll"),
        "macos" => runtime_dir.join("lib/server/libjvm.dylib"),
        _ => runtime_dir.join("lib/server/libjvm.so"),
    };
    // Use the compile-time provided Grobid JAR and home paths
    let classpath = PathBuf::from(env!("GROBID_JAR_PATH"));
    let grobid_home_path = PathBuf::from(env!("GROBID_HOME_PATH"));
    let lib_path = grobid_home_path.join("lib");

    // ---------- JVM args ----------
    let class_path_arg = format!("-Djava.class.path={}", classpath.display());
    let grobid_home_arg = format!("-Dorg.grobid.home={}", grobid_home_path.display());
    let library_path_arg = format!("-Djava.library.path={}", lib_path.display());

    // Add JSONIC-specific system property to prevent classloader issues
    let jsonic_option =
        "-Dnet.arnx.jsonic.factory=net.arnx.jsonic.factory.StrictClassLoaderFactory";

    // Collect all JVM options in a vector
    let mut jvm_options = Vec::new();

    // Add basic options
    jvm_options.push(class_path_arg);
    jvm_options.push(grobid_home_arg);
    jvm_options.push(library_path_arg);
    jvm_options.push(format!("-Xmx{}", config.max_memory));

    // Add JSONIC option
    jvm_options.push(jsonic_option.to_string());

    // Add custom JVM options from the configuration
    jvm_options.extend(config.jvm_options.iter().cloned());

    // Add all custom system properties from config
    for (key, value) in &config.system_properties {
        jvm_options.push(format!("-D{}={}", key, value));
    }

    // Configure log level if not default
    match config.log_level {
        LogLevel::Off => {
            jvm_options.push("-Dorg.slf4j.simpleLogger.defaultLogLevel=OFF".to_string());
        }
        LogLevel::Error => {
            jvm_options.push("-Dorg.slf4j.simpleLogger.defaultLogLevel=ERROR".to_string());
        }
        LogLevel::Warn => {
            jvm_options.push("-Dorg.slf4j.simpleLogger.defaultLogLevel=WARN".to_string());
        }
        LogLevel::Info => {} // Default, no need to set
        LogLevel::Debug => {
            jvm_options.push("-Dorg.slf4j.simpleLogger.defaultLogLevel=DEBUG".to_string());
        }
        LogLevel::Trace => {
            jvm_options.push("-Dorg.slf4j.simpleLogger.defaultLogLevel=TRACE".to_string());
        }
    }

    // Create JVM args builder with all options
    let mut args_builder = InitArgsBuilder::new().version(JNIVersion::V8);

    // Add all collected options
    for option in &jvm_options {
        args_builder = args_builder.option(option);
    }

    let args = args_builder
        .build()
        .map_err(|e| GrobidError::JvmInitialization(e.to_string()))?;

    // ---------- start JVM ----------
    info!("Starting JVM with library at {}", jvm_lib.display());
    let jvm_lib_path_buf = jvm_lib.clone();
    let jvm = match JavaVM::with_libjvm(args, move || Ok(jvm_lib_path_buf)) {
        Ok(jvm) => {
            info!("JVM started successfully");
            jvm
        }
        Err(e) => {
            error!("Failed to start JVM: {}", e);
            return Err(GrobidError::JvmInitialization(e.to_string()));
        }
    };

    {
        // New scope for env
        let mut env = jvm.attach_current_thread().map_err(GrobidError::Jni)?;

        // Set the thread's context classloader to prevent JSONIC initialization issues
        let thread_cls = env
            .find_class("java/lang/Thread")
            .map_err(GrobidError::Jni)?;
        let current_thread = env
            .call_static_method(thread_cls, "currentThread", "()Ljava/lang/Thread;", &[])
            .map_err(GrobidError::Jni)?
            .l()
            .map_err(GrobidError::Jni)?;

        let system_cls = env
            .find_class("java/lang/ClassLoader")
            .map_err(GrobidError::Jni)?;
        let system_classloader = env
            .call_static_method(
                system_cls,
                "getSystemClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[],
            )
            .map_err(GrobidError::Jni)?
            .l()
            .map_err(GrobidError::Jni)?;

        env.call_method(
            current_thread,
            "setContextClassLoader",
            "(Ljava/lang/ClassLoader;)V",
            &[JValue::Object(&system_classloader)],
        )
        .map_err(GrobidError::Jni)?;

        // ---------- init Grobid ----------
        info!("Initializing Grobid engine");
        debug!("Finding GrobidFactory class");
        let factory_cls = env
            .find_class("org/grobid/core/factory/GrobidFactory")
            .map_err(GrobidError::Jni)?;

        debug!("Getting GrobidFactory instance");
        let factory = env
            .call_static_method(
                factory_cls,
                "getInstance",
                "()Lorg/grobid/core/factory/GrobidFactory;",
                &[],
            )
            .map_err(GrobidError::Jni)?
            .l()
            .map_err(GrobidError::Jni)?;

        debug!("Creating Grobid engine");
        let engine_obj = env
            .call_method(
                factory,
                "createEngine",
                "()Lorg/grobid/core/engines/Engine;",
                &[],
            )
            .map_err(GrobidError::Jni)?
            .l()
            .map_err(GrobidError::Jni)?;

        let engine_global_ref = env.new_global_ref(engine_obj).map_err(GrobidError::Jni)?;
        if ENGINE.set(Mutex::new(engine_global_ref)).is_err() {
            return Err(GrobidError::JvmInitialization(
                "ENGINE already initialized".to_string(),
            ));
        }
    } // env is dropped here

    if JVM.set(jvm).is_err() {
        error!("Failed to set JVM global reference: JVM already initialized");
        return Err(GrobidError::JvmInitialization(
            "JVM already initialized".to_string(),
        ));
    }

    info!("Grobid initialized successfully");
    Ok(())
}

// JniHandle is now defined in jni_handle.rs

// ---------------- version compatibility check ----------
/// Check if the installed Grobid version matches the expected version.
///
/// This helps prevent cryptic Java errors by validating version compatibility
/// before attempting to initialize the JVM and Grobid engine.
fn check_grobid_version(base_path: &Path) -> Result<(), GrobidError> {
    let properties_path = base_path.join("grobid-home/config/grobid.properties");

    debug!("Checking Grobid version in {}", properties_path.display());

    if !properties_path.exists() {
        // For development purposes, we'll log a warning but continue
        let warning_msg = format!(
            "Grobid properties file not found at {}. Version check skipped.",
            properties_path.display()
        );
        warn!("{}", warning_msg);
        return Ok(());
    }

    let content = match std::fs::read_to_string(&properties_path) {
        Ok(content) => content,
        Err(e) => {
            // For development purposes, we'll log a warning but continue
            let warning_msg = format!(
                "Failed to read Grobid properties file: {}. Version check skipped.",
                e
            );
            warn!("{}", warning_msg);
            return Ok(());
        }
    };

    // Look for version line (grobid.version=X.Y.Z)
    if let Some(line) = content
        .lines()
        .find(|l| l.trim().starts_with("grobid.version="))
    {
        if let Some(found_version) = line.trim().strip_prefix("grobid.version=") {
            let found_version = found_version.trim();
            debug!("Found Grobid version: {}", found_version);

            // Compare with expected version
            if !found_version.starts_with(GROBID_VERSION) {
                warn!(
                    "Grobid version mismatch: expected {}, found {}. This may cause issues.",
                    GROBID_VERSION, found_version
                );
                // For now, we'll continue despite version mismatch
                return Ok(());
            }

            // Version matches, return Ok
            info!("Grobid version check passed: {}", GROBID_VERSION);
            return Ok(());
        }
    }

    // Version property not found in the file
    let warning_msg = format!(
        "Grobid version not found in properties file. Expected version: {}. Version check skipped.",
        GROBID_VERSION
    );
    warn!("{}", warning_msg);
    Ok(())
}

#[cfg(test)]
mod tests;
