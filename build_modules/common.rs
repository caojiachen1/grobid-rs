#![allow(dead_code)] // Allow dead code for now, as not all constants might be used immediately

pub use std::{
    env,
    fs::{self as fs, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub use anyhow::{bail, Context, Result};
pub use fs_extra::dir::{copy as copy_dir_contents, CopyOptions as DirCopyOptions};
pub use fs_extra::error::Error as FsExtraError;
pub use indicatif::{ProgressBar, ProgressState, ProgressStyle};
pub use rayon::prelude::*;
pub use reqwest::{blocking::Client, header, StatusCode};
pub use sha2::{Digest, Sha256};
pub use zip::ZipArchive;

// ----------- Grobid Configuration -----------
pub const GROBID_VERSION: &str = "0.8.2";
pub const GROBID_RELEASE_TAG: &str = "0.8.2"; // Used for constructing onejar name
pub const GROBID_DOWNLOAD_URL_PREFIX: &str =
    "https://github.com/kermitt2/grobid/archive/refs/tags/";
pub const GROBID_ZIP_SHA256: &str =
    "19397444474e7456fc6fa416fb4aa22ba88f34979e7e9cd8f97aa81a28f2d2f2";

// ----------- Environment Variables -----------
pub const GROBID_RS_ASSETS_PATH_ENV_VAR: &str = "GROBID_RS_ASSETS_PATH";
pub const FORCE_GROBID_REBUILD_ENV_VAR: &str = "FORCE_GROBID_REBUILD";
pub const JAVA_HOME_ENV_VAR: &str = "JAVA_HOME";

// ----------- Directory and File Names -----------
// Root directory for all Grobid related assets within the main assets_dir
pub const GROBID_DIR_NAME_PREFIX: &str = "grobid-";
// Subdirectory within GROBID_DIR_NAME for the extracted source code
pub const GROBID_SOURCE_SUBDIR_NAME: &str = "source";
// Subdirectory within GROBID_DIR_NAME for the jlink runtime
pub const JLINK_RUNTIME_SUBDIR_NAME: &str = "runtime";
// Name of the grobid-home directory
pub const GROBID_HOME_DIR_NAME: &str = "grobid-home";
// Prefix for the main Grobid JAR file
pub const GROBID_JAR_NAME_PREFIX: &str = "grobid-core";
// Suffix for the Grobid onejar
pub const GROBID_ONEJAR_NAME_SUFFIX: &str = "-onejar.jar";
// Marker file to indicate successful extraction of Grobid source
pub const EXTRACTION_SUCCESS_MARKER_FILE: &str = ".extraction_successful";
// Marker file to indicate successful Grobid build (JAR and grobid-home copied)
pub const BUILD_SUCCESS_MARKER_FILE: &str = ".build_successful";
// Marker file to indicate successful JRE build
pub const JRE_SUCCESS_MARKER_FILE: &str = ".jre_successful";

// ----------- JLink Configuration -----------
// Modules for jlink. Note: java.xml.bind and java.activation were replaced by
// jakarta.xml.bind and jakarta.activation.api respectively in Java 11+
pub const JAKARTA_JLINK_MODULES: &str = "java.base,java.logging,java.xml,jdk.unsupported,java.naming,java.desktop,java.sql,java.management";

// ----------- Cargo Output Variables -----------
pub const CARGO_RERUN_IF_CHANGED_ENV_VAR: &str = "CARGO_RERUN_IF_CHANGED";
pub const CARGO_RERUN_IF_ENV_CHANGED_ENV_VAR: &str = "CARGO_RERUN_IF_ENV_CHANGED";
pub const CARGO_WARNING_PREFIX: &str = "cargo:warning=";
pub const CARGO_LINK_SEARCH_NATIVE_PREFIX: &str = "cargo:rustc-link-search=native=";
pub const CARGO_LINK_LIB_STATIC_PREFIX: &str = "cargo:rustc-link-lib=static=";
pub const CARGO_LINK_LIB_DYLIB_PREFIX: &str = "cargo:rustc-link-lib=dylib=";

// ----------- Utility Functions (moved here for now, might go to utils.rs later) -----------

pub fn print_cargo_warning(message: &str) {
    println!("{CARGO_WARNING_PREFIX}{message}");
}
