# Static, Cross-arch Release Bundles (musl + Zig + mold)

## Motivation

A zero-dependency, static binary install story is a game-changer for developer experience and adoption. Users should be able to `curl | tar | ./grobid-cli` on any platform and get started instantly, with no system dependencies, Python, or JVM setup. This removes 80% of support issues and makes Grobid-RS suitable for CI, cloud, and enterprise environments.

## What You Ship

| Path         | Contents                                  |
|--------------|-------------------------------------------|
| bin/         | grobid-cli, grobid-server (axum)          |
| runtime/     | jlink'd JRE or native models              |
| models/      | Wapiti/ONNX/Tokenizer files               |
| completions/ | bash, zsh, fish, pwsh completions         |
| README, LICENSE, CHANGELOG | Docs and legal info         |

Each release artifact is a self-contained tar/zip per target triple, e.g.:
```
grobid-rs-v0.5.2-x86_64-unknown-linux-musl.tar.zst
```

## Tooling Choices

- **Cargo-dist (easy path):**
  - One-command releases, signs, uploads assets
  - Handles musl static builds (zig linker flag)
- **Hand-rolled xtask dist (power path):**
  - Extend your existing xtask crate
  - Embed extra post-steps (strip, upx)
  - Create checksums / SBOM
  - Push to Homebrew/Scoop

## Engineering Tasks Checklist

- [ ] 1️⃣  Build matrix in CI: linux-musl, linux-glibc, darwin, windows
- [ ] 2️⃣  Switch to zig cc + musl for Linux static
- [ ] 3️⃣  Teach build.rs to "bundle" runtime & models under $OUT_DIR/bundle
- [ ] 4️⃣  Add smoke-test step: `./grobid-cli --version` on the unpacked bundle
- [ ] 5️⃣  Publish workflow on tag push

## Implementation Plan

### 1. Define Artifact Layout

```
bin/           # CLI binaries (grobid-cli, grobid-server, etc.)
runtime/       # JVM or model runtime files (if needed)
models/        # ML models (CRF, ONNX, etc.)
completions/   # Shell completion scripts
README.md      # Usage instructions
```

### 2. Build System Setup
- Add musl as a target: `rustup target add x86_64-unknown-linux-musl`
- Install Zig: `brew install zig` or download from ziglang.org
- Use `cargo-zigbuild` for cross-compilation:
  ```sh
  cargo install cargo-zigbuild
  cargo zigbuild --release --target x86_64-unknown-linux-musl
  ```
- Use mold for faster linking (optional):
  ```sh
  export RUSTFLAGS="-C linker=mold -C link-arg=-fuse-ld=mold"
  ```

### 3. CI/CD Integration
- Add a GitHub Actions workflow to build for all targets:
  - Linux (musl, glibc)
  - macOS (amd64, arm64)
  - Windows (x86_64)
- Use `actions/upload-artifact` or `cargo-dist` to package tarballs/zip files.
- Test install and run on clean containers/VMs.

### 4. Zero-Dependency Experience
- Ensure all binaries are statically linked (check with `ldd`).
- Bundle all required runtime/model files.
- Provide shell completions and README in the tarball.
- Document install in README:
  ```sh
  curl -L https://github.com/yourorg/grobid-rs/releases/download/vX.Y.Z/grobid-cli-x86_64-unknown-linux-musl.tar.gz | tar xz
  ./grobid-cli --help
  ```

### 5. Troubleshooting & Platform Notes
- **JNI**: If using JNI, ensure the JRE is bundled or statically linked.
- **macOS**: Codesigning may be required for Apple Silicon.
- **Windows**: Use `cross` or GitHub runners for native builds.
- **musl quirks**: Some crates (e.g., openssl) may need special config for musl.

### 6. Optional: Use `cargo-dist`
- Add `cargo-dist` to your workspace for automated packaging and release notes.
- Configure `dist.toml` for artifact layout and metadata.

## Quick Next Steps

### Engineering Steps
1. **Finalize bundle layout**: Commit the artifact structure and update `xtask dist` or `cargo-dist` config.
2. **CI matrix builds**: Ensure all targets build and pass smoke tests (`./grobid-cli --version`).
3. **Switch to zig/musl**: For Linux, use zig as linker for static builds.
4. **Bundle runtime/models**: Update `build.rs` to copy runtime and model files into the bundle.
5. **Publish on tag**: Automate release on tag push, with checksums and release notes.

### Risks & Costs
- 3–5 days dev-time (most is CI yak-shaving)
- Bundle size: keep < 90 MB for GitHub release limit (use `strip` + `zstd -19`)

### Success Metrics
- "Time-to-first-parse" on a fresh VM ≈ 30 s
- GitHub issues labelled install drop
- Downloads of static tarballs exceed `cargo install`

### How You'll Feel the Impact
- Static bundles land → GitHub "it doesn't run on my server" issues nosedive the same week.
- CI/CD users can install and run Grobid-RS in one line.

## Quick Wins
- Provide a single tarball per platform.
- Add a `--version` flag that prints build info.
- Test on major CI providers (GitHub Actions, GitLab CI).

## References
- [musl libc](https://musl.libc.org/)
- [Zig](https://ziglang.org/)
- [mold linker](https://github.com/rui314/mold)
- [cargo-zigbuild](https://github.com/messense/cargo-zigbuild)
- [cargo-dist](https://github.com/axodotdev/cargo-dist) 