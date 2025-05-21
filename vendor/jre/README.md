# JRE Vendored Files

This directory contains minimal JRE (Java Runtime Environment) components necessary for building and running `grobid-rs` without requiring a full download of the OpenJDK.

## Purpose

The files in this directory enable:
- Offline builds without network access
- Faster builds by avoiding the download and extraction of OpenJDK
- Consistent runtime environment across different platforms
- Minimal Java runtime tailored specifically for Grobid's needs

## Contents

This directory should contain:

- `bin/` - Essential JRE executables (java, jlink, etc.)
- `lib/` - Core JRE libraries needed to run Grobid

## Usage

When present, the `build.rs` script will use these JRE components instead of downloading and configuring a full JRE. If you need to recreate or modify the JRE, you should set `FORCE_GROBID_REBUILD=true` to bypass these vendored files.

## Updating

To update these files:

1. Set `FORCE_GROBID_REBUILD=true` environment variable
2. Run a full build to download and create a minimal JRE using jlink
3. Copy the minimal required JRE components from the build output to this directory
4. Update the version information in this README if necessary

## Current Version

These files are from OpenJDK version 11.

## License

The OpenJDK components included here are subject to the GPLv2 with Classpath Exception. See the top-level LICENSE file and documentation for details.