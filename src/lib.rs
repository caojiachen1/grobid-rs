use jni::{objects::*, JNIEnv, JavaVM, InitArgsBuilder, JNIVersion};
use once_cell::sync::OnceCell;
use std::{path::{Path, PathBuf}, process::Command, sync::Mutex};

#[derive(thiserror::Error, Debug)]
pub enum GrobidError {
    #[error("Grobid not initialised")] NotInitialised,
    #[error("JNI error: {0}")] Jni(#[from] jni::errors::Error),
    #[error("JVM initialization error: {0}")] JvmInitialization(String),
    #[error("Java exception: {0}")] Java(String),
    #[error("pdfalto failed: {0}")] PdfAlto(String),
}

static JVM: OnceCell<JavaVM> = OnceCell::new();
static ENGINE: OnceCell<Mutex<GlobalRef>> = OnceCell::new();

/// Boot JVM + Grobid. `base` should point to directory containing `runtime/` and `grobid/`.
/// The `runtime` directory is expected to have a subdirectory named after the OS (e.g., "linux-latest", "macos-14", "windows-latest")
/// which is created by the CI script.
pub fn init(base: &Path) -> Result<(), GrobidError> {
    if JVM.get().is_some() { return Ok(()); }

    // ---------- paths ----------
    // The CI script places the jlinked JRE in runtime/${{ matrix.os }}
    let os_specific_runtime_dir_name = match std::env::consts::OS {
        "linux" => "ubuntu-latest", // Assuming CI uses ubuntu-latest for linux
        "macos" => "macos-14",      // Assuming CI uses macos-14 for macOS
        "windows" => "windows-latest",// Assuming CI uses windows-latest for windows
        _ => unimplemented!("Unsupported OS for jlink runtime path"),
    };
    let runtime_os_dir = base.join("runtime").join(os_specific_runtime_dir_name);
    let grobid_dir  = base.join("grobid");
    let jvm_lib = match std::env::consts::OS {
        "windows" => runtime_os_dir.join("bin/server/jvm.dll"),
        "macos"   => runtime_os_dir.join("lib/server/libjvm.dylib"),
        _          => runtime_os_dir.join("lib/server/libjvm.so"),
    };
    let classpath = grobid_dir.join("grobid-core.jar");
    let grobid_home_path = grobid_dir.join("grobid-home");
    let lib_path = grobid_home_path.join("lib");

    // ---------- JVM args ----------
    let class_path_arg = format!("-Djava.class.path={}", classpath.display());
    let grobid_home_arg = format!("-Dorg.grobid.home={}", grobid_home_path.display());
    let library_path_arg = format!("-Djava.library.path={}", lib_path.display());

    let args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option(&class_path_arg)
        .option(&grobid_home_arg)
        .option(&library_path_arg)
        .option("-Xmx1G")
        .build()
        .map_err(|e| GrobidError::JvmInitialization(e.to_string()))?;

    // ---------- start JVM ----------
    let jvm_lib_path_buf = jvm_lib.clone();
    let jvm = JavaVM::with_libjvm(args, move || Ok(jvm_lib_path_buf))
        .map_err(|e| GrobidError::JvmInitialization(e.to_string()))?;
    
    { // New scope for env
        let mut env = jvm.attach_current_thread().map_err(GrobidError::Jni)?;

        // ---------- init Grobid ----------
        let factory_cls = env.find_class("org/grobid/core/factory/GrobidFactory").map_err(GrobidError::Jni)?;
        let factory = env.call_static_method(factory_cls, "getInstance", "()Lorg/grobid/core/factory/GrobidFactory;", &[])
            .map_err(GrobidError::Jni)?.l().map_err(GrobidError::Jni)?;
        let engine_obj = env.call_method(factory, "createEngine", "()Lorg/grobid/core/engines/Engine;", &[])
            .map_err(GrobidError::Jni)?.l().map_err(GrobidError::Jni)?;

        let engine_global_ref = env.new_global_ref(engine_obj).map_err(GrobidError::Jni)?;
        if ENGINE.set(Mutex::new(engine_global_ref)).is_err() {
            return Err(GrobidError::JvmInitialization("ENGINE already initialized".to_string()));
        }
    } // env is dropped here

    if JVM.set(jvm).is_err() {
        return Err(GrobidError::JvmInitialization("JVM already initialized".to_string()));
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

    let eng_ref = ENGINE.get().ok_or(GrobidError::NotInitialised)?;
    
    let locked_engine_gref = eng_ref.lock().unwrap();
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
                Ok(msg_l) => guard.get_string(&JString::from(msg_l)).map(|s| s.into()).unwrap_or_else(|_| "Failed to get exception message".to_string()),
                Err(_) => "Exception object was null or not a String".to_string(),
            },
            Err(_) => "Failed to call toString on exception object".to_string(),
        };
        return Err(GrobidError::Java(java_msg));
    }
    Ok(out)
}

// ---------- helpers for calling engine methods ----------
fn call_engine_process_method_with_file_input(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    method_name: &str,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    let file_cls = env.find_class("java/io/File")?;
    let j_path_str = env.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj = env.new_object(file_cls, "(Ljava/lang/String;)V", &[JValue::from(&j_path_str)])?;

    let j_result_string_obj = env.call_method(
        engine,
        method_name,
        "(Ljava/io/File;)Ljava/lang/String;",
        &[JValue::from(&j_file_obj)],
    )?.l().map_err(GrobidError::from)?;

    // Convert to Rust String internally
    let result_string = env.get_string(&JString::from(j_result_string_obj))?.into();
    Ok(result_string)
}

fn call_engine_fulltext_to_tei(
    env: &mut JNIEnv<'_>,
    engine: JObject<'_>,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    let file_cls = env.find_class("java/io/File")?;
    let j_path_str = env.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj = env.new_object(file_cls, "(Ljava/lang/String;)V", &[JValue::from(&j_path_str)])?;

    let cfg_cls = env.find_class("org/grobid/core/engines/config/GrobidAnalysisConfig")?;
    let cfg_obj = env.call_static_method(cfg_cls, "defaultInstance", "()Lorg/grobid/core/engines/config/GrobidAnalysisConfig;", &[])?.l()?;

    let j_tei_string_obj = env.call_method(
        engine,
        "fullTextToTEI",
        "(Ljava/io/File;Lorg/grobid/core/engines/config/GrobidAnalysisConfig;)Ljava/lang/String;",
        &[JValue::from(&j_file_obj), JValue::from(&cfg_obj)],
    )?.l().map_err(GrobidError::from)?;

    // Convert to Rust String internally
    let tei_string = env.get_string(&JString::from(j_tei_string_obj))?.into();
    Ok(tei_string)
}


// ---------------- public API ------------------

pub fn fulltext_to_tei(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env, engine| {
        // Now this directly returns String
        call_engine_fulltext_to_tei(env, engine, pdf)
    })
}

pub fn process_header(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env, engine| {
        // Now this directly returns String
        call_engine_process_method_with_file_input(env, engine, "processHeader", pdf)
    })
}

pub fn process_references(pdf: &Path) -> Result<String, GrobidError> {
    with_env(|env, engine| {
        // Now this directly returns String
        call_engine_process_method_with_file_input(env, engine, "processReferences", pdf)
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

    let bin = grobid_home.join("pdfalto").join(platform_name).join(bin_name);

    if !bin.exists() {
        return Err(GrobidError::PdfAlto(format!("pdfalto binary not found at {}", bin.display())));
    }
    let out_xml = pdf.with_extension("alto.xml");
    let status = Command::new(&bin)
        .arg("--inputFile").arg(pdf)
        .arg("--outputFile").arg(&out_xml)
        .status()
        .map_err(|e| GrobidError::PdfAlto(format!("pdfalto call failed: {}", e)))?;
    if !status.success() {
        return Err(GrobidError::PdfAlto(format!("pdfalto failed with status {:?}", status.code())));
    }
    Ok(out_xml)
} 