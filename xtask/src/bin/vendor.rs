//! Utility for vendoring Grobid and JRE files into the repository.
//!
//! This script extracts the minimal necessary files from a full Grobid build
//! and copies them to the vendor directory, so future builds can use these files
//! instead of downloading and building Grobid from scratch.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// Constants from build_modules/common.rs
const GROBID_VERSION: &str = "0.8.2";
const GROBID_HOME_DIR_NAME: &str = "grobid-home";
const GROBID_JAR_NAME_PREFIX: &str = "grobid-core";
const GROBID_ONEJAR_NAME_SUFFIX: &str = "-onejar.jar";
const JLINK_RUNTIME_SUBDIR_NAME: &str = "runtime";
const GROBID_RS_ASSETS_PATH_ENV_VAR: &str = "GROBID_RS_ASSETS_PATH";
const FORCE_GROBID_REBUILD_ENV_VAR: &str = "FORCE_GROBID_REBUILD";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Vendoring minimal Grobid files for offline builds...");
    
    // Get the project root directory (parent of xtask directory)
    let current_dir = env::current_dir()?;
    let project_root = current_dir.parent()
        .ok_or("Expected to be run from the xtask directory")?;
    
    println!("Project root: {}", project_root.display());
    
    // Locate the Grobid deployment directory
    println!("Step 1: Locating Grobid deployment directory...");
    let grobid_dir = find_grobid_deployment_dir(project_root)?;
    println!("Found Grobid deployment directory: {}", grobid_dir.display());
    
    // Create the vendor directories
    println!("Step 2: Creating vendor directories...");
    let vendor_dir = project_root.join("vendor");
    let vendor_grobid_dir = vendor_dir.join("grobid");
    let vendor_jre_dir = vendor_dir.join("jre");
    
    fs::create_dir_all(&vendor_grobid_dir)?;
    fs::create_dir_all(&vendor_jre_dir)?;
    
    // Copy Grobid files
    println!("Step 3: Copying minimal Grobid files...");
    copy_grobid_files(&grobid_dir, &vendor_grobid_dir)?;
    
    // Copy JRE files
    println!("Step 4: Copying minimal JRE files...");
    copy_jre_files(&grobid_dir, &vendor_jre_dir)?;
    
    // Create success marker
    fs::write(vendor_dir.join(".vendor_complete"), 
             format!("Vendoring completed on {}", chrono::Local::now()))?;
    
    println!("\nVendoring complete! The minimal required files have been copied to the vendor directory.");
    println!("You can now commit these files to the repository for offline builds.");
    
    Ok(())
}

fn find_grobid_deployment_dir(project_root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // First check if the environment variable is set
    if let Ok(path) = env::var(GROBID_RS_ASSETS_PATH_ENV_VAR) {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            println!("Using Grobid directory from environment variable: {}", path_buf.display());
            return Ok(path_buf);
        }
    }
    
    // Check common locations
    let locations = [
        project_root.join("target/grobid_assets").join(format!("grobid-{}", GROBID_VERSION)),
        project_root.join("target/debug/grobid_assets").join(format!("grobid-{}", GROBID_VERSION)),
        project_root.join("target/release/grobid_assets").join(format!("grobid-{}", GROBID_VERSION)),
    ];
    
    for location in &locations {
        if location.exists() {
            return Ok(location.clone());
        }
    }
    
    // Not found in common locations, ask if we should build it
    println!("Grobid deployment directory not found in common locations.");
    print!("Do you want to run a build to generate it? (y/n): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        println!("Running build with FORCE_GROBID_REBUILD=true...");
        env::set_var(FORCE_GROBID_REBUILD_ENV_VAR, "true");
        
        let status = Command::new("cargo")
            .current_dir(project_root)
            .args(&["build"])
            .status()?;
            
        if !status.success() {
            return Err("Build failed".into());
        }
        
        // Check locations again
        for location in &locations {
            if location.exists() {
                println!("Build successful! Found Grobid deployment at: {}", location.display());
                return Ok(location.clone());
            }
        }
    }
    
    Err("Could not find or create Grobid deployment directory".into())
}

fn copy_grobid_files(source_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Copy main JAR file
    let jar_name = format!("{}-{}{}", GROBID_JAR_NAME_PREFIX, GROBID_VERSION, GROBID_ONEJAR_NAME_SUFFIX);
    let jar_path = source_dir.join(&jar_name);
    
    if jar_path.exists() {
        let target_jar_path = target_dir.join(&jar_name);
        println!("  Copying Grobid JAR: {} -> {}", jar_path.display(), target_jar_path.display());
        fs::copy(&jar_path, &target_jar_path)?;
    } else {
        // Try to find any JAR file with the right name pattern
        println!("  JAR not found at expected path: {}", jar_path.display());
        println!("  Searching for alternative JAR file...");
        
        let mut found = false;
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && 
               path.extension().map_or(false, |ext| ext == "jar") &&
               path.file_name().unwrap().to_string_lossy().contains(GROBID_JAR_NAME_PREFIX) {
                
                let file_name = path.file_name().unwrap();
                let target_jar_path = target_dir.join(file_name);
                println!("  Found alternative JAR: {}", path.display());
                println!("  Copying to: {}", target_jar_path.display());
                fs::copy(&path, &target_jar_path)?;
                found = true;
                break;
            }
        }
        
        if !found {
            return Err("Could not find any Grobid JAR file".into());
        }
    }
    
    // Copy grobid-home directory
    let grobid_home_path = source_dir.join(GROBID_HOME_DIR_NAME);
    if !grobid_home_path.exists() {
        return Err(format!("grobid-home directory not found at {}", grobid_home_path.display()).into());
    }
    
    let target_home_path = target_dir.join(GROBID_HOME_DIR_NAME);
    if target_home_path.exists() {
        println!("  Removing existing grobid-home directory");
        fs::remove_dir_all(&target_home_path)?;
    }
    
    println!("  Copying grobid-home directory");
    copy_dir_recursive(&grobid_home_path, &target_home_path)?;
    
    Ok(())
}

fn copy_jre_files(source_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_dir = source_dir.join(JLINK_RUNTIME_SUBDIR_NAME);
    
    if !runtime_dir.exists() {
        return Err(format!("JRE runtime directory not found at {}", runtime_dir.display()).into());
    }
    
    // Copy each platform-specific JRE
    for entry in fs::read_dir(&runtime_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Convert to String to avoid the Cow type issue
            let platform_name = path.file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
                
            println!("  Copying JRE for platform: {}", platform_name);
            
            let target_platform_dir = target_dir.join(&platform_name);
            if target_platform_dir.exists() {
                fs::remove_dir_all(&target_platform_dir)?;
            }
            
            copy_dir_recursive(&path, &target_platform_dir)?;
        }
    }
    
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}