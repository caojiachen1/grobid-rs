use crate::build_modules::common::*;
use crate::build_modules::fingerprint;
use crate::build_modules::utils::run_command;

fn run_gradle_build(grobid_source_root: &Path, java_home: &Path) -> Result<()> {
    let gradlew_name = if cfg!(windows) {
        "gradlew.bat"
    } else {
        "gradlew"
    };
    let gradlew_path = grobid_source_root.join(gradlew_name);

    if !gradlew_path.exists() {
        bail!("gradlew script not found at {}. Please ensure the Grobid source is correctly extracted and contains the Gradle wrapper.", gradlew_path.display());
    }

    print_cargo_warning(&format!(
        "Starting Gradle build in {} using JAVA_HOME={}",
        grobid_source_root.display(),
        java_home.display()
    ));

    // Disable Gradle Daemon (--no-daemon) to avoid persistent locks. Clean first, then build tasks.
    let clean_task = "clean";
    print_cargo_warning(&format!("Running Gradle task: {}", clean_task));
    run_command(
        &gradlew_path,
        &["--no-daemon", clean_task],
        grobid_source_root,
        Some(&[("JAVA_HOME", java_home)]),
    )
    .with_context(|| format!("Gradle task {} failed.", clean_task))?;

    let build_tasks = vec![":grobid-core:shadowJar", "assemble"]; // assemble builds grobid-home resources
    for task in build_tasks {
        print_cargo_warning(&format!("Running Gradle task: {}", task));
        run_command(
            &gradlew_path,
            &["--no-daemon", task],
            grobid_source_root,
            Some(&[("JAVA_HOME", java_home)]),
        )
        .with_context(|| format!("Gradle task {} failed.", task))?;
    }

    print_cargo_warning("Gradle build tasks completed successfully.");
    Ok(())
}

fn copy_grobid_artifacts(
    grobid_source_root: &Path,
    target_grobid_deployment_dir: &Path,
) -> Result<()> {
    print_cargo_warning(&format!(
        "Copying Grobid artifacts from {} to {}",
        grobid_source_root.display(),
        target_grobid_deployment_dir.display()
    ));

    if !target_grobid_deployment_dir.exists() {
        fs::create_dir_all(target_grobid_deployment_dir).with_context(|| {
            format!(
                "Failed to create target Grobid deployment directory: {}",
                target_grobid_deployment_dir.display()
            )
        })?;
    }

    // 1. Copy grobid-core-X.Y.Z-onejar.jar
    let onejar_name = format!(
        "{}-{}{}",
        GROBID_JAR_NAME_PREFIX,
        GROBID_RELEASE_TAG, // This comes from common.rs, should match version
        GROBID_ONEJAR_NAME_SUFFIX
    );
    let onejar_source_path = grobid_source_root
        .join("grobid-core/build/libs")
        .join(&onejar_name);

    // Use GROBID_VERSION for the target jar name to maintain consistency
    let target_jar_name = format!(
        "{}-{}{}",
        GROBID_JAR_NAME_PREFIX, GROBID_VERSION, GROBID_ONEJAR_NAME_SUFFIX
    );
    let onejar_target_path = target_grobid_deployment_dir.join(&target_jar_name);

    if !onejar_source_path.exists() {
        bail!(
            "Built grobid-core onejar not found at {}. Ensure Gradle build was successful.",
            onejar_source_path.display()
        );
    }
    print_cargo_warning(&format!(
        "Copying JAR: {} to {}",
        onejar_source_path.display(),
        onejar_target_path.display()
    ));
    fs::copy(&onejar_source_path, &onejar_target_path).with_context(|| {
        format!(
            "Failed to copy onejar from {} to {}",
            onejar_source_path.display(),
            onejar_target_path.display()
        )
    })?;

    // 2. Copy grobid-home contents
    let grobid_home_source_path = grobid_source_root.join(GROBID_HOME_DIR_NAME);
    let grobid_home_target_path = target_grobid_deployment_dir.join(GROBID_HOME_DIR_NAME);

    if !grobid_home_source_path.exists() {
        bail!(
            "Built grobid-home directory not found at {}. Ensure Gradle build was successful.",
            grobid_home_source_path.display()
        );
    }
    if grobid_home_target_path.exists() {
        // Clean if exists to ensure fresh copy
        fs::remove_dir_all(&grobid_home_target_path).with_context(|| {
            format!(
                "Failed to clean existing target grobid-home directory: {}",
                grobid_home_target_path.display()
            )
        })?;
    }
    fs::create_dir_all(&grobid_home_target_path).with_context(|| {
        format!(
            "Failed to create target grobid-home directory: {}",
            grobid_home_target_path.display()
        )
    })?;

    print_cargo_warning(&format!(
        "Copying grobid-home contents from {} to {}",
        grobid_home_source_path.display(),
        grobid_home_target_path.display()
    ));
    let mut options = DirCopyOptions::new();
    options.overwrite = true;
    options.content_only = true;
    copy_dir_contents(&grobid_home_source_path, &grobid_home_target_path, &options).map_err(
        |e: FsExtraError| anyhow::anyhow!("Failed to copy grobid-home contents: {}", e.to_string()),
    )?;

    print_cargo_warning("Grobid artifacts copied successfully.");
    Ok(())
}

pub fn build_and_stage_grobid(
    grobid_source_root: &Path, // e.g., .../assets_dir/grobid-0.8.2/source/grobid-0.8.2
    target_grobid_deployment_dir: &Path, // e.g., .../assets_dir/grobid-0.8.2/deployment
    java_home_path: &Path,
) -> Result<()> {
    let success_marker = target_grobid_deployment_dir.join(BUILD_SUCCESS_MARKER_FILE);
    let fp_path = target_grobid_deployment_dir.join(".fingerprint.json");

    let gradlew = grobid_source_root.join(if cfg!(windows) {"gradlew.bat"} else {"gradlew"});
    let current_fp = fingerprint::Fingerprint::current(java_home_path, &gradlew)?;
    
    let up_to_date = fp_path.is_file()
        && success_marker.exists()
        && match File::open(&fp_path) {
            Ok(file) => match serde_json::from_reader::<_, fingerprint::Fingerprint>(file) {
                Ok(stored_fp) => stored_fp == current_fp,
                Err(_) => false,
            },
            Err(_) => false,
        };

    if !up_to_date {
        print_cargo_warning(&format!(
            "Grobid artifacts not found or build incomplete at {}. Will build and stage.",
            target_grobid_deployment_dir.display()
        ));

        if !target_grobid_deployment_dir.exists() {
            fs::create_dir_all(target_grobid_deployment_dir).with_context(|| {
                format!(
                    "Failed to create Grobid deployment directory: {}",
                    target_grobid_deployment_dir.display()
                )
            })?;
        }

        run_gradle_build(grobid_source_root, java_home_path)?;
        copy_grobid_artifacts(grobid_source_root, target_grobid_deployment_dir)?;

        fs::File::create(&success_marker).with_context(|| {
            format!(
                "Failed to create build success marker: {}",
                success_marker.display()
            )
        })?;
        
        // Save the fingerprint
        let file = File::create(&fp_path).with_context(|| 
            format!("Failed to create fingerprint file at {}", fp_path.display())
        )?;
        serde_json::to_writer_pretty(file, &current_fp).with_context(||
            format!("Failed to write fingerprint data to {}", fp_path.display())
        )?;
        
        print_cargo_warning(&format!(
            "Grobid successfully built and artifacts staged at: {}",
            target_grobid_deployment_dir.display()
        ));
    } else {
        print_cargo_warning("Grobid artefacts unchanged – skipping Gradle build");
    }
    Ok(())
}
