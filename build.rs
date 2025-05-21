use std::{env, fs, io, path::{Path, PathBuf}, process::Command};

const GROBID_VERSION: &str = "0.8.2";
const GROBID_RELEASE_TAG: &str = "v0.8.2";

fn get_os_specific_runtime_dir_name() -> &'static str {
    match env::consts::OS {
        "linux" => "ubuntu-latest",
        "macos" => "macos-14",
        "windows" => "windows-latest",
        _ => panic!("Unsupported OS for jlink runtime path determination in build.rs"),
    }
}

fn download_file(url: &str, to: &Path) -> Result<(), String> {
    println!("cargo:warning=Downloading {} to {}", url, to.display());
    let response = reqwest::blocking::get(url)
        .map_err(|e| format!("Failed to download {}: {}", url, e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to download {}: status {}", url, response.status()));
    }
    let mut dest_file = fs::File::create(to)
        .map_err(|e| format!("Failed to create file {}: {}", to.display(), e))?;
    let content = response.bytes()
        .map_err(|e| format!("Failed to read bytes from download {}: {}", url, e))?;
    io::copy(&mut content.as_ref(), &mut dest_file)
        .map_err(|e| format!("Failed to write to {}: {}", to.display(), e))?;
    Ok(())
}

fn extract_grobid_assets(zip_path: &Path, grobid_base_dir: &Path) -> Result<(), String> {
    println!("cargo:warning=Extracting Grobid assets from {} to {}", zip_path.display(), grobid_base_dir.display());
    let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open zip {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive {}: {}", zip_path.display(), e))?;

    let core_jar_name_in_zip = format!("grobid-{}/grobid-core/build/libs/grobid-core-{}-onejar.jar", GROBID_VERSION, GROBID_VERSION);
    let home_dir_name_in_zip_prefix = format!("grobid-{}/grobid-home/", GROBID_VERSION);

    let target_core_jar_path = grobid_base_dir.join("grobid-core.jar");
    let target_grobid_home_path = grobid_base_dir.join("grobid-home");

    if !target_grobid_home_path.exists() {
        fs::create_dir_all(&target_grobid_home_path)
            .map_err(|e| format!("Failed to create dir {}: {}", target_grobid_home_path.display(), e))?;
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if file.enclosed_name().is_none() {
            continue;
        }

        if file.name() == core_jar_name_in_zip {
            println!("cargo:warning=Extracting Grobid core JAR to {}", target_core_jar_path.display());
            let mut outfile = fs::File::create(&target_core_jar_path)
                .map_err(|e| format!("Failed to create {}: {}", target_core_jar_path.display(), e))?;
            io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to copy to {}: {}", target_core_jar_path.display(), e))?;
        } else if file.name().starts_with(&home_dir_name_in_zip_prefix) {
            let relative_path = PathBuf::from(file.name().strip_prefix(&home_dir_name_in_zip_prefix).unwrap());
            let target_path = target_grobid_home_path.join(relative_path);

            if file.is_dir() {
                fs::create_dir_all(&target_path).map_err(|e| format!("Failed to create dir {}: {}", target_path.display(), e))?;
            } else {
                if let Some(p) = target_path.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p).map_err(|e| format!("Failed to create parent dir {}: {}", p.display(), e))?;
                    }
                }
                let mut outfile = fs::File::create(&target_path)
                    .map_err(|e| format!("Failed to create {}: {}", target_path.display(), e))?;
                io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to copy to {}: {}", target_path.display(), e))?;
            }
        }
    }
    Ok(())
}

fn build_jlink_runtime(java_home: &Path, runtime_output_dir: &Path) -> Result<(), String> {
    println!("cargo:warning=Building jlink runtime at {}", runtime_output_dir.display());
    let jlink_exe = java_home.join("bin/jlink");
    if !jlink_exe.exists() {
        return Err(format!("jlink executable not found at {}", jlink_exe.display()));
    }

    if runtime_output_dir.exists() {
        fs::remove_dir_all(runtime_output_dir)
            .map_err(|e| format!("Failed to clean existing runtime dir {}: {}", runtime_output_dir.display(), e))?;
    }
    fs::create_dir_all(runtime_output_dir)
        .map_err(|e| format!("Failed to create runtime dir {}: {}", runtime_output_dir.display(), e))?;

    let status = Command::new(jlink_exe)
        .arg("--add-modules").arg("java.base,java.logging,java.xml,jdk.unsupported")
        .arg("--strip-debug")
        .arg("--no-header-files")
        .arg("--no-man-pages")
        .arg("--compress=2")
        .arg("--output").arg(runtime_output_dir)
        .status()
        .map_err(|e| format!("Failed to execute jlink: {}", e))?;

    if !status.success() {
        return Err(format!("jlink execution failed with status: {}", status));
    }
    Ok(())
}

fn main() {
    dotenv::dotenv().ok(); // Load .env file if present

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let assets_base_dir = out_dir.join("grobid_deps");
    let grobid_dir = assets_base_dir.join("grobid");
    let runtime_dir = assets_base_dir.join("runtime");
    let os_specific_runtime_name = get_os_specific_runtime_dir_name();
    let final_runtime_os_dir = runtime_dir.join(os_specific_runtime_name);

    let marker_file_content = format!("setup_complete_v{}", GROBID_VERSION);
    let marker_file_path = assets_base_dir.join(format!("{}.marker", marker_file_content));

    if !marker_file_path.exists() {
        println!("cargo:warning=Grobid assets not found or version mismatch, setting up in {}", assets_base_dir.display());

        if assets_base_dir.exists() { // Clean up old/partial assets
            fs::remove_dir_all(&assets_base_dir).expect("Failed to clean up old assets directory");
        }
        fs::create_dir_all(&assets_base_dir).expect("Failed to create assets base directory");
        fs::create_dir_all(&grobid_dir).expect("Failed to create Grobid directory");
        fs::create_dir_all(&runtime_dir).expect("Failed to create runtime directory");

        // 1. Download Grobid ZIP
        let grobid_zip_filename = format!("grobid-{}.zip", GROBID_RELEASE_TAG);
        let grobid_download_url = format!(
            "https://github.com/kermitt2/grobid/releases/download/{}/{}",
            GROBID_RELEASE_TAG, grobid_zip_filename
        );
        let temp_zip_path = out_dir.join(&grobid_zip_filename);
        download_file(&grobid_download_url, &temp_zip_path)
            .unwrap_or_else(|e| panic!("Grobid download failed: {}", e));

        // 2. Extract Grobid core JAR and grobid-home
        extract_grobid_assets(&temp_zip_path, &grobid_dir)
            .unwrap_or_else(|e| panic!("Grobid extraction failed: {}", e));
        fs::remove_file(&temp_zip_path).ok(); // Clean up downloaded zip

        // 3. Locate JAVA_HOME for jlink
        let java_home_path = match env::var("JAVA_HOME") {
            Ok(path) => PathBuf::from(path),
            Err(_) => java_locator::locate_java_home().map(PathBuf::from)
                .unwrap_or_else(|e| panic!("JAVA_HOME not set and could not be located: {}", e)),
        };
        println!("cargo:warning=Using JAVA_HOME: {}", java_home_path.display());
        
        // 4. Build jlink runtime
        build_jlink_runtime(&java_home_path, &final_runtime_os_dir)
            .unwrap_or_else(|e| panic!("jlink runtime build failed: {}", e));
        
        // 5. Create marker file
        fs::write(&marker_file_path, &marker_file_content)
            .unwrap_or_else(|e| panic!("Failed to write marker file {}: {}", marker_file_path.display(), e));

        println!("cargo:warning=Grobid assets setup complete in {}", assets_base_dir.display());
    } else {
        println!("cargo:warning=Grobid assets already present in {}", assets_base_dir.display());
    }

    println!("cargo:rustc-env=GROBID_RS_ASSETS_PATH={}", assets_base_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=JAVA_HOME");
    println!("cargo:rerun-if-changed=.env");


    // --- Existing JVM linking logic (for crate compilation, might be redundant if always using jlinked JVM via path) ---
    let java_home_for_linking = match env::var("JAVA_HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            // This part is mainly for compiling the jni crate itself or if `JavaVM::new` is used without specific path.
            // The `init` function should rely on the jlinked JVM via `GROBID_RS_ASSETS_PATH`.
            match java_locator::locate_java_home() {
                Ok(path_str) => PathBuf::from(path_str),
                Err(err) => {
                    // If assets are already downloaded, this linking part might not be critical,
                    // but it's good to have a consistent JAVA_HOME.
                    // If we panic here, it might be too strict if the jlinked one is already available.
                    // However, jni crate might still need it.
                    eprintln!("Warning: JAVA_HOME not set for build script's own linking, relying on previously located or jlinked JVM. Error: {}", err);
                    return; // Or try to proceed if assets_base_dir is fine. For now, be less strict here.
                            // This path won't be hit if marker file exists and JAVA_HOME was set during initial setup.
                }
            }
        }
    };

    let server_dir = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "windows" => java_home_for_linking.join("bin/server"),
        "macos" => {
            let macos_jdk_home = java_home_for_linking.join("Contents/Home");
            if macos_jdk_home.join("lib/server").exists() {
                macos_jdk_home.join("lib/server")
            } else {
                java_home_for_linking.join("lib/server")
            }
        }
        _         => java_home_for_linking.join("lib/server"),
    };

    if server_dir.exists() {
        println!("cargo:rustc-link-search=native={}", server_dir.display());
        println!("cargo:rustc-link-lib=dylib=jvm");
    } else {
        // This is a warning because the jlinked JVM should be used by the application logic.
        // The jni crate's build might still have issues if it can't find a system JVM here.
        println!("cargo:warning=System JVM server directory not found at {} for native linking. The application should use the jlinked JVM from GROBID_RS_ASSETS_PATH.", server_dir.display());
    }
} 