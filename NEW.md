Yes — you can ditch Docker entirely and ship Grobid + a minimal JRE right inside your crate/release.

Below is the leanest, cross-platform way to do it, plus the few caveats that matter.

⸻

1 · What you need to vendor

Piece	Produced by	Size (unzipped)	Where to place
Grobid one-jar grobid-core-0.8.2-onejar.jar	./gradlew :grobid-core:shadowJar	~40 MB	vendor/grobid/
grobid-home (models, config, pdfalto, wapiti)	./gradlew assemble	~420 MB	vendor/grobid/grobid-home/
Trimmed JRE per OS/arch (made with jlink)	your CI	35-55 MB each	vendor/runtime/<triple>/e.g. runtime/linux-x86_64

Total per-platform payload ≈ 475 MB → zstd-compressed ≈ 140 MB.

⸻

2 · Change build.rs (minimal diff)

- let use_vendored = check_for_vendored_files();
+ let use_vendored = check_for_vendored_files();      // already there
  …
- if !force_clean && use_vendored.is_some() {
+ if use_vendored.is_some() {
     use_vendored_files(&use_vendored.unwrap(), &deployment_dir)?;
 } else {
     // fallback: download & build
 }

No other logic has to change; your existing helper already copies:

vendor/
 ├─ grobid/…   -> deployment/
 └─ runtime/…  -> deployment/runtime/


⸻

3 · Automate the vendor bundle in CI

# .github/workflows/vendor.yml
jobs:
  build-vendor:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest   triple: linux-x86_64
          - os: macos-14        triple: macos-aarch64
          - os: windows-latest  triple: windows-x86_64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      # 1. Build Grobid JAR + models once (Linux only is fine, they're platform-agnostic)
      - name: Build Grobid
        if: matrix.os == 'ubuntu-latest'
        run: ./gradlew :grobid-core:shadowJar assemble

      # 2. Build jlink runtime for this OS/arch
      - name: Jlink
        run: |
          $JAVA_HOME/bin/jlink \
            --module-path $JAVA_HOME/jmods \
            --add-modules java.base,java.logging,java.xml \
            --strip-debug --no-man-pages --no-header-files \
            --compress=2 \
            --output vendor/runtime/${{ matrix.triple }}

      # 3. Pack everything
      - name: Pack
        run: |
          tar -I 'zstd -19' -cf grobid-vendor-${{ matrix.triple }}.

          …pack & publish, done → end-users just unzip and run

                # 3. Pack everything (Rust exe + vendored tree) -------------------------
                - name: Build release binary
                  run: cargo build --release --locked

                - name: Assemble vendored tree
                  run: |
                    mkdir -p dist/${{ matrix.triple }}
                    cp target/release/grobid-cli  dist/${{ matrix.triple }}/   # .exe on Windows
                    cp -r vendor               dist/${{ matrix.triple }}/
                    tar -I 'zstd -19' -cf grobid-rs-${{ matrix.triple }}.tar.zst -C dist/${{ matrix.triple }} .
                - uses: actions/upload-artifact@v4
                  with: {name: grobid-rs-${{ matrix.triple }}, path: grobid-rs-${{ matrix.triple }}.tar.zst}

          (Run the Gradle build once on Linux, stash vendor/grobid/; the matrix jobs then reuse it or you build it once outside the matrix and upload as a dependency - whichever is simpler.)

          ⸻

          4 · Release instructions (for you & for users)

          Who	Command	Result
          CI	gh release create v0.1.0 ./grobid-rs-*tar.zst --notes "…"	3 assets, one per platform
          Linux user	curl -LO https://github.com/you/grobid-rs/releases/latest/download/grobid-rs-linux-x86_64.tar.zst | tar --zstd -xf - && ./grobid-cli -h	instant local run, no Docker, no Java
          macOS Homebrew	https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap (formula just downloads your tarball)	brew install you/tap/grobid-rs
          Windows Scoop	manifest points to .tar.zst → scoop install grobid-rs	same

          The Rust binary reads:

          let base = exe_dir().join("vendor");
          grobid_rs::init_with_config(&GrobidConfig::new(base))?;

          So it automatically finds the vendored runtime on every machine.

          ⸻

          5 · Small caveats
	1.	Size: even trimmed you ship ~140 MB compressed. Acceptable for CLI/tools; still ⅓ of Docker.
	2.	Updates: bump GROBID_VERSION + SHA → CI rebuild → new release. Provide grobid-cli self-upgrade (e.g. self_update crate) if you want polish.
	3.	Licences: copy $JAVA_HOME/legal/* + Grobid’s BSD-2 into THIRD_PARTY_NOTICES.txt.
	4.	AV false-positives (Windows): sign the exe or tell users to –unblock.

          That’s it.  From a user’s view:

          # download 120-150 MB once
          ./grobid-cli process --type header document.pdf

          No Docker, no JDK, no network — just works.
