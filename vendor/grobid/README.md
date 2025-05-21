# Grobid Vendored Files

This directory contains minimal pre-built Grobid components necessary for building and running `grobid-rs` without requiring a full download of the Grobid source code.

## Purpose

The files in this directory enable:
- Offline builds without network access
- Faster builds by avoiding the download and compilation of Grobid
- Consistent builds across different environments

## Contents

This directory should contain:

- `core/` - Essential JAR files and compiled Grobid components
- `models/` - Pre-trained machine learning models used by Grobid

## Usage

When present, the `build.rs` script will use these files instead of downloading the full Grobid package. If you need to modify Grobid itself, you should set `FORCE_GROBID_REBUILD=true` to bypass these vendored files.

## Updating

To update these files:

1. Set `FORCE_GROBID_REBUILD=true` environment variable
2. Run a full build to download and compile the latest Grobid
3. Copy the minimal required files from `target/*/grobid_assets/` to this directory
4. Update the version information in this README if necessary

## Current Version

These files are from Grobid version 0.8.2.

## License

The Grobid files included here are subject to the Apache License 2.0. See the top-level LICENSE file and documentation for details.