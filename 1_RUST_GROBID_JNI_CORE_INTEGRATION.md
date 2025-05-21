# Core Rust-Grobid JNI Integration Guide

## 1. Introduction

### What is Grobid?
Grobid is a powerful open-source Java library designed for parsing, structuring, and extracting information from scholarly documents, particularly PDFs. It employs a cascade of machine learning models (primarily Conditional Random Fields - CRFs, with Wapiti as the default engine) to perform tasks such as header extraction (title, authors, affiliations, abstract), full-text parsing, citation extraction, and reference parsing.

### Why Integrate Grobid with Rust?
While Grobid offers an HTTP API, directly embedding its Java core into a Rust application via the Java Native Interface (JNI) provides several advantages:
*   **Performance:** Eliminates network overhead, leading to faster processing, especially for batch operations.
*   **Control:** Offers finer-grained control over Grobid's initialization, configuration, and lifecycle.
*   **Deployment:** Allows for self-contained Rust applications or libraries that bundle Grobid, simplifying deployment for end-users.
*   **Resource Management:** Enables more direct management of JVM resources from the Rust side.

This guide details how to achieve such an integration, leveraging the `jni` Rust crate.

### Overview of the JNI Approach
The Java Native Interface (JNI) is a programming framework that enables Java code running in a Java Virtual Machine (JVM) to call and be called by native applications (programs specific to a hardware and operating system platform) and libraries written in other languages such as C, C++, or Rust.

We will use the `jni` crate in Rust, which provides safe bindings to the JNI, to:
1.  Launch and configure an embedded JVM from Rust.
2.  Load Grobid's Java classes.
3.  Invoke Grobid's Java API methods directly.
4.  Convert data types between Java and Rust.
5.  Handle errors and Java exceptions.

## 2. Understanding Grobid's Architecture for Embedding

To effectively embed Grobid, it's crucial to understand its key components:

*   **`grobid-core.jar`:** This JAR file contains Grobid's main Java code and its dependencies. For convenience, Grobid often releases a "onejar" or "fat JAR" that bundles `grobid-core` and all its third-party libraries.
*   **`grobid-home`:** This is a critical external resource directory. It contains:
    *   Machine learning models (CRFs, etc.).
    *   Configuration files (e.g., `grobid.yaml`, `grobid.properties`).
    *   Dictionaries and other lexical resources.
    *   Native libraries required by Grobid, notably for the Wapiti CRF engine (e.g., `libwapiti.so` on Linux, `libwapiti.dylib` on macOS, `wapiti.dll` on Windows). These are platform-specific and located in subdirectories like `lib/lin-64/`, `lib/mac-arm-64/`, etc.
    The `grobid-home` directory must match the version of the `grobid-core.jar` being used.
*   **`pdfalto`:** Grobid relies on `pdfalto`, an external command-line tool, for the initial conversion of PDF documents into structured XML (ALTO format), which includes layout information. Grobid invokes `pdfalto` as a separate process.
*   **Key Java Classes:**
    *   `org.grobid.core.factory.GrobidFactory`: Used to obtain an instance of the Grobid engine.
    *   `org.grobid.core.engines.Engine`: The primary entry point for invoking Grobid's processing functionalities (e.g., header processing, full-text extraction). It implements `java.io.Closeable`, and instances should be closed when no longer needed to free resources.
    *   `org.grobid.core.engines.config.GrobidAnalysisConfig`: Allows fine-grained configuration for various parsing operations.
    *   `org.grobid.core.utils.GrobidProperties`: Manages Grobid's configuration, including the path to `grobid-home`.

## 3. Java Native Interface (JNI) and the `jni` Crate

### Brief Explanation of JNI
JNI defines a standard way for Java code to interoperate with native code. It specifies how native functions must be declared and implemented to be callable from Java, and provides a set of functions (the JNI API) that native code can use to interact with the JVM, such as creating Java objects, calling Java methods, and handling exceptions.

### Introduction to the `jni` Crate
The `jni` crate (version 0.21.1 as of recent examples) provides safe Rust bindings to the JNI. It aims to make JNI programming in Rust less error-prone by using Rust's ownership and lifetime system.

### Key `jni` Crate Components:
*   **`JavaVM`**: Represents the Java Virtual Machine. It's used to launch a JVM and attach/detach native threads.
*   **`JNIEnv`**: The primary interface to the JVM for a native thread. All JNI functions are called on a `JNIEnv` instance. It carries lifetime information to ensure safety.
*   **`JObject`, `JClass`, `JString`, etc.**: These are wrapper types for Java object references (local or global). They also carry lifetime information and provide methods for interacting with the underlying Java objects. For example, `JString` represents a Java string.
*   **Lifetimes**: The `jni` crate extensively uses lifetimes (e.g., `'local` or `'_`) to ensure that Java objects are not accessed after their references become invalid (e.g., after a JNI local frame is popped or a thread detaches).
*   **`sys` module**: Contains raw JNI types (e.g., `jstring`, `jobject`). You often convert safe wrappers to these raw types when returning from a native method to Java (e.g., `JString::into_raw()`).
*   **`AttachGuard`**: A RAII (Resource Acquisition Is Initialization) helper that automatically detaches the current thread from the JVM when the guard goes out of scope.

## 4. Setting Up the Rust Project for Grobid JNI Integration

### Prerequisites
*   **Java Development Kit (JDK):** A compatible JDK (Grobid 0.8.x requires Java 11 or higher) must be installed on the system where the Rust code will be compiled and run. The `JAVA_HOME` environment variable should ideally be set.
*   **Grobid Resources:**
    *   `grobid-core-X.Y.Z.jar` (or the "onejar" variant).
    *   A `grobid-home` directory matching the JAR version.
    *   `pdfalto` executables for the target platform(s).

### `Cargo.toml`
Add the `jni` crate to your project's `Cargo.toml` dependencies:
```toml
[dependencies]
jni = "0.21.1" # Or the latest compatible version
# Other dependencies like once_cell, thiserror might be useful
once_cell = "1.18"
thiserror = "1.0"
```

If your crate is a library that will be called by Java (not our primary use case here, but good to know for general JNI), you'd also configure it as a dynamic system library:
```toml
[lib]
crate-type = ["cdylib"]
```

### `build.rs` (Build Script)
While some JNI setups use a `build.rs` script to link against the JVM's shared library at compile time (e.g., `libjvm.so`, `libjvm.dylib`, `jvm.dll`), the `jni` crate also supports dynamically loading the JVM library at runtime. The `JavaVM::with_libjvm` function facilitates this by allowing you to specify the path to the JVM library. This approach can be more flexible as it doesn't hardcode paths at compile time.

If using `JavaVM::with_libjvm`, ensure you can locate the JVM shared library path correctly at runtime. This path is typically within the JDK installation directory (e.g., `$JAVA_HOME/lib/server/libjvm.so` on Linux, `$JAVA_HOME/lib/server/libjvm.dylib` on macOS, `$JAVA_HOME/bin/server/jvm.dll` on Windows).

## 5. Initializing the JVM and Grobid

A single JVM instance should be created per process and should typically live for the duration of the process.

### Launching the JVM from Rust
The `jni` crate provides `JavaVM::with_libjvm` (or `JavaVM::new` if the JVM library is already in the system's library search path). `JavaVM::with_libjvm` is generally preferred for explicit control.

```rust
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use std::path::PathBuf; // Or your preferred path type

// Example: Dynamically locating and loading libjvm
fn find_jvm_lib_path() -> PathBuf {
    // Implementation to find libjvm.so/dylib/dll based on JAVA_HOME or other means
    // This is platform-specific.
    let java_home = std::env::var("JAVA_HOME")
        .expect("JAVA_HOME environment variable not set.");
    let jvm_lib_path = if cfg!(target_os = "windows") {
        PathBuf::from(java_home).join("bin\server\jvm.dll")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(java_home).join("lib/server/libjvm.dylib")
    } else {
        PathBuf::from(java_home).join("lib/server/libjvm.so")
    };
    if !jvm_lib_path.exists() {
        panic!("JVM library not found at: {}", jvm_lib_path.display());
    }
    jvm_lib_path
}

// Store the JVM in a static variable, e.g., using once_cell
static JVM: once_cell::sync::OnceCell<JavaVM> = once_cell::sync::OnceCell::new();

pub fn init_jvm_and_grobid(
    grobid_core_jar_path: &str,
    grobid_home_path: &str,
    native_lib_path: &str, // Path to Grobid's native libs (e.g., Wapiti)
) -> Result<(), String> {
    if JVM.get().is_some() {
        return Ok(()); // Already initialized
    }

    let class_path_arg = format!("-Djava.class.path={}", grobid_core_jar_path);
    let grobid_home_arg = format!("-Dorg.grobid.home={}", grobid_home_path);
    let library_path_arg = format!("-Djava.library.path={}", native_lib_path);

    let jvm_args = InitArgsBuilder::new()
        .version(JNIVersion::V8) // Or a higher compatible version
        .option(&class_path_arg)
        .option(&grobid_home_arg)
        .option(&library_path_arg)
        .option("-Xmx2G") // Example: Set max heap size to 2GB
        // .option("-Xcheck:jni") // Useful for debugging JNI issues
        .build()
        .map_err(|e| format!("Failed to build JVM args: {}", e))?;

    let jvm_lib_path_buf = find_jvm_lib_path(); // Or pass it as an argument

    let jvm = JavaVM::with_libjvm(jvm_args, move || Ok(jvm_lib_path_buf.clone()))
        .map_err(|e| format!("Failed to create JVM: {}", e))?;

    if JVM.set(jvm).is_err() {
        return Err("Failed to store JVM instance (already initialized?)".to_string());
    }
    Ok(())
}
```

### JVM Arguments (`InitArgsBuilder`)
*   **Classpath (`-Djava.class.path`):** Must point to `grobid-core.jar` (or the onejar) and any other required dependencies if not using the onejar. Separate multiple paths with `:` (Linux/macOS) or `;` (Windows).
*   **Grobid Home (`-Dorg.grobid.home`):** Specifies the path to the `grobid-home` directory. Grobid uses this to find models, configurations, etc.
*   **Native Library Path (`-Djava.library.path`):** Crucial for Grobid to find its native dependencies, especially the Wapiti JNI library. This path should point to the directory within `grobid-home` containing the platform-specific shared libraries (e.g., `grobid-home/lib/linux-x86-64`).
*   **Other Options:**
    *   `-Xmx<size>`: Sets the maximum Java heap size (e.g., `-Xmx1G`, `-Xmx4G`). Grobid can be memory-intensive.
    *   `-Xcheck:jni`: Enables additional JNI checks, helpful for debugging but incurs a performance penalty.
    *   `-Djava.awt.headless=true`: Recommended if Grobid uses PDF libraries that might interact with AWT.

### Attaching the Current Thread
Any Rust thread that needs to interact with the JVM must first be attached. `JavaVM::attach_current_thread()` (or `attach_current_thread_as_daemon()`) performs this attachment and returns an `AttachGuard` which, when dropped, automatically detaches the thread. The `AttachGuard` dereferences to a `JNIEnv`.

```rust
// Assuming JVM is initialized and stored in a static variable:
// let jvm = JVM.get().ok_or("JVM not initialized")?;
// let mut env = jvm.attach_current_thread()?; // env is actually an AttachGuard
// // Use env (JNIEnv) to make JNI calls
```
The `mut` keyword for `env` is often necessary because many `JNIEnv` methods modify internal JVM state or create local references, conceptually taking `&mut self` even if the `jni` crate declares them as `&self` for ergonomic reasons related to how `AttachGuard` works. Recent versions of `jni` might require explicit mutable borrows of the guard or environment.

### Initializing Grobid (Factory and Engine)
Once the JVM is running and a thread is attached, you can initialize Grobid's Java components:

```rust
use jni::objects::{GlobalRef, JObject};
use jni::JNIEnv;
use std::sync::Mutex; // For thread-safe access to GlobalRef

// Store the Grobid Engine as a GlobalRef in a static variable
static ENGINE: once_cell::sync::OnceCell<Mutex<GlobalRef>> = once_cell::sync::OnceCell::new();

fn initialize_grobid_engine(env: &mut JNIEnv<'_>) -> Result<(), String> {
    if ENGINE.get().is_some() {
        return Ok(()); // Already initialized
    }

    let factory_cls = env.find_class("org/grobid/core/factory/GrobidFactory")
        .map_err(|e| format!("Failed to find GrobidFactory class: {}", e))?;

    let factory_obj = env.call_static_method(&factory_cls, "getInstance", "()Lorg/grobid/core/factory/GrobidFactory;", &[])
        .map_err(|e| format!("Failed to get GrobidFactory instance: {}", e))?
        .l() // Convert JValue to JObject
        .map_err(|e| format!("Factory instance was not an object: {}", e))?;

    let engine_obj = env.call_method(&factory_obj, "createEngine", "()Lorg/grobid/core/engines/Engine;", &[])
        .map_err(|e| format!("Failed to create Grobid Engine: {}", e))?
        .l()
        .map_err(|e| format!("Engine instance was not an object: {}", e))?;

    let engine_global_ref = env.new_global_ref(engine_obj)
        .map_err(|e| format!("Failed to create global ref for engine: {}", e))?;

    if ENGINE.set(Mutex::new(engine_global_ref)).is_err() {
        return Err("Failed to store Grobid engine (already stored?)".to_string());
    }
    Ok(())
}
```
The `Grobid Engine` instance (`engine_obj`) is typically long-lived and reused for multiple parsing tasks to avoid the overhead of re-initializing models. Storing it as a `GlobalRef` is essential if it's to be accessed from different `JNIEnv` contexts.

## 6. Interacting with Grobid's Java API from Rust

With `JNIEnv` and a `JObject` reference to the Grobid `Engine`, you can call its methods.

### Finding Java Classes (`env.find_class()`)
Use `env.find_class("fully/qualified/ClassName")` (using `/` instead of `.`).
Example: `env.find_class("java/io/File")?`

### Creating Java Objects
*   **Strings:** `env.new_string("your rust string")?` creates a Java `String` (`JString`).
*   **Other Objects:** `env.new_object(class_ref, constructor_signature, &[arg1, arg2])?` calls a constructor.
    Example: Creating a `java.io.File` object:
    ```rust
    let file_cls = env.find_class("java/io/File")?;
    let path_jstring = env.new_string("/path/to/your/document.pdf")?;
    let file_obj = env.new_object(&file_cls, "(Ljava/lang/String;)V", &[(&path_jstring).into()])?;
    ```

### Calling Java Methods
*   **Static Methods:** `env.call_static_method(class_ref, "methodName", "signature", &[args])?`
*   **Instance Methods:** `env.call_method(object_ref, "methodName", "signature", &[args])?`

Args are passed as a slice of `JValue`. The `.l()?` method is often used on the `Result<JValue, _>` to extract `JObject`.

### Understanding JNI Method Signatures
JNI method signatures are strings that uniquely identify a method. Format: `(ParameterTypes)ReturnType`.
Types: `V` (void), `Z` (boolean), `B` (byte), `C` (char), `S` (short), `I` (int), `J` (long), `F` (float), `D` (double), `Lfully/qualified/ClassName;` (class), `[Type` (array).
Example: `Engine.fullTextToTEI(File, GrobidAnalysisConfig)` is `(Ljava/io/File;Lorg/grobid/core/engines/config/GrobidAnalysisConfig;)Ljava/lang/String;`.

### Working with Java Strings
*   Rust to `JString`: `let j_string = env.new_string("Hello")?;`
*   `JString` to Rust `String`: `let rust_string: String = env.get_string(&j_string.into())?.into();`

### Example: Calling an Engine Method (Conceptual)
```rust
use jni::objects::{JObject, JString, JValue};
use jni::JNIEnv;
use std::path::Path;

// Conceptual helper, assumes JVM/ENGINE are set up.
// Actual implementation in src/lib.rs uses a more robust `with_env` helper.
fn call_engine_process_method_with_file_input_conceptual(
    env: &mut JNIEnv<'_>,      // Passed in from an attached thread context
    engine_obj: JObject<'_>, // Passed in (e.g., from GlobalRef)
    method_name: &str,
    pdf_path: &Path,
) -> Result<String, String> {
    let file_cls = env.find_class("java/io/File").map_err(|e| e.to_string())?;
    let path_jstr = env.new_string(pdf_path.to_string_lossy()).map_err(|e| e.to_string())?;
    let file_obj = env.new_object(&file_cls, "(Ljava/lang/String;)V", &[(&path_jstr).into()])
        .map_err(|e| format!("Failed to create File object: {}", e))?;

    let result_jvalue = env.call_method(
        engine_obj,
        method_name,
        "(Ljava/io/File;)Ljava/lang/String;", // Signature for methods like processHeader
        &[(&file_obj).into()],
    ).map_err(|e| format!("Java method call to '{}' failed: {}", method_name, e))?;

    let result_jobject = result_jvalue.l()
        .map_err(|e| format!("Method '{}' did not return an object: {}", method_name, e))?;
    let result_jstring = JString::from(result_jobject);

    let rust_string: String = env.get_string(&result_jstring)
        .map_err(|e| format!("Failed to get string from Java: {}", e))?
        .into();
    
    // Crucial: Exception checking should be done here or by the caller!
    // if env.exception_check().map_err(|e| e.to_string())? { ... handle ... }

    Ok(rust_string)
}
```

## 7. Error Handling

Robust error handling is vital in JNI programming.
*   **JNI Method Call Results:** Most `JNIEnv` methods return `jni::errors::Result<T>`. Always check these.
*   **Java Exceptions:** After any JNI call that might throw an exception, **must** check:
    *   `env.exception_check()?`: Returns `true` if an exception occurred.
    *   `env.exception_occurred()?`: Gets the throwable `JObject`.
    *   `env.exception_describe()?`: Prints stack trace (for debugging).
    *   `env.exception_clear()?`: Clears the exception. **Crucial** to do this.
*   **Custom Error Types:** Use a Rust error enum (e.g., with `thiserror`).
    ```rust
    #[derive(thiserror::Error, Debug)]
    pub enum GrobidRustError {
        #[error("Grobid not initialised")] NotInitialised,
        #[error("JNI error: {0}")] Jni(#[from] jni::errors::Error),
        #[error("JVM initialization error: {0}")] JvmInitialization(String),
        #[error("Java exception occurred: {0}")] JavaException(String),
    }
    ```

### Common Errors and Troubleshooting:
*   **`ClassNotFoundException`**: Incorrect `-Djava.class.path`, typo in class name.
*   **`NoSuchMethodError`**: Typo in method name, incorrect JNI signature. Use `javap -s`.
*   **`UnsatisfiedLinkError`**: Incorrect `-Djava.library.path`, missing native library (e.g., Wapiti).
*   **Crashes/Segfaults:** JNI misuse (e.g., wrong thread `JNIEnv`, invalid local ref). Use `-Xcheck:jni`.

## 8. Core JNI Advanced Topics

### Memory Management (JNI References)
*   **Local References (`JObject<'local>`)**:
    *   Valid only within a single native method call or `JNIEnv` lifetime.
    *   Automatically freed (mostly by `AttachGuard` or local frames).
    *   Limited number per `JNIEnv`. Use `env.with_local_frame` for many local refs.
*   **Global References (`GlobalRef`)**:
    *   Valid across calls/threads until explicitly freed (`env.delete_global_ref`).
    *   Create with `env.new_global_ref(local_jobject)?`.
    *   Essential for long-lived objects like the Grobid `Engine` instance.

### Threading
*   **`JavaVM` vs `JNIEnv`**: `JavaVM` is process-wide. `JNIEnv` is thread-local.
*   **Attaching Threads**: Each Rust thread needing JNI access must call `jvm.attach_current_thread()` to get its own `JNIEnv`.
*   **Grobid Engine Thread Safety**: The Grobid `Engine` is generally thread-safe for concurrent requests. The `src/lib.rs` approach of a single `Engine` in a `Mutex<GlobalRef>` accessed via `with_env` serializes calls at the Rust level, which is a safe starting point.

## 9. Core Integration Best Practices

*   **Isolate JNI Logic:** Encapsulate JNI calls in a dedicated Rust module/wrapper (e.g., `with_env` from `src/lib.rs`).
*   **Single JVM, Single Engine (GlobalRef):** Initialize `JavaVM` once. Initialize Grobid `Engine` once and store as `GlobalRef`.
*   **Error Handling:** Diligently check JNI results and Java exceptions.
*   **Lifetimes:** Manage `JObject` lifetimes; use `GlobalRef` for shared, long-lived Java objects.
*   **Resource Paths:** Correctly set `java.class.path`, `org.grobid.home`, `java.library.path`.
*   **Thread Safety:** Ensure `JNIEnv` is thread-local. Synchronize access to shared `GlobalRef`s if multiple Rust threads make JNI calls. 