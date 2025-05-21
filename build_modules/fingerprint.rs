//! Very small helper: (de)serialise a JSON file with the hash of every
//! input that should trigger a rebuild (Gradle sources, JDK, build.rs).

use crate::build_modules::common::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Fingerprint {
    pub grobid_version: String,
    pub grobid_zip_sha256: String,
    pub gradle_wrapper_sha256: String,
    pub jdk_release_file: Option<String>, // e.g. contents of $JAVA_HOME/release
    pub build_rs_mtime: u64,              // unix‐secs
    pub jlink_modules: String,            // modules used for jlink runtime
}

impl Fingerprint {
    pub fn current(java_home: &Path, gradlew: &Path) -> Result<Self> {
        // 1) Gradle wrapper
        let mut hasher = Sha256::new();
        if gradlew.exists() {
            let mut file = File::open(gradlew).with_context(|| {
                format!("Failed to open gradle wrapper at {}", gradlew.display())
            })?;
            io::copy(&mut file, &mut hasher).with_context(|| {
                format!("Failed to read gradle wrapper at {}", gradlew.display())
            })?;
        } else if gradlew.as_os_str().is_empty() {
            print_cargo_warning("No Gradle wrapper path provided, using empty hash");
        } else {
            print_cargo_warning(&format!(
                "Gradle wrapper not found at {}, using empty hash",
                gradlew.display()
            ));
        }
        let gradle_wrapper_sha256 = format!("{:x}", hasher.finalize());

        // 2) tiny text file shipped with every JDK
        let jdk_release_file = if java_home.join("release").is_file() {
            fs::read_to_string(java_home.join("release"))
                .with_context(|| {
                    format!(
                        "Failed to read JDK release file at {}",
                        java_home.join("release").display()
                    )
                })
                .ok()
        } else {
            None
        };

        // 3) build.rs mtime
        let manifest_dir =
            env::var("CARGO_MANIFEST_DIR").with_context(|| "CARGO_MANIFEST_DIR not set")?;
        let build_rs_path = PathBuf::from(manifest_dir).join("build.rs");

        let build_rs_mtime = if build_rs_path.exists() {
            let metadata = fs::metadata(&build_rs_path).with_context(|| {
                format!(
                    "Failed to get metadata for build.rs at {}",
                    build_rs_path.display()
                )
            })?;

            let modified = metadata
                .modified()
                .with_context(|| "Failed to get modification time for build.rs")?;

            let duration = modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .with_context(|| "Failed to calculate duration since epoch for build.rs")?;

            duration.as_secs()
        } else {
            print_cargo_warning("build.rs not found, using 0 as modification time");
            0
        };

        Ok(Self {
            grobid_version: GROBID_VERSION.to_owned(),
            grobid_zip_sha256: GROBID_ZIP_SHA256.to_owned(),
            gradle_wrapper_sha256,
            jdk_release_file,
            build_rs_mtime,
            jlink_modules: JAKARTA_JLINK_MODULES.to_owned(),
        })
    }
}
