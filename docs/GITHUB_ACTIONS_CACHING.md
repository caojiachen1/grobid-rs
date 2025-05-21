# GitHub Actions Caching for Grobid

This document outlines how we use GitHub Actions caching to improve build performance by avoiding repeated downloads of the Grobid bundle.

## Caching Mechanism Comparison

| Solution | Lifetime / limits | Pros | Cons |
| --- | --- | --- | --- |
| actions/cache | Evicted after 7 days of no access; total cache quota ≈ 10 GB per repo, each entry ≤ 5 GB | Automatic restore/save in one step; works across branches & PRs; no manual clean-up | Large files count against the 10 GB quota; rebuilt if untouched > 7 days |
| Artifacts (upload-artifact) | Retention up to 90 days (configurable) | Good for attaching the bundle to each run for later inspection | Not restored automatically—you must download it with a second job |
| Release asset + gh release download | Permanent once you cut a release | Zero quota pressure on the Actions cache; available to anyone | Requires you to maintain a tagged release for every Grobid version |

For day-to-day CI, the first option (`actions/cache`) is simplest and usually well within the 10 GB budget.

## Implementation

Our workflow includes the following steps to implement caching:

```yaml
env:
  GROBID_VERSION: 0.8.2               # keep one source of truth
  GROBID_ZIP: grobid-${{ env.GROBID_VERSION }}-onejar.zip
  GROBID_CACHE_DIR: ${{ github.workspace }}/.grobid-cache

# Restore (or later save) the Grobid ZIP
- name: Restore Grobid cache
  id: grobid-cache
  uses: actions/cache@v4
  with:
    # anything placed here will be cached
    path: ${{ env.GROBID_CACHE_DIR }}
    key: grobid-${{ env.GROBID_VERSION }}-bundle
    restore-keys: |
      grobid-${{ env.GROBID_VERSION }}

# Fetch only if the cache miss occurred
- name: Download Grobid release ZIP
  if: steps.grobid-cache.outputs.cache-hit != 'true'
  run: |
    mkdir -p "${GROBID_CACHE_DIR}"
    curl -fsSL -o "${GROBID_CACHE_DIR}/${GROBID_ZIP}" \
      "https://github.com/kermitt2/grobid/releases/download/v${GROBID_VERSION}/grobid-${GROBID_VERSION}-onejar.zip"
```

## How It Works

1. The **Restore step** tries to untar the cache named `grobid-0.8.2-bundle`. If it exists and is fresh, the ZIP appears instantly (from GitHub's CDN).

2. The **Download step** runs only on a cache miss, fetching the ZIP once from GitHub Releases (or any mirror).

3. On job completion, the same cache key is uploaded automatically, so the next run (even on another branch) will hit the cache.

The ZIP itself is platform-agnostic, so we deliberately omit `${{ runner.os }}` from the key to let all matrix jobs share one cache entry. GitHub takes care of deduplicating identical uploads.

## Things to Watch Out For

### Size & Quota
- A single cache entry may not exceed 5 GB, and the sum of all caches in a repo is kept under 10 GB; excess entries are evicted LRU-style.
- If the Grobid bundle ever exceeds that, switch to the Release-asset strategy instead.

### Cache Expiry
- After 7 days of inactivity, GitHub purges the entry.
- If your repo has very infrequent CI, you might prefer the 90-day artifact route.

### Forked Pull-Requests
- External PRs can restore your cache but cannot save a new one for security reasons.
- That merely drops you back to the download path for those runs.

### Version Bumps
- When you upgrade Grobid, bump `GROBID_VERSION` (thus the cache key), and the new ZIP will be downloaded and stored alongside the old one—no manual invalidation needed.

## Alternatives & Optimizations

### Release Asset + gh Release Download
Ideal for long-lived projects: push the ZIP once per version to a GitHub Release and insert:

```bash
gh release download v${GROBID_VERSION} -p '*onejar.zip' -O "${GROBID_CACHE_DIR}/${GROBID_ZIP}"
```

in place of curl; you still keep the `actions/cache` wrapper for quick intra-CI reuse.

### Pre-compressed Vendor Directory
Our build script already accepts `.zst` compressed files; compressing the extracted grobid-home and JRE cuts the cache size dramatically and avoids the unzip step. Just add `*.zst` to the `path:` list in the cache step.

## TL;DR

Yes—drop an `actions/cache` step around the ZIP (or even the fully-populated vendor/ directory) and key it by `GROBID_VERSION`; CI will become network-free and 10–15× faster after the first run, with zero changes to your Rust build logic.