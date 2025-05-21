use crate::build_modules::common::*;
use crate::build_modules::utils::run_command;

fn build_jlink_runtime(
    java_home: &Path,
    jlink_output_dir: &Path,
    _target_grobid_deployment_dir: &Path,
) -> Result<()> {
    print_cargo_warning(&format!(
        "Building jlink runtime at {} using JAVA_HOME={}",
        jlink_output_dir.display(),
        java_home.display()
    ));
    let jlink_exe_name = if cfg!(windows) { "jlink.exe" } else { "jlink" };
    let jlink_exe_path = java_home.join("bin").join(jlink_exe_name);

    if !jlink_exe_path.exists() {
        bail!(
            "jlink executable not found at {}. Ensure JAVA_HOME points to a full JDK, not just a JRE.",
            jlink_exe_path.display()
        );
    }

    // Clean the output directory before running jlink
    if jlink_output_dir.exists() {
        fs::remove_dir_all(jlink_output_dir).with_context(|| {
            format!(
                "Failed to remove existing jlink runtime directory: {}",
                jlink_output_dir.display()
            )
        })?;
    }

    // Define the module path for jlink. We intentionally point this to the JDK's
    // own `jmods` directory rather than the Grobid application JARs. Supplying
    // the Grobid JARs here caused them to be baked into the custom runtime as
    // *automatic modules*, which in turn made their classes load through the
    // bootstrap class loader (returning `null` from `Class::getClassLoader`). A
    // few third-party libraries – most notably JSONIC used by Grobid – assume a
    // non-null class loader and blow up with a `NullPointerException` during
    // static initialisation. Restricting the module path to the standard JDK
    // modules keeps application libraries on the regular class path and avoids
    // that problem.

    let jmods_dir = java_home.join("jmods");
    if !jmods_dir.exists() {
        bail!("JDK jmods directory not found at {}. Ensure JAVA_HOME points to a full JDK installation.", jmods_dir.display());
    }

    let module_path_str = jmods_dir.to_str().with_context(|| {
        format!(
            "Failed to convert jmods directory path to string: {}",
            jmods_dir.display()
        )
    })?;

    let modules_to_include = JAKARTA_JLINK_MODULES;
    let args = vec![
        "--module-path",
        module_path_str,
        "--add-modules",
        modules_to_include,
        "--strip-debug",
        "--no-header-files",
        "--no-man-pages",
        "--compress=2",
        "--output",
        jlink_output_dir.to_str().unwrap(), // jlink requires a string path
    ];

    run_command(&jlink_exe_path, &args, java_home, None)
        .with_context(|| "jlink execution failed.")?;

    print_cargo_warning(&format!(
        "jlink runtime built successfully at: {}",
        jlink_output_dir.display()
    ));
    Ok(())
}

pub fn ensure_jlink_runtime(
    java_home_path: &Path,
    target_grobid_deployment_dir: &Path, // Base directory where JRE will be a subdir
) -> Result<PathBuf> {
    let jlink_runtime_dir = target_grobid_deployment_dir.join(JLINK_RUNTIME_SUBDIR_NAME);
    let success_marker = jlink_runtime_dir.join(JRE_SUCCESS_MARKER_FILE);

    if !success_marker.exists() {
        print_cargo_warning(&format!(
            "jlink JRE not found or build incomplete at {}. Will build JRE.",
            jlink_runtime_dir.display()
        ));
        build_jlink_runtime(
            java_home_path,
            &jlink_runtime_dir,
            target_grobid_deployment_dir,
        )?;
        fs::File::create(&success_marker).with_context(|| {
            format!(
                "Failed to create JRE success marker: {}",
                success_marker.display()
            )
        })?;
        print_cargo_warning(&format!(
            "jlink JRE successfully built at: {}",
            jlink_runtime_dir.display()
        ));
    } else {
        print_cargo_warning(&format!(
            "Found existing successfully built jlink JRE at: {}",
            jlink_runtime_dir.display()
        ));
    }
    Ok(jlink_runtime_dir)
}
