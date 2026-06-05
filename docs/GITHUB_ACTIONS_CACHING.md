# GitHub Actions Caching for Grobid

This document outlines how we use GitHub Actions caching to improve build performance for Grobid in our CI.

## Caching Strategy

Our approach is to:

1. Leverage the vendored Grobid files in the repository (no download needed)
2. Cache only the platform-specific JRE runtime built with jlink

This strategy:
- Avoids downloading the large Grobid bundle (~1 GB) repeatedly
- Eliminates network dependency on external GitHub releases
- Ensures consistency across all matrix jobs
- Still benefits from caching for the platform-specific components

## Caching Mechanism Comparison

| Solution | Lifetime / limits | Pros | Cons |
| --- | --- | --- | --- |
| actions/cache | Evicted after 7 days of no access; total cache quota ≈ 10 GB per repo, each entry ≤ 5 GB | Automatic restore/save in one step; works across branches & PRs; no manual clean-up | Large files count against the 10 GB quota; rebuilt if untouched > 7 days |
| Artifacts (upload-artifact) | Retention up to 90 days (configurable) | Good for attaching the bundle to each run for later inspection | Not restored automatically—you must download it with a second job |
| Release asset + gh release download | Permanent once you cut a release | Zero quota pressure on the Actions cache; available to anyone | Requires you to maintain a tagged release for every Grobid version |

For day-to-day CI, we use `actions/cache` for the JRE runtime and rely on vendored files for Grobid assets.

## Implementation

Our workflow includes the following steps:

```yaml
env:
  GROBID_VERSION: '0.9.1'  # Used for JRE cache keys

# Cache JRE runtime by platform
- name: Cache JRE runtime
  id: jre-cache
  uses: actions/cache@v4
  with:
    path: ${{ github.workspace }}/vendor/jre/${{ matrix.os }}
    key: jre-${{ runner.os }}-${{ matrix.target }}-0.9.1

# Build minimal JRE via jlink only if not in cache
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

1. The **Restore step** tries to retrieve the cached JRE runtime for the specific platform (identified by runner.os and target).

2. The **Build step** runs only on a cache miss, creating a minimal JRE runtime with just the required modules.

3. On job completion, the cache is automatically saved for future runs.

Each matrix job has its own platform-specific JRE cache, but all jobs use the same vendored Grobid files from the repository.

## Things to Watch Out For

### Vendored Files
- Ensure your vendored files are kept up-to-date with the Grobid version you want to use
- The repository now contains the Grobid files, which increases its size but improves CI reliability

### Cache Expiry
- After 7 days of inactivity, GitHub purges JRE cache entries
- For infrequent CI, you might want to consider GitHub Actions scheduled workflows to keep the cache fresh

### Version Bumps
- When upgrading Grobid, update the version number in both the workflow and in vendored files
- Update the JRE cache key when you modify the JRE configuration to force a rebuild

## Optimizations

### Pre-compressed Vendor Directory
Our build script accepts `.zst` compressed files; compressing the extracted grobid-home and JRE cuts the repository size dramatically. The build.rs script automatically handles decompression as needed.

## TL;DR

We use a hybrid approach: vendored Grobid files plus cached JRE runtimes. This makes CI builds network-free and 10–15× faster with zero changes to Rust build logic and excellent reliability.