use std::{env, fs, path::PathBuf};

// Constants from build_modules/common.rs
const GROBID_RS_ASSETS_PATH_ENV_VAR: &str = "GROBID_RS_ASSETS_PATH";
const GROBID_DIR_NAME_PREFIX: &str = "grobid-";
const GROBID_VERSION: &str = "0.9.1";
const JLINK_RUNTIME_SUBDIR_NAME: &str = "runtime";
const JRE_SUCCESS_MARKER_FILE: &str = ".jre_successful";

fn main() -> anyhow::Result<()> {
    let assets_dir = locate_assets_dir()?;
    let grobid_dir = assets_dir.join(format!("{}{}", GROBID_DIR_NAME_PREFIX, GROBID_VERSION));
    let runtime_dir = grobid_dir.join(JLINK_RUNTIME_SUBDIR_NAME);
    let success_marker = grobid_dir.join(JRE_SUCCESS_MARKER_FILE);

    println!("🔍 Grobid assets directory: {}", assets_dir.display());
    println!("🔍 Grobid directory: {}", grobid_dir.display());
    println!("🔍 Runtime directory: {}", runtime_dir.display());

    // Remove runtime directory if it exists
    if runtime_dir.exists() {
        println!("🧹 Removing runtime directory: {}", runtime_dir.display());
        fs::remove_dir_all(&runtime_dir)?;
    } else {
        println!("ℹ️ Runtime directory doesn't exist, nothing to clean.");
    }

    // Remove success marker if it exists
    if success_marker.exists() {
        println!("🧹 Removing JRE success marker: {}", success_marker.display());
        fs::remove_file(&success_marker)?;
    }

    println!("✅ Clean completed successfully!");
    println!("\nTo rebuild the JRE runtime, run:\n  cargo build");

    Ok(())
}

/// Locate the Grobid assets directory
fn locate_assets_dir() -> anyhow::Result<PathBuf> {
    // Try environment variable first
    if let Ok(path) = env::var(GROBID_RS_ASSETS_PATH_ENV_VAR) {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            return Ok(path_buf);
        }
    }
    
    // Otherwise, fallback to the default location in target/
    let project_root = find_project_root()?;
    let default_path = project_root.join("target").join("grobid_assets");
    
    if !default_path.exists() {
        println!("⚠️  Warning: Default assets directory doesn't exist yet: {}", default_path.display());
        println!("   It will be created during the next build.");
        // Create it to ensure we can continue
        fs::create_dir_all(&default_path)?;
    }
    
    Ok(default_path)
}

/// Find the project root directory
fn find_project_root() -> anyhow::Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // If not running via cargo, try to find the project root
            let current_dir = env::current_dir().expect("Failed to get current directory");
            current_dir
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(PathBuf::from)
                .unwrap_or(current_dir)
        });
    
    // xtask is in a subfolder, so we need to go up one level
    let project_root = manifest_dir.parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to find project root from xtask directory"))?;
    
    Ok(project_root.to_path_buf())
}