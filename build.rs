mod build_modules;

use anyhow::{Context, Result};
use build_modules::{
    build_ops::build_and_stage_grobid,
    common::{
        print_cargo_warning, FORCE_GROBID_REBUILD_ENV_VAR, GROBID_DIR_NAME_PREFIX,
        GROBID_HOME_DIR_NAME, GROBID_JAR_NAME_PREFIX, GROBID_ONEJAR_NAME_SUFFIX,
        GROBID_RS_ASSETS_PATH_ENV_VAR, GROBID_VERSION, JLINK_RUNTIME_SUBDIR_NAME,
        JRE_SUCCESS_MARKER_FILE,
    },
    java_env::locate_java_home,
    jni_config::setup_jni_linkage,
    jre_ops::ensure_jlink_runtime,
    source_ops::ensure_grobid_source_extracted,
};
use dotenv::dotenv;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::process::Command;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Check if vendored Grobid files are available
fn check_for_vendored_files() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let vendor_dir = manifest_dir.join("vendor");

    if !vendor_dir.exists() {
        return None;
    }

    let grobid_dir = vendor_dir.join("grobid");
    let jre_dir = vendor_dir.join("jre");

    // Check if required files exist
    let jar_name = format!(
        "{}-{}{}",
        GROBID_JAR_NAME_PREFIX, GROBID_VERSION, GROBID_ONEJAR_NAME_SUFFIX
    );
    let jar_path = grobid_dir.join(&jar_name);
    let jar_zst_path = grobid_dir.join(format!("{}.zst", &jar_name));
    let grobid_home_path = grobid_dir.join(GROBID_HOME_DIR_NAME);

    if (jar_path.exists() || jar_zst_path.exists()) && grobid_home_path.exists() && jre_dir.exists()
    {
        print_cargo_warning("Found vendored Grobid files");
        return Some(vendor_dir);
    }

    None
}

/// Copy vendored files to the deployment directory
fn use_vendored_files(vendor_dir: &Path, deployment_dir: &Path) -> Result<()> {
    print_cargo_warning(&format!(
        "Using vendored files from {} to {}",
        vendor_dir.display(),
        deployment_dir.display()
    ));

    // Create deployment directory if it doesn't exist
    fs::create_dir_all(deployment_dir)?;

    // Copy Grobid JAR and home directory
    let vendor_grobid_dir = vendor_dir.join("grobid");

    // Copy JAR file (decompressing if needed)
    let jar_name = format!(
        "{}-{}{}",
        GROBID_JAR_NAME_PREFIX, GROBID_VERSION, GROBID_ONEJAR_NAME_SUFFIX
    );
    let vendor_jar_path = vendor_grobid_dir.join(&jar_name);
    let target_jar_path = deployment_dir.join(&jar_name);

    if vendor_jar_path.exists() {
        fs::copy(&vendor_jar_path, &target_jar_path).with_context(|| {
            format!(
                "Failed to copy JAR from {} to {}",
                vendor_jar_path.display(),
                target_jar_path.display()
            )
        })?;
    } else {
        // Check for compressed version
        let compressed_jar_path = vendor_grobid_dir.join(format!("{}.zst", jar_name));
        if compressed_jar_path.exists() {
            print_cargo_warning(&format!(
                "Decompressing JAR from {} to {}",
                compressed_jar_path.display(),
                target_jar_path.display()
            ));
            decompress_zstd_file(&compressed_jar_path, &target_jar_path).with_context(|| {
                format!(
                    "Failed to decompress JAR from {} to {}",
                    compressed_jar_path.display(),
                    target_jar_path.display()
                )
            })?;
        } else {
            return Err(anyhow::anyhow!(
                "Neither JAR file nor compressed JAR file found at expected locations"
            ));
        }
    }
    // Process any other compressed files in grobid-home

    // Copy grobid-home directory
    let vendor_home_path = vendor_grobid_dir.join(GROBID_HOME_DIR_NAME);
    let target_home_path = deployment_dir.join(GROBID_HOME_DIR_NAME);

    if target_home_path.exists() {
        fs::remove_dir_all(&target_home_path).with_context(|| {
            format!(
                "Failed to remove existing grobid-home directory: {}",
                target_home_path.display()
            )
        })?;
    }

    copy_dir_recursive(&vendor_home_path, &target_home_path).with_context(|| {
        format!(
            "Failed to copy grobid-home from {} to {}",
            vendor_home_path.display(),
            target_home_path.display()
        )
    })?;

    // Copy JRE runtime directory
    let vendor_jre_dir = vendor_dir.join("jre");
    let target_runtime_dir = deployment_dir.join(JLINK_RUNTIME_SUBDIR_NAME);

    if target_runtime_dir.exists() {
        fs::remove_dir_all(&target_runtime_dir).with_context(|| {
            format!(
                "Failed to remove existing runtime directory: {}",
                target_runtime_dir.display()
            )
        })?;
    }

    fs::create_dir_all(&target_runtime_dir)?;

    // Copy platform-specific JRE directories
    for entry in fs::read_dir(&vendor_jre_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let platform_name = path.file_name().unwrap();
            let target_platform_dir = target_runtime_dir.join(platform_name);

            copy_dir_recursive(&path, &target_platform_dir).with_context(|| {
                format!(
                    "Failed to copy JRE platform dir from {} to {}",
                    path.display(),
                    target_platform_dir.display()
                )
            })?;
        }
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            // Check if this is a compressed file that needs decompression
            let file_name = src_path.file_name().unwrap().to_string_lossy().to_string();
            if file_name.ends_with(".zst") {
                // This is a compressed file, decompress it
                let target_path = dst_path.with_file_name(file_name.trim_end_matches(".zst"));
                print_cargo_warning(&format!(
                    "Decompressing {} to {}",
                    src_path.display(),
                    target_path.display()
                ));
                decompress_zstd_file(&src_path, &target_path)?;
            } else {
                // Regular file, just copy it
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }

    Ok(())
}

// Function to decompress a zstd file
fn decompress_zstd_file(compressed_path: &PathBuf, target_path: &PathBuf) -> Result<()> {
    if cfg!(unix) {
        // Try using zstd command line tool if available
        let result = Command::new("zstd")
            .args([
                "-d",
                "-f",
                compressed_path.to_str().unwrap(),
                "-o",
                target_path.to_str().unwrap(),
            ])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                print_cargo_warning("Successfully decompressed using zstd command");
                return Ok(());
            }
            _ => {
                print_cargo_warning(
                    "zstd command failed or not available, using internal decompression",
                );
                // Fall back to internal implementation
            }
        }
    }

    // If command-line tool failed or not on Unix, use Rust implementation
    let compressed_file = File::open(compressed_path)?;
    let mut compressed_data = Vec::new();
    BufReader::new(compressed_file).read_to_end(&mut compressed_data)?;

    let decompressed_data = zstd::stream::decode_all(io::Cursor::new(compressed_data))
        .with_context(|| "Failed to decompress zstd data")?;

    let mut output_file = File::create(target_path)?;
    output_file.write_all(&decompressed_data)?;

    Ok(())
}

fn main() -> Result<()> {
    // Load .env if present
    dotenv().ok();
    print_cargo_warning("Starting Grobid-RS modular build script");

    // Determine assets directory (override via env or default to target/grobid_assets)
    let assets_dir = match env::var(GROBID_RS_ASSETS_PATH_ENV_VAR) {
        Ok(val) => PathBuf::from(val),
        Err(_) => {
            let out_dir = PathBuf::from(env::var("OUT_DIR").context("OUT_DIR not set")?);
            let target_dir = out_dir
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .context("Failed to compute target directory from OUT_DIR")?;
            target_dir.join("grobid_assets")
        }
    };

    // Check if we should use vendored files or build from scratch
    let use_vendored = check_for_vendored_files();

    // Force clean if requested
    let force_clean = env::var(FORCE_GROBID_REBUILD_ENV_VAR).unwrap_or_default() == "true";
    if force_clean {
        print_cargo_warning("FORCE_GROBID_REBUILD is set to true, removing cached Grobid deployment and source directories");
        // Remove the entire grobid-<version> directory, which includes source, deployment, and runtime.
        // The ZIP file itself is directly under assets_dir and will be preserved.
        let grobid_version_dir =
            assets_dir.join(format!("{}{}", GROBID_DIR_NAME_PREFIX, GROBID_VERSION));
        if grobid_version_dir.exists() {
            fs::remove_dir_all(&grobid_version_dir).with_context(|| {
                format!(
                    "Failed to remove cached Grobid version directory: {}",
                    grobid_version_dir.display()
                )
            })?;
        }
    } else {
        // Even in non-force clean mode, we want to check if the JRE runtime needs rebuilding
        // because we might have updated the JRE modules
        let runtime_dir = assets_dir
            .join(format!("{}{}", GROBID_DIR_NAME_PREFIX, GROBID_VERSION))
            .join(JLINK_RUNTIME_SUBDIR_NAME);
        let jre_marker = assets_dir
            .join(format!("{}{}", GROBID_DIR_NAME_PREFIX, GROBID_VERSION))
            .join(JRE_SUCCESS_MARKER_FILE);

        // If runtime doesn't exist or no marker, we'll need to rebuild it
        if !runtime_dir.exists() || !jre_marker.exists() {
            print_cargo_warning("JRE runtime directory missing or incomplete, will rebuild");
        }
    }
    // Ensure the assets directory exists (retain any existing ZIP files)
    fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;
    print_cargo_warning(&format!("Assets directory: {}", assets_dir.display()));

    // Locate JAVA_HOME
    let java_home = locate_java_home()?;

    // Define the deployment directory path (where Grobid JAR, home, and JRE reside)
    let deployment_dir = assets_dir.join(format!("{}{}", GROBID_DIR_NAME_PREFIX, GROBID_VERSION));

    // Ensure the deployment directory structure is in place if force_clean removed it
    // or if it's the first build.
    if !deployment_dir.exists() {
        fs::create_dir_all(&deployment_dir).with_context(|| {
            format!(
                "Failed to create deployment directory: {}",
                deployment_dir.display()
            )
        })?;
    }

    // Expose the deployment directory path to Rust code at compile time
    // This env var is used by the lib.rs to locate assets at runtime.
    println!(
        "cargo:rustc-env={}={}",
        GROBID_RS_ASSETS_PATH_ENV_VAR,
        deployment_dir.display()
    );

    // Check if we should use vendored files and they're available
    if !force_clean && use_vendored.is_some() {
        // Use vendored files instead of building from scratch
        use_vendored_files(&use_vendored.unwrap(), &deployment_dir)?;
    } else {
        // Normal build process - download, extract, and build Grobid
        let source_dir = ensure_grobid_source_extracted(&assets_dir)?;

        // Build and stage Grobid artifacts (JAR, grobid-home) into the deployment directory
        // This step no longer handles its own cleaning; it relies on the top-level force_clean.
        build_and_stage_grobid(&source_dir, &deployment_dir, &java_home)?;
    }

    // Ensure custom JRE via jlink is built into the deployment directory
    // Note: If we're using vendored files, we've already copied the JRE,
    // but we still need to get the path for JNI linkage
    let jlink_dir = deployment_dir.join(JLINK_RUNTIME_SUBDIR_NAME);
    let jre_marker = deployment_dir.join(JRE_SUCCESS_MARKER_FILE);
    if !jlink_dir.exists() || !jre_marker.exists() || force_clean {
        // If runtime doesn't exist, or marker is missing, or force clean is on, rebuild the JRE
        if jlink_dir.exists() {
            print_cargo_warning(&format!(
                "Removing existing JRE runtime for rebuild at {}",
                jlink_dir.display()
            ));
            fs::remove_dir_all(&jlink_dir).with_context(|| {
                format!(
                    "Failed to remove existing JRE runtime directory: {}",
                    jlink_dir.display()
                )
            })?;
        }
        if jre_marker.exists() {
            fs::remove_file(&jre_marker).with_context(|| {
                format!(
                    "Failed to remove JRE success marker: {}",
                    jre_marker.display()
                )
            })?;
        }
        let jlink_result = ensure_jlink_runtime(&java_home, &deployment_dir)?;
        print_cargo_warning(&format!(
            "JRE runtime created at {}",
            jlink_result.display()
        ));
    } else {
        print_cargo_warning(&format!(
            "Using existing JRE runtime at {}",
            jlink_dir.display()
        ));
    }

    // Configure JNI linkage using the paths from the deployment and JRE directories
    setup_jni_linkage(&java_home, &jlink_dir, &deployment_dir)?;

    // The Grobid artefacts themselves live under `target/`; Cargo should **not**
    // rebuild when they change.  Tell it explicitly:
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={}", FORCE_GROBID_REBUILD_ENV_VAR);
    // Everything else (GROBID_VERSION, fingerprints…) is handled inside build.rs.

    Ok(())
}
