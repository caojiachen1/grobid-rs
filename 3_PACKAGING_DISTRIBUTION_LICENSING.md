# Packaging, Distribution, and Licensing for Rust-Grobid JNI Applications

## 1. Introduction

Distributing a Rust application that embeds Grobid via JNI involves careful consideration of how to package the Java Runtime Environment (JRE), Grobid's own resources (`grobid-core.jar`, `grobid-home`), native libraries (Wapiti, `pdfalto`), and how to comply with the various licenses involved.

## 2. Packaging Strategies

### 2.1. Java Runtime Environment (JRE)

Your Rust application needs a JRE to run the embedded Grobid Java code. You have a few options:

1.  **System JRE/JDK (User-Provided):**
    *   **How:** The application relies on the end-user having a compatible Java Development Kit (JDK) or Java Runtime Environment (JRE) installed on their system, and potentially `JAVA_HOME` being set correctly.
    *   **Pros:** Simplest for the Rust crate developer; keeps the application distribution size small.
    *   **Cons:** Less user-friendly; can lead to version conflicts or missing JRE issues for the end-user. Your application needs to robustly find `libjvm` (e.g., using `JAVA_HOME` or known system paths).

2.  **Bundled Custom JRE (using `jlink`):**
    *   **How:** Use the `jlink` tool (available in JDK 9+) to create a minimal, custom JRE containing only the Java modules required by Grobid. This custom JRE is then bundled with your Rust application.
        *   You'll need to identify Grobid's module dependencies (e.g., using `jdeps` on `grobid-core.jar` and its own dependencies).
        *   Example `jlink` command (conceptual):
            ```bash
            jlink --module-path $JAVA_HOME/jmods:path/to/grobid/modules --add-modules <module1,module2,...> --output bundled-jre --strip-debug --no-header-files --no-man-pages --compress=2
            ```
    *   **Pros:** Creates a self-contained application, improving user experience and reducing external dependencies. Ensures a compatible Java version.
    *   **Cons:** Significantly increases the application distribution size. Requires more complex build/packaging steps. You need to ensure your Rust code can locate `libjvm` within this bundled JRE (e.g., by having a known relative path).
    *   The `grobid-rs` project aims for this, as indicated by the CI scripts trying to create a jlinked runtime.

### 2.2. Grobid Resources

*   **`grobid-core.jar` (and dependencies if not a onejar):**
    *   Typically small enough to be directly bundled with your Rust application (e.g., in an `assets` or `vendor` directory).
    *   The path to this JAR is then used in the `-Djava.class.path` JVM argument.

*   **`grobid-home` directory:**
    *   **Option A: User-Provided:** Simplest for distribution size. The application requires the user to download/configure `grobid-home` separately.
    *   **Option B: Bundled and Extracted:** Compress `grobid-home` (e.g., `.zip`, `.tar.gz`) and include it in your application's assets. Extract it to a suitable location (e.g., application data directory, temporary folder) on first run or during installation. This makes the application self-contained but large.
        *   The `grobid-rs` project structure (e.g., `base.join("grobid")` for `grobid-home`) suggests an expectation that these resources are co-located or managed by the application/build process.

### 2.3. Native Libraries

*   **Wapiti JNI Library:**
    *   These are the platform-specific shared libraries (e.g., `libwapiti.so`, `libwapiti.dylib`, `wapiti.dll`) found within `grobid-home/lib/<platform-arch>/`.
    *   If `grobid-home` is bundled, these are included. The `-Djava.library.path` JVM argument must point to their directory.
    *   If `grobid-home` is user-provided, the user is responsible for its integrity, including these libraries.

*   **`pdfalto` Executable:**
    *   Platform-specific executables found in `grobid-home/pdfalto/<platform-arch>/`.
    *   If `grobid-home` is bundled, these are included. Grobid usually finds them automatically if `org.grobid.home` is set correctly.
    *   Ensure they have execute permissions after extraction if bundling.

## 3. Distribution

*   **Cross-Platform Support:** If targeting multiple operating systems (Linux, macOS, Windows) and architectures (x86_64, aarch64):
    *   You'll need to package the correct bundled JRE (if used), Wapiti libraries, and `pdfalto` binaries for each target platform.
    *   Cargo features, conditional compilation (`#[cfg(target_os = ...)]`), and build scripts (`build.rs`) can help manage platform-specific components and paths.
    *   CI/CD pipelines are essential for building and testing on all supported platforms.
*   **Installation:**
    *   **Simple Archive:** Distribute as a `.zip` or `.tar.gz` containing the Rust executable and all bundled resources. Users extract and run.
    *   **Installers:** For a more polished experience, create native installers (e.g., MSI for Windows, PKG for macOS, DEB/RPM for Linux) that handle placing files, setting up paths, and potentially extracting bundled resources.
*   **Configuration:** Clearly document how users should configure paths (if `grobid-home` or JRE are not bundled) or how the application expects its bundled resources to be laid out.

## 4. Licensing Considerations

Complying with the licenses of all components is critical.

*   **Your Rust Code:** You choose the license (e.g., MIT, Apache 2.0). It must be compatible with any statically linked Rust dependencies.

*   **`jni` Crate:** Typically MIT and/or Apache 2.0 (check its `Cargo.toml`). Permissive.

*   **Grobid (`grobid-core`, Grobid-modified Wapiti Java wrapper):** Apache License 2.0.
    *   **Requirement:** Permissive. You can include and distribute Grobid JARs. You must include a copy of the Apache 2.0 license and any NOTICE files if present.

*   **Wapiti (CRF C library):** BSD-like (typically 2-clause or 3-clause).
    *   **Requirement:** Permissive. You can distribute the Wapiti native libraries. You must include a copy of its license.

*   **`pdfalto` (PDF to ALTO converter):** GNU General Public License v3.0 (GPL-3.0).
    *   **Requirement:** This is a strong copyleft license.
        *   If you distribute `pdfalto` binaries, you **must** comply with GPL-3.0. This typically means providing the complete corresponding source code for `pdfalto` (or a written offer to provide it) alongside the binaries.
        *   Grobid invokes `pdfalto` as a separate command-line executable. This is generally considered "mere aggregation" and does **not** typically require your Rust application or Grobid itself to be licensed under GPL-3.0, as long as they are distinct works and only communicate via standard inter-process mechanisms (like command-line calls, pipes).
        *   **Action:** If you bundle `pdfalto`, ensure you also provide access to its source code as required by the GPL-3.0. Include the GPL-3.0 license text.

*   **OpenJDK (if bundling a custom JRE built with `jlink`):** Typically GPLv2 with Classpath Exception.
    *   **Requirement:** The Classpath Exception allows you to link your application (including Grobid running on the JRE) with the standard Java libraries without your application becoming subject to GPLv2.
    *   If you distribute OpenJDK binaries (even a custom JRE), you must comply with GPLv2 for those binaries. This includes providing the GPLv2 license text, any relevant notices, and a copy of (or offer for) the OpenJDK source code corresponding to the binaries you distribute.

### Summary of Licensing Actions:

1.  **Identify all components:** List all third-party software you are distributing (Rust crates, Java JARs, native libraries, executables, JRE).
2.  **Identify their licenses:** Determine the specific license for each component.
3.  **Comply with each license:**
    *   For permissive licenses (MIT, Apache 2.0, BSD): Usually requires including the license text and any copyright/NOTICE files.
    *   For copyleft licenses (GPL): Stricter requirements, especially regarding source code availability for the GPL-licensed component itself.
4.  **Provide a consolidated licenses document:** It's good practice to include a file (e.g., `LICENSES.md` or a `NOTICE` file) in your distribution that lists all bundled software and their licenses, and points to where the full license texts can be found.

## 5. Alternatives to JNI (Brief Mention)

While JNI (via the `jni` crate) is the primary method for embedding Java into Rust:

*   **Project Panama (OpenJDK):** A newer initiative in OpenJDK aimed at simplifying native interoperation from Java. It could potentially offer less cumbersome alternatives to JNI in the future for Java calling native code, or native code interacting with Java memory layouts more directly. Its relevance for Rust-calls-Java might be indirect but could influence future JNI interop libraries.
*   **JNA (Java Native Access) / JNR-FFI (Java Native Runtime):** These are Java libraries that make it easier for Java code to call native C libraries without writing manual JNI boilerplate, often by using `libffi`. They are less relevant when the primary goal is for Rust to initiate and control the Java environment, but are part of the broader Java-native ecosystem.

For the use case of a Rust application embedding and calling a Java library like Grobid, the `jni` crate remains the most direct, mature, and well-supported approach.

## 6. Conclusion

Packaging and distributing a Rust application with embedded Grobid requires careful planning for the JRE, Grobid resources, native dependencies, and meticulous attention to licensing. A self-contained application using a bundled custom JRE and bundled (and extracted) `grobid-home` offers the best user experience but comes with a larger distribution size and more complex build process. Always prioritize clear documentation for your users regarding any prerequisites or setup steps if not fully bundling all components. Finally, ensure full compliance with all software licenses involved. 