//! Public API for grobid-rs
//!
//! This module contains the high-level API functions for interacting with GROBID.
//! Functions are organized into submodules by functionality.

pub mod common;
pub mod fulltext;
pub mod header;
pub mod references;

// Re-export commonly used functions for convenient access
