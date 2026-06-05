# Managing Grobid Resources in Rust

## 1. Introduction

Successfully integrating Grobid with Rust requires proper management of various external resources. This document explains how to handle Grobid's resources, including the `grobid-home` directory, JARs, native libraries, and the `pdfalto` executable.

## 2. Essential Grobid Resources

### 2.1. `grobid-home` Directory

The `grobid-home` directory is the cornerstone of a Grobid installation, containing:

- **Machine Learning Models:** CRF models for parsing and extraction (e.g., header, citation, affiliation models)
- **Configuration Files:** `grobid.yaml` for settings, model paths, and processing options
- **Lexical Resources:** Dictionaries, language detection data, and other supplementary files
- **Native Libraries (`lib/`):** Platform-specific native libraries, primarily the Wapiti CRF engine JNI wrapper
- **pdfalto Directory:** Platform-specific executables for PDF to ALTO XML conversion

Key considerations:
- The content of `grobid-home` is tightly coupled with the version of `grobid-core.jar`
- The directory can be quite large (hundreds of megabytes) due to the models
- Path must be communicated to the JVM via `-Dorg.grobid.home=/path/to/grobid-home`

### 2.2. Grobid JAR Files

- **grobid-core-{version}-onejar.jar:** The main Grobid library with all dependencies bundled
- Must be included in the JVM classpath using `-Djava.class.path`
- Version must match the `grobid-home` directory version

### 2.3. Native Libraries

- **Wapiti JNI Library:** Platform-specific shared libraries (`libwapiti.so`, `libwapiti.dylib`, `wapiti.dll`)
- Located in `grobid-home/lib/<platform-specific-directory>/`
- Path must be in `-Djava.library.path` for the JVM to find them

### 2.4. Java Runtime (JRE)

- Required to run the Grobid Java code
- Can be system-provided or bundled (recommended via `jlink`)
- Contains the `libjvm` shared library needed for JNI

## 3. Resource Management Strategies

### 3.1. Managing `grobid-home`

There are three main approaches to handling `grobid-home`:

1. **User-Provided Path (Development/Flexibility):**
   - User specifies path via CLI argument, environment variable, or config file
   - Pros: Flexible, reduces distribution size
   - Cons: Less convenient for end-users

2. **Bundling and Extraction (Self-Contained Applications):**
   - Package `grobid-home` (compressed) with application assets
   - Extract on first run to application-specific data directory
   - Pros: Self-contained, better user experience
   - Cons: Large distribution size, complex extraction logic

3. **Hybrid Approach:**
   - Try user-provided path first, fall back to embedded/extracted version

Example implementation for path configuration:
```rust
// Get grobid_home path from various sources
let grobid_home_path = match std::env::var("GROBID_HOME") {
    Ok(path) => PathBuf::from(path),
    Err(_) => {
        if let Some(path) = cmd_args.grobid_home {
            path
        } else {
            // Fall back to default or extracted location
            let app_data = app_dirs2::app_root(
                AppDataType::UserData, 
                &AppInfo{name: "grobid-rs", author: "your-org"}
            )?;
            app_data.join("grobid-home")
        }
    }
};

// Verify the path is valid
if !grobid_home_path.exists() || !grobid_home_path.join("models").exists() {
    return Err(anyhow!("Invalid grobid-home directory: {}", grobid_home_path.display()));
}

// Use the path when initializing JVM
let grobid_home_arg = format!("-Dorg.grobid.home={}", grobid_home_path.display());
```

### 3.2. Managing `pdfalto`

The `pdfalto` tool is an external executable that Grobid uses to convert PDF to ALTO XML:

1. **Location:** Within `grobid-home/pdfalto/<platform>/` directory
2. **Execution:** Usually invoked by Grobid automatically, but can be called directly from Rust
3. **Permissions:** Must be executable (check and set if extracting from an archive)
4. **Licensing:** GNU GPL v3.0 (important licensing consideration)

Example of direct `pdfalto` invocation from Rust:
```rust
pub fn run_pdfalto(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError> {
    // Determine platform-specific executable name and path
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

    // Verify executable exists and has correct permissions
    if !bin.exists() {
        return Err(GrobidError::PdfAlto(format!("pdfalto binary not found at {}", bin.display())));
    }
    
    // Run pdfalto and check for errors
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
```

### 3.3. Build-Time Resource Management

For an optimized workflow, consider automating resource management in your build script:

```rust
// In build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("grobid_assets");
    
    // Download and extract Grobid if needed
    let grobid_dir = download_and_extract_grobid(&assets_dir, "0.9.1")?;
    
    // Create custom JRE with jlink
    let jre_dir = create_jlink_runtime(&grobid_dir)?;
    
    // Set environment variables for the main Rust code
    println!("cargo:rustc-env=GROBID_JAR_PATH={}", grobid_dir.join("grobid-core-0.9.1-onejar.jar").display());
    println!("cargo:rustc-env=GROBID_HOME_PATH={}", grobid_dir.join("grobid-home").display());
    println!("cargo:rustc-env=JLINK_RUNTIME_PATH={}", jre_dir.display());
    
    Ok(())
}
```

## 4. Java System Properties for Grobid Configuration

### 4.1. Essential JVM System Properties

- **`-Dorg.grobid.home`**: Path to the `grobid-home` directory
- **`-Djava.library.path`**: Path to native libraries (often `grobid-home/lib/<platform>`)
- **`-Djava.class.path`**: Path to the Grobid JAR file(s)

### 4.2. Additional Configuration Options

- **Memory Settings:** `-Xmx<size>` (e.g., `-Xmx1G` for 1GB heap)
- **Debug Options:** `-Xcheck:jni` for JNI debugging (not for production)
- **Headless Mode:** `-Djava.awt.headless=true` for server environments

### 4.3. Grobid-Specific Configuration

Expose these key Grobid configuration options to Rust users:

- **Consolidation:** Controls reference lookups (0=none, 1=fast, 2=full)
- **TEI Coordinates:** Include PDF coordinates in the output
- **Page Range:** Process only specific pages of the PDF
- **Language Hints:** Help model select appropriate language resources

## 5. Cross-Platform Considerations

### 5.1. Platform-Specific Paths

Handle different platforms by detecting the OS and architecture:

```rust
let lib_subdir = match (std::env::consts::OS, std::env::consts::ARCH) {
    ("windows", _) => "win-64",
    ("macos", "aarch64") => "mac_arm-64",
    ("macos", _) => "mac-64",
    ("linux", "aarch64") => "lin-arm64",
    ("linux", _) => "lin-64",
    _ => return Err(anyhow!("Unsupported platform: {}-{}", 
                            std::env::consts::OS, 
                            std::env::consts::ARCH)),
};

let lib_path = grobid_home_path.join("lib").join(lib_subdir);
```

### 5.2. Library Name Differences

Account for platform-specific library name conventions:

```rust
let jvm_lib = match std::env::consts::OS {
    "windows" => runtime_dir.join("bin/server/jvm.dll"),
    "macos"   => runtime_dir.join("lib/server/libjvm.dylib"),
    _         => runtime_dir.join("lib/server/libjvm.so"),
};
```

### 5.3. Executable Extensions

Handle executable name differences:

```rust
let pdfalto_exe = if std::env::consts::OS == "windows" {
    "pdfalto.exe"
} else {
    "pdfalto"
};
```

## 6. Advanced Resource Management

### 6.1. Hot-Swapping Models

Allow users to update or replace models without rebuilding:

```rust
// Check for model override
let models_dir = if let Ok(override_path) = std::env::var("GROBID_MODELS_OVERRIDE") {
    PathBuf::from(override_path)
} else {
    grobid_home_path.join("models")
};

// Add as system property
let models_arg = format!("-Dorg.grobid.models.dir={}", models_dir.display());
```

### 6.2. Caching and Deduplication

Implement caching to avoid reprocessing unchanged PDFs:

```rust
pub fn process_with_cache(pdf_path: &Path, cache_dir: &Path) -> Result<String, GrobidError> {
    // Calculate hash of PDF
    let mut file = std::fs::File::open(pdf_path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = format!("{:x}", hasher.finalize());
    
    // Check cache
    let cache_path = cache_dir.join(format!("{}.tei.xml", hash));
    if cache_path.exists() {
        return Ok(std::fs::read_to_string(cache_path)?);
    }
    
    // Process and cache
    let result = grobid_rs::fulltext_to_tei(pdf_path)?;
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(cache_path, &result)?;
    
    Ok(result)
}
```

### 6.3. Memory Optimization

Control memory usage for large-scale processing:

```rust
pub fn configure_memory(heap_mb: usize) -> String {
    format!("-Xmx{}m", heap_mb)
}
```

## 7. Best Practices

- **Version Consistency:** Ensure `grobid-home`, JAR, and native libraries have matching versions
- **Path Validation:** Always check resource paths before use and provide clear error messages
- **Resource Cleanup:** Clean up temporary files after processing
- **Error Handling:** Handle missing resources or configuration errors gracefully
- **License Compliance:** Respect the licenses of all components, especially GPL-licensed `pdfalto`
- **Security:** Validate checksums of downloaded resources
- **Performance:** Consider caching and optimized resource paths for high-volume processing

## 8. Summary

Proper resource management is crucial for a robust Grobid integration. By carefully handling `grobid-home`, JARs, native libraries, and platform-specific considerations, you can create a reliable, cross-platform Rust application that leverages Grobid's powerful document processing capabilities.