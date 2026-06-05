use crate::build_modules::common::{
    bail, env, print_cargo_info, print_cargo_warning, Path, PathBuf, Result,
    CARGO_LINK_LIB_DYLIB_PREFIX, CARGO_LINK_LIB_STATIC_PREFIX, CARGO_LINK_SEARCH_NATIVE_PREFIX,
    CARGO_RERUN_IF_CHANGED_ENV_VAR, CARGO_RERUN_IF_ENV_CHANGED_ENV_VAR,
    FORCE_GROBID_REBUILD_ENV_VAR, GROBID_HOME_DIR_NAME, GROBID_JAR_NAME_PREFIX,
    GROBID_ONEJAR_NAME_SUFFIX, GROBID_RS_ASSETS_PATH_ENV_VAR, GROBID_VERSION, JAVA_HOME_ENV_VAR,
};

#[allow(clippy::too_many_lines)]
pub fn setup_jni_linkage(
    java_home_path: &Path,
    jlink_runtime_path: &Path,
    target_grobid_deployment_dir: &Path,
) -> Result<()> {
    print_cargo_info("Configuring JNI linkage...");

    // --- JNI Library Path ---
    // On Windows, jvm.dll is in bin/server; on Linux/macOS, libjvm.so/dylib is in lib/server
    let jni_lib_path = ["bin/server", "lib/server"]
        .iter()
        .flat_map(|subdir| {
            [
                ("jlink runtime (bin)", jlink_runtime_path.join(subdir)),
                ("JDK", java_home_path.join(subdir)),
            ]
        })
        .find(|(_name, path)| {
            path.exists()
                && (path.join("libjvm.dylib").exists()
                    || path.join("libjvm.so").exists()
                    || path.join("jvm.dll").exists())
        })
        .map(|(name, path)| {
            print_cargo_info(&format!("Using JNI lib path from {}: {}", name, path.display()));
            path
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find JNI 'server' library directory in jlink runtime ({}) or JDK ({}).",
                jlink_runtime_path.display(),
                java_home_path.display()
            )
        })?;
    println!(
        "{}{}",
        CARGO_LINK_SEARCH_NATIVE_PREFIX,
        jni_lib_path.display()
    );

    // --- JNI Include Path --- (less critical if not compiling C code, but good for completeness)
    let jni_include_path = java_home_path.join("include");
    if jni_include_path.exists() {
        println!("cargo:include={}", jni_include_path.display());
        let platform_specific_jni_include_path = match env::consts::OS {
            "linux" => jni_include_path.join("linux"),
            "macos" => jni_include_path.join("darwin"),
            "windows" => jni_include_path.join("win32"),
            _ => PathBuf::new(), // Empty path if OS not recognized
        };
        if platform_specific_jni_include_path.exists() {
            println!(
                "cargo:include={}",
                platform_specific_jni_include_path.display()
            );
        }
    } else {
        print_cargo_warning(
            "JNI include directory not found. This might be an issue if compiling JNI C code.",
        );
    }

    // --- Dynamic Library Linking ---
    // For macOS, Linux, and Windows, we link against libjvm dynamically.
    // The exact name can vary slightly or be handled by the linker finding it in the search path.
    match env::consts::OS {
        "macos" | "linux" => println!("{CARGO_LINK_LIB_DYLIB_PREFIX}jvm"),
        "windows" => {
            // On Windows, the jvm.dll is usually found, but we might need to link against jvm.lib
            // The linker search path should handle finding jvm.dll at runtime.
            // Check for jvm.lib in the JDK lib directory (not server, usually just lib)
            let jdk_lib_dir = java_home_path.join("lib");
            if jdk_lib_dir.join("jvm.lib").exists() {
                println!(
                    "{}{}",
                    CARGO_LINK_SEARCH_NATIVE_PREFIX,
                    jdk_lib_dir.display()
                );
                println!("{CARGO_LINK_LIB_STATIC_PREFIX}jvm"); // Link against the import library
            } else {
                print_cargo_warning("jvm.lib not found in JDK lib directory. Relying on linker to find jvm.dll via server path.");
                // If no jvm.lib, still try to hint for dylib, though it's less common to specify this for Windows DLLs directly.
                println!("{CARGO_LINK_LIB_DYLIB_PREFIX}jvm");
            }
        }
        _ => print_cargo_warning(&format!(
            "Unsupported OS for JNI dynamic linking: {}",
            env::consts::OS
        )),
    }

    // Embed runtime rpath so the dynamic loader can find JNI lib at runtime
    // Windows uses DLL search path (app dir, PATH, etc.) — skip rpath
    if env::consts::OS != "windows" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", jni_lib_path.display());
    }

    // --- Grobid JAR and Home Path for Runtime ---
    // These are passed as environment variables that the Rust application can use at runtime.
    let grobid_jar_name =
        format!("{GROBID_JAR_NAME_PREFIX}-{GROBID_VERSION}{GROBID_ONEJAR_NAME_SUFFIX}");
    let final_jar_path = target_grobid_deployment_dir.join(grobid_jar_name);
    let final_grobid_home_path = target_grobid_deployment_dir.join(GROBID_HOME_DIR_NAME);

    if !final_jar_path.exists() {
        bail!(
            "Final Grobid JAR not found at {} after build and staging. This should not happen.",
            final_jar_path.display()
        );
    }
    if !final_grobid_home_path.exists() {
        bail!("Final Grobid Home directory not found at {} after build and staging. This should not happen.", final_grobid_home_path.display());
    }

    // Set environment variables for the main Rust compilation (accessible via env::var! in Rust code)
    println!(
        "cargo:rustc-env=GROBID_JAR_PATH={}",
        final_jar_path.display()
    );
    println!(
        "cargo:rustc-env=GROBID_HOME_PATH={}",
        final_grobid_home_path.display()
    );
    println!(
        "cargo:rustc-env=JLINK_RUNTIME_PATH={}",
        jlink_runtime_path.display()
    );

    print_cargo_info(&format!(
        "GROBID_JAR_PATH set to: {}",
        final_jar_path.display()
    ));
    print_cargo_info(&format!(
        "GROBID_HOME_PATH set to: {}",
        final_grobid_home_path.display()
    ));
    print_cargo_info(&format!(
        "JLINK_RUNTIME_PATH set to: {}",
        jlink_runtime_path.display()
    ));

    // --- Rerun Conditions ---
    println!("{CARGO_RERUN_IF_CHANGED_ENV_VAR}build.rs"); // Rerun if build.rs changes
    println!("{CARGO_RERUN_IF_ENV_CHANGED_ENV_VAR}{GROBID_RS_ASSETS_PATH_ENV_VAR}");
    println!("{CARGO_RERUN_IF_ENV_CHANGED_ENV_VAR}{FORCE_GROBID_REBUILD_ENV_VAR}");
    println!("{CARGO_RERUN_IF_ENV_CHANGED_ENV_VAR}{JAVA_HOME_ENV_VAR}");
    // Potentially add rerun if changed for files in build_modules/*, but that can get complex.
    // For now, changing build.rs itself (e.g. by adding a comment) will trigger a rerun.

    print_cargo_info("JNI linkage configuration complete.");
    Ok(())
}
