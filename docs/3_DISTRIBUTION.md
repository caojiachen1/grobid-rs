# Packaging and Distribution of grobid-rs

## 1. Introduction

This guide covers how to package and distribute a Rust application that embeds Grobid via JNI, with considerations for various platforms, packaging strategies, and licensing requirements.

## 2. Packaging Components

### 2.1. Java Runtime Environment (JRE)

Your application needs a JRE to execute the embedded Grobid Java code. There are several approaches:

#### 2.1.1. User-Provided JRE/JDK

- **Implementation:** Require users to have a compatible JDK/JRE installed and properly configured
- **Pros:** Minimal distribution size, simplifies packaging
- **Cons:** Worse user experience, potential compatibility issues, requires additional setup instructions
- **Code Example:**
  ```rust
  fn locate_system_jre() -> Result<PathBuf> {
      if let Ok(java_home) = std::env::var("JAVA_HOME") {
          let path = PathBuf::from(java_home);
          if path.exists() {
              return Ok(path);
          }
      }
      
      // Try common installation locations based on platform
      // ...
      
      Err(anyhow!("Could not locate Java installation. Please set JAVA_HOME."))
  }
  ```

#### 2.1.2. Bundled Custom JRE (Recommended)

- **Implementation:** Use `jlink` to create a minimal, customized JRE with only the required Java modules
- **Pros:** Self-contained application, consistent Java version, better user experience
- **Cons:** Larger distribution size, more complex build process
- **Code Example:**
  ```rust
  fn create_jlink_runtime(output_dir: &Path) -> Result<()> {
      let modules = "java.base,java.logging,java.xml,jdk.unsupported,java.naming,java.desktop,java.sql";
      
      let jlink_cmd = Command::new("jlink")
          .args(["--module-path", &jmods_path.to_string_lossy()])
          .args(["--add-modules", modules])
          .args(["--strip-debug", "--no-header-files", "--no-man-pages", "--compress=2"])
          .args(["--output", &output_dir.to_string_lossy()])
          .output()?;
      
      if !jlink_cmd.status.success() {
          return Err(anyhow!("jlink failed: {}", String::from_utf8_lossy(&jlink_cmd.stderr)));
      }
      
      Ok(())
  }
  ```

### 2.2. Grobid Resources

#### 2.2.1. JAR Files

- **grobid-core-{version}-onejar.jar:** The main Grobid library (typically 50-100MB)
- **Deployment:** Bundle with your application in a resources or vendor directory
- **Configuration:** Set `-Djava.class.path` to point to this JAR during JVM initialization

#### 2.2.2. grobid-home Directory

Due to its large size (hundreds of MB), there are three strategies for handling `grobid-home`:

1. **User-Provided (Development/Testing):**
   - User downloads/configures `grobid-home` separately
   - Application accepts a path via argument or environment variable

2. **Bundled and Extracted (Production/End-user):**
   - Compress `grobid-home` and include it with your application
   - Extract on first run to a user-specific data directory
   - Increases application size but creates a better user experience

3. **On-demand Download (Hybrid):**
   - Application offers to download required resources on first run
   - Provides a progress indicator and verification of downloaded assets

### 2.3. Native Libraries

#### 2.3.1. Wapiti JNI Libraries

- **Location:** Within `grobid-home/lib/<platform>/`
- **Deployment:** Included when bundling `grobid-home`
- **Configuration:** `-Djava.library.path` must point to the platform-specific directory

#### 2.3.2. pdfalto Executables

- **Location:** Within `grobid-home/pdfalto/<platform>/`
- **Deployment:** Included when bundling `grobid-home`
- **Permissions:** Must be executable (ensure correct permissions after extraction)
- **Licensing:** GPL-3.0 (see licensing section)

## 3. Cross-Platform Distribution

### 3.1. Platform Matrix

For broad compatibility, consider packaging for:

| Platform      | Architecture      | JRE/Libraries Path                    | Special Considerations               |
|---------------|-------------------|---------------------------------------|--------------------------------------|
| Linux         | x86_64            | lib/lin-64                            | Dynamic linking, GLIBC compatibility |
| Linux         | aarch64 (ARM64)   | lib/lin-arm64                         | Less common, but important for servers|
| macOS         | x86_64            | lib/mac-64                            | Code signing, notarization          |
| macOS         | aarch64 (M1/M2)   | lib/mac_arm-64                        | Native performance on Apple Silicon |
| Windows       | x86_64            | lib/win-64                            | Path handling, DLL dependencies     |

### 3.2. Platform-Specific Code

Use Rust's conditional compilation to handle platform differences:

```rust
#[cfg(target_os = "macos")]
fn get_platform_dir() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "mac_arm-64"
    } else {
        "mac-64"
    }
}

#[cfg(target_os = "windows")]
fn get_platform_dir() -> &'static str {
    "win-64"
}

#[cfg(target_os = "linux")]
fn get_platform_dir() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "lin-arm64"
    } else {
        "lin-64"
    }
}
```

### 3.3. Distribution Methods

- **Binary Release:** Precompiled binaries for each supported platform
- **Package Managers:** Homebrew (macOS), Apt/RPM (Linux), Chocolatey/Winget (Windows)
- **Docker Container:** For server deployments or CI/CD environments

## 4. Build and CI Considerations

### 4.1. Reproducible Builds

Ensure deterministic builds by:
- Pinning dependency versions in `Cargo.toml`
- Setting `SOURCE_DATE_EPOCH` during build
- Using deterministic compression (e.g., `zip -X` for JARs)

### 4.2. CI Pipeline

Set up a CI workflow that:
1. Builds your application on all target platforms
2. Downloads and verifies Grobid resources with checksums
3. Creates custom JREs for each platform
4. Packages everything into distribution-ready archives

### 4.3. Resource Verification

Always verify downloaded resources with checksums:

```rust
fn verify_checksum(path: &Path, expected_sha256: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash_hex = format!("{:x}", hasher.finalize());
    
    if hash_hex != expected_sha256 {
        return Err(anyhow!("Checksum verification failed"));
    }
    
    Ok(())
}
```

## 5. Licensing Considerations

Your distribution must comply with the licenses of all included components:

### 5.1. Component Licenses

| Component                | License              | Implications                                           |
|--------------------------|----------------------|--------------------------------------------------------|
| Your Rust Application    | Your choice          | Defines the overall licensing terms                    |
| jni-rs Crate             | MIT/Apache-2.0       | Permissive, typically no issues                        |
| Grobid Core              | Apache-2.0           | Include license and NOTICE file                        |
| Wapiti                   | BSD-like             | Include license notice                                 |
| pdfalto                  | GPL-3.0              | Provide source code or written offer                   |
| OpenJDK (bundled JRE)    | GPL-2.0 w/CPE        | Include license and source code access                |

### 5.2. License Compliance Checklist

1. **Identify all components:** Create a comprehensive list of all third-party software included
2. **Determine license requirements:** Research the specific obligations for each license
3. **Include license texts:** Bundle all relevant license files in a `LICENSES` directory
4. **Source code access:** For GPL components, provide access to the source code
5. **Attribution:** Include proper attribution for all components in documentation
6. **Notice file:** Create a consolidated `NOTICE` file listing all components and their licenses

### 5.3. Special Considerations for pdfalto

The GPL-3.0 license of `pdfalto` requires special attention:

- **Separation:** Grobid invokes `pdfalto` as a separate executable, which is generally considered "mere aggregation"
- **Source Code:** When distributing `pdfalto` binaries, you must also provide the corresponding source code (or a written offer)
- **No Static Linking:** Never statically link GPL code with your non-GPL application

## 6. Installation Experience

### 6.1. First-Run Experience

Consider implementing a first-run wizard that:
1. Verifies all required components are present
2. Extracts or downloads missing resources
3. Checks for sufficient disk space before extraction
4. Sets up proper file permissions
5. Creates default configuration

### 6.2. Update Mechanism

For long-lived applications, implement a way to check for and apply updates:

```rust
pub async fn check_for_updates() -> Result<Option<UpdateInfo>> {
    let client = reqwest::Client::new();
    let response = client.get("https://api.github.com/repos/your-org/grobid-rs/releases/latest")
        .header("User-Agent", "grobid-rs-updater")
        .send()
        .await?;
    
    if response.status().is_success() {
        let release: serde_json::Value = response.json().await?;
        let latest_version = release["tag_name"].as_str()
            .ok_or_else(|| anyhow!("Invalid release format"))?;
        
        if latest_version != CURRENT_VERSION {
            return Ok(Some(UpdateInfo {
                version: latest_version.to_string(),
                download_url: release["assets"][0]["browser_download_url"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            }));
        }
    }
    
    Ok(None)
}
```

## 7. Security Considerations

- **Download Verification:** Always verify checksums of downloaded resources
- **Sandboxing:** Consider sandboxing `pdfalto` execution as it processes untrusted PDFs
- **Temporary Files:** Clean up temporary files after processing
- **Supply Chain:** Pin dependencies with lockfiles and use `cargo-audit` to check for vulnerabilities
- **Runtime Permissions:** Run with minimal required privileges

## 8. Documentation

Provide clear documentation for users, including:
- System requirements
- Installation instructions for each platform
- Configuration options
- Troubleshooting common issues
- License information and third-party attributions

## 9. Conclusion

Creating a distributable Rust application with embedded Grobid requires careful planning for packaging, platform support, and licensing compliance. By following the guidelines in this document, you can create a robust, cross-platform solution that provides a good user experience while respecting all license obligations.