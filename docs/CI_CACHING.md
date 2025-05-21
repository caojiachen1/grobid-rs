# GitHub Actions Caching for Grobid

This document explains how we cache Grobid assets in GitHub Actions workflows to improve build speeds and reduce network usage.

## Overview

The Grobid bundle (~1 GB) includes extensive machine learning models and a one-jar that are downloaded during the build process. By caching these assets with GitHub's cache action, we:

- Speed up CI builds by 10-15× after the first run
- Reduce network usage and external dependencies
- Ensure consistency across all matrix jobs
- Minimize GitHub Actions billing minutes

## Implementation

Our caching strategy has two parts:

1. Cache the original Grobid ZIP file (~500 MB)
2. Cache the platform-specific minimal JREs (~30-40 MB each)

### Workflow Implementation

In our GitHub Actions workflow file:

```yaml
env:
  GROBID_VERSION: 0.8.2
  GROBID_ZIP: grobid-${{ env.GROBID_VERSION }}-onejar.zip
  GROBID_CACHE_DIR: ${{ github.workspace }}/.grobid-cache

steps:
  # Restore Grobid ZIP from cache
  - name: Restore Grobid cache
    id: grobid-cache
    uses: actions/cache@v4
    with:
      path: ${{ env.GROBID_CACHE_DIR }}
      key: grobid-${{ env.GROBID_VERSION }}-zip
      restore-keys: |
        grobid-${{ env.GROBID_VERSION }}

  # Download only on cache miss
  - name: Download Grobid release ZIP
    if: steps.grobid-cache.outputs.cache-hit != 'true'
    run: |
      mkdir -p "${{ env.GROBID_CACHE_DIR }}"
      curl -fsSL -o "${{ env.GROBID_CACHE_DIR }}/${{ env.GROBID_ZIP }}" \
        "https://github.com/kermitt2/grobid/releases/download/v${{ env.GROBID_VERSION }}/${{ env.GROBID_ZIP }}"

  # Extract to vendor directory
  - name: Extract Grobid
    run: |
      mkdir -p vendor/grobid
      unzip -q "${{ env.GROBID_CACHE_DIR }}/${{ env.GROBID_ZIP }}" -d tmp
      mv tmp/grobid-${{ env.GROBID_VERSION }}/grobid-core-${{ env.GROBID_VERSION }}-onejar.jar vendor/grobid/grobid-core-${{ env.GROBID_VERSION }}-onejar.jar
      mv tmp/grobid-${{ env.GROBID_VERSION }}/grobid-home vendor/grobid/

  # Cache JRE runtime too
  - name: Cache JRE runtime
    id: jre-cache
    uses: actions/cache@v4
    with:
      path: ${{ github.workspace }}/vendor/jre/${{ matrix.os }}
      key: jre-${{ runner.os }}-${{ matrix.target }}

  # Build JRE only on cache miss
  - name: Build jlink image
    if: steps.jre-cache.outputs.cache-hit != 'true'
    run: |
      mkdir -p vendor/jre/${{ matrix.os }}
      jlink \
        --add-modules java.base,java.logging,java.xml,jdk.unsupported \
        --strip-debug --no-header-files --no-man-pages --compress=2 \
        --output vendor/jre/${{ matrix.os }}
```

## How It Works

1. The workflow first tries to restore the Grobid ZIP from cache using a key based on the Grobid version.
2. If the cache is found, the ZIP is immediately available. If not, it's downloaded from GitHub Releases.
3. The ZIP is extracted to the vendor directory, which our build.rs script automatically detects.
4. Similarly, we cache the platform-specific JRE runtime created with jlink.
5. At the end of the workflow, these caches are automatically saved for future runs.

## Key Benefits

- **Platform Agnostic**: The Grobid ZIP is cached once and shared across all matrix jobs
- **Version Management**: When the Grobid version changes, a new cache entry is created automatically
- **Cross-Branch Sharing**: The cache works across branches and PRs (external PRs can read but not write)
- **Minimal Maintenance**: No manual invalidation needed as keys change with version updates

## Cache Limitations

- **Maximum Size**: Each entry must be under 5 GB (not an issue for us)
- **Total Quota**: GitHub limits repositories to ~10 GB total cache (not a concern with our strategy)
- **Expiration**: Unused cache entries expire after 7 days of inactivity
- **External PRs**: Forks can read but not write to the cache (falling back to download)

## Optimizations

For our specific use-case, we've implemented these optimizations:

1. **Single Cache Entry**: By omitting the platform from the Grobid cache key, all matrix jobs share one cache 
2. **Minimal JRE**: We build and cache minimal JREs for each platform to reduce size
3. **Version-Specific Keys**: Cache keys include version numbers for automatic invalidation

## Future Improvements

Potential improvements to the caching strategy:

1. **Pre-compressed vendor directory**: Store .zst compressed versions of the extracted files to reduce cache size
2. **Release asset hosting**: For long-lived projects, host the bundle ourselves as a GitHub Release asset
3. **Partial extractions**: Only extract the specific models needed for tests to reduce CI storage needs