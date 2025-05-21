# GitHub Actions Caching for Grobid

This document explains how we cache the Grobid bundle inside GitHub Actions to significantly speed up CI workflows.

## Overview

Caching the Grobid bundle (~1 GB of models and the one-jar) is essential for efficient CI workflows. By using GitHub's caching mechanism, we:

- Eliminate repeated downloads of large files across matrix jobs
- Reduce build times by 10-15× after the first run
- Maintain consistency across branches and PRs
- Optimize GitHub Actions usage and reduce network load

## Caching Options Comparison

| Solution | Lifetime / limits | Pros | Cons |
|----------|-------------------|------|------|
| **actions/cache** | Evicted after 7 days of no access; total cache quota ≈ 10 GB per repo, each entry ≤ 5 GB | Automatic restore/save in one step; works across branches & PRs; no manual clean-up | Large files count against the 10 GB quota; rebuilt if untouched > 7 days |
| **Artifacts** (upload-artifact) | Retention up to 90 days (configurable) | Good for attaching the bundle to each run for later inspection | Not restored automatically—you must download it with a second job |
| **Release asset** + gh release download | Permanent once you cut a release | Zero quota pressure on the Actions cache; available to anyone | Requires you to maintain a tagged release for every Grobid version |

For day-to-day CI the first option (`actions/cache`) is simplest and usually well within the 10 GB budget.

## Implementation

Our workflow uses the following pattern:

```yaml
env:
  GROBID_VERSION: 0.8.2               # keep one source of truth
  GROBID_ZIP: grobid-${{ env.GROBID_VERSION }}-onejar.zip
  GROBID_CACHE_DIR: ${{ github.workspace }}/.grobid-cache

# -----------------------------------------------
# Restore (or later save) the Grobid ZIP
# -----------------------------------------------
- name: Restore Grobid cache
  id: grobid-cache
  uses: actions/cache@v4
  with:
    # anything placed here will be cached
    path: ${{ env.GROBID_CACHE_DIR }}
    key: grobid-${{ env.GROBID_VERSION }}-zip
    restore-keys: |
      grobid-${{ env.GROBID_VERSION }}

# -----------------------------------------------
# Fetch only if the cache miss occurred
# -----------------------------------------------
- name: Download Grobid release ZIP
  if: steps.grobid-cache.outputs.cache-hit != 'true'
  run: |
    mkdir -p "${GROBID_CACHE_DIR}"
    curl -fsSL -o "${GROBID_CACHE_DIR}/${GROBID_ZIP}" \
      "https://github.com/kermitt2/grobid/releases/download/v${GROBID_VERSION}/grobid-${GROBID_VERSION}-onejar.zip"
    # optional: verify SHA-256, then unzip into vendor/
    unzip -q "${GROBID_CACHE_DIR}/${GROBID_ZIP}" -d vendor/

# -----------------------------------------------
# Build as normal – your existing build.rs will
# pick up vendor/ automatically
# -----------------------------------------------
- name: cargo build
  run: cargo build --release --features cli
```

## How It Works

1. **Restore step** tries to untar the cache named `grobid-0.8.2-zip`.
   If it exists and is fresh, the ZIP appears instantly (from GitHub's CDN).
2. **Download step** runs only on a cache miss, fetching the ZIP once from GitHub Releases (or any mirror).
3. On job completion the same cache key is uploaded automatically, so the next run (even on another branch) will hit the cache.

The ZIP itself is platform-agnostic, so we deliberately omit `${{ runner.os }}` from the key to let all matrix jobs share one cache entry. GitHub takes care of deduplicating identical uploads.

## Things to Watch Out For

### Size & Quota
- A single cache entry may not exceed 5 GB and the sum of all caches in a repo is kept under 10 GB; excess entries are evicted LRU-style.
- If the Grobid bundle ever
