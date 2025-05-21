use jni::{objects::*, InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use once_cell::sync::OnceCell;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

mod config;
mod errors;
mod cache;
mod cache_prune;

pub use config::{
    GrobidAnalysisConfig, GrobidAnalysisConfigBuilder, GrobidConfig, GrobidConfigBuilder,
};
pub use errors::GrobidError;

// Cache types and functions
pub use cache::{
    CacheConfig, OutputType, 
    process_with_cache, get_cache_path,
    get_cache_dir, ensure_cache_dir,
};

// Cache management functions
pub use cache_prune::{
    get_cache_size, get_human_readable_cache_size,
    prune_cache, clear_cache, get_cache_summary,
    list_cache_files,
};

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
    if JVM.get().is_some() {
        return Ok(());
    }

    // Validate the configuration
    config.validate()?;

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
    let jvm_lib_path_buf = jvm_lib.clone();
    let jvm = JavaVM::with_libjvm(args, move || Ok(jvm_lib_path_buf))
        .map_err(|e| GrobidError::JvmInitialization(e.to_string()))?;

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
        let factory_cls = env
            .find_class("org/grobid/core/factory/GrobidFactory")
            .map_err(GrobidError::Jni)?;
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
        return Err(GrobidError::JvmInitialization(
            "JVM already initialized".to_string(),
        ));
    }

    Ok(())
}

// ---------------- helper: attach & handle exceptions ------------------
fn with_env<F, R>(f: F) -> Result<R, GrobidError>
where
    F: FnOnce(&mut JNIEnv<'_>, JObject<'_>) -> Result<R, GrobidError>,
{
    let jvm = JVM.get().ok_or(GrobidError::NotInitialised)?;
    let mut guard = jvm.attach_current_thread().map_err(GrobidError::Jni)?;
    let env_mut_ref = &mut guard;

    // Set the context classloader for this thread too, to ensure consistent behavior
    let thread_cls = env_mut_ref
        .find_class("java/lang/Thread")
        .map_err(GrobidError::Jni)?;
    let current_thread = env_mut_ref
        .call_static_method(thread_cls, "currentThread", "()Ljava/lang/Thread;", &[])
        .map_err(GrobidError::Jni)?
        .l()
        .map_err(GrobidError::Jni)?;

    let system_cls = env_mut_ref
        .find_class("java/lang/ClassLoader")
        .map_err(GrobidError::Jni)?;
    let system_classloader = env_mut_ref
        .call_static_method(
            system_cls,
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .map_err(GrobidError::Jni)?
        .l()
        .map_err(GrobidError::Jni)?;

    env_mut_ref
        .call_method(
            current_thread,
            "setContextClassLoader",
            "(Ljava/lang/ClassLoader;)V",
            &[JValue::Object(&system_classloader)],
        )
        .map_err(GrobidError::Jni)?;

    let engine_obj = ENGINE.get().ok_or(GrobidError::NotInitialised)?;

    let locked_engine_gref = engine_obj.lock().unwrap();
    let raw_engine_ptr = (*locked_engine_gref).as_raw();
    let engine_jobject = env_mut_ref.new_local_ref(unsafe { JObject::from_raw(raw_engine_ptr) })?;

    let out = f(env_mut_ref, engine_jobject)?;
    if guard.exception_check().map_err(GrobidError::Jni)? {
        let exception = guard.exception_occurred()?;
        guard.exception_describe().ok(); // Print details to stderr
        guard.exception_clear().ok();
        let msg_obj = guard.call_method(exception, "toString", "()Ljava/lang/String;", &[]);
        let java_msg = match msg_obj {
            Ok(msg_jval) => match msg_jval.l() {
                Ok(msg_l) => guard
                    .get_string(&JString::from(msg_l))
                    .map(|s| s.into())
                    .unwrap_or_else(|_| "Failed to get exception message".to_string()),
                Err(_) => "Exception object was null or not a String".to_string(),
            },
            Err(_) => "Failed to call toString on exception object".to_string(),
        };
        return Err(GrobidError::Java(java_msg));
    }
    Ok(out)
}

// ---------- helpers for calling engine methods ----------
#[allow(dead_code)]
fn call_engine_process_method_with_file_input(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    method_name: &str,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    let file_cls = env.find_class("java/io/File")?;
    let j_path_str = env.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj = env.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    let j_result_string_obj = env
        .call_method(
            engine,
            method_name,
            "(Ljava/io/File;)Ljava/lang/String;",
            &[JValue::from(&j_file_obj)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Convert to Rust String internally
    let result_string = env.get_string(&JString::from(j_result_string_obj))?.into();
    Ok(result_string)
}

fn call_engine_fulltext_to_tei(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    let file_cls: JClass<'_> = env.find_class("java/io/File")?;
    let j_path_str: JString<'_> = env.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj: JObject<'_> = env.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    let cfg_cls: JClass<'_> =
        env.find_class("org/grobid/core/engines/config/GrobidAnalysisConfig")?;
    let cfg_obj: JObject<'_> = env
        .call_static_method(
            cfg_cls,
            "defaultInstance",
            "()Lorg/grobid/core/engines/config/GrobidAnalysisConfig;",
            &[],
        )?
        .l()?;

    let j_tei_string_obj: JObject<'_> = env.call_method(
        engine,
        "fullTextToTEI",
        "(Ljava/io/File;Lorg/grobid/core/engines/config/GrobidAnalysisConfig;)Ljava/lang/String;",
        &[JValue::from(&j_file_obj), JValue::from(&cfg_obj)],
    )?.l().map_err(GrobidError::from)?;

    // Convert to Rust String internally
    let tei_string = env.get_string(&JString::from(j_tei_string_obj))?.into();
    Ok(tei_string)
}

// New helper for calling the correct processHeader overload that accepts (String, BiblioItem)
fn call_engine_process_header(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    // Convert Rust Path to Java String
    let j_path_str: JString<'_> = env.new_string(pdf_path.to_string_lossy())?;
    // Instantiate a new BiblioItem
    let biblio_cls: JClass<'_> = env.find_class("org/grobid/core/data/BiblioItem")?;
    let biblio_obj: JObject<'_> = env.new_object(biblio_cls, "()V", &[])?;
    // Call Engine.processHeader(String, BiblioItem)
    let j_result_obj: JObject<'_> = env
        .call_method(
            engine,
            "processHeader",
            "(Ljava/lang/String;Lorg/grobid/core/data/BiblioItem;)Ljava/lang/String;",
            &[JValue::from(&j_path_str), JValue::from(&biblio_obj)],
        )?
        .l()
        .map_err(GrobidError::from)?;
    // Convert Java String to Rust String
    let result_string = env.get_string(&JString::from(j_result_obj))?.into();
    Ok(result_string)
}

fn call_engine_process_references(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    // Convert Rust Path to Java File object
    let file_cls: JClass<'_> = env.find_class("java/io/File")?;
    let j_path_str: JString<'_> = env.new_string(pdf_path.to_string_lossy())?;
    let j_file: JObject<'_> = env.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    // Call Engine.processReferences(File, int)
    // The int parameter is the consolidation option (0 = no consolidation)
    let j_bib_list: JObject<'_> = env
        .call_method(
            engine,
            "processReferences",
            "(Ljava/io/File;I)Ljava/util/List;",
            &[JValue::from(&j_file), JValue::from(0)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Call the static method references2TEI to convert BibDataSet list to TEI String
    let empty_path: JString<'_> = env.new_string("")?;

    let j_result_obj: JObject<'_> = env
        .call_static_method(
            "org/grobid/core/engines/Engine",
            "references2TEI",
            "(Ljava/lang/String;Ljava/util/List;)Ljava/lang/String;",
            &[JValue::from(&empty_path), JValue::from(&j_bib_list)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Convert Java String to Rust String
    let result_string = env.get_string(&JString::from(j_result_obj))?.into();
    Ok(result_string)
}

// ---------------- public API ------------------

pub fn fulltext_to_tei(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env: &mut JNIEnv<'_>, engine: JObject<'_>| {
        // Now this directly returns String
        call_engine_fulltext_to_tei(env, engine, pdf)
    })
}

pub fn process_header(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env: &mut JNIEnv<'_>, engine: JObject<'_>| {
        // Call the correct processHeader overload with String and BiblioItem
        call_engine_process_header(env, engine, pdf)
    })
}

pub fn process_references(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env: &mut JNIEnv<'_>, engine: JObject<'_>| {
        call_engine_process_references(env, engine, pdf)
    })
}

// ---------------- pdfalto helper ------------------
/// Run pdfalto and return the path to the generated ALTO XML.
pub fn run_pdfalto(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError> {
    let bin_name = match std::env::consts::OS {
        "windows" => "pdfalto.exe",
        _ => "pdfalto",
    };
    let platform_name = match std::env::consts::OS {
        "windows" => "win-64",
        "macos" => {
            if cfg!(target_arch = "aarch64") {
                "mac_arm-64"
            } else {
                "mac-64"
            }
        }
        _ => "lin-64",
    };

    let bin = grobid_home
        .join("pdfalto")
        .join(platform_name)
        .join(bin_name);

    if !bin.exists() {
        return Err(GrobidError::PdfAlto(format!(
            "pdfalto binary not found at {}",
            bin.display()
        )));
    }
    let out_xml = pdf.with_extension("alto.xml");
    let status = Command::new(&bin)
        .arg("--inputFile")
        .arg(pdf)
        .arg("--outputFile")
        .arg(&out_xml)
        .status()
        .map_err(|e| GrobidError::PdfAlto(format!("pdfalto call failed: {}", e)))?;
    if !status.success() {
        return Err(GrobidError::PdfAlto(format!(
            "pdfalto failed with status {:?}",
            status.code()
        )));
    }
    Ok(out_xml)
}

#[cfg(test)]
mod tests;
