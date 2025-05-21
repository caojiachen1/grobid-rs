# Core Rust-Grobid JNI Integration

## 1. Introduction

This document explains how to integrate Grobid, a Java-based document processing library, with Rust applications using the Java Native Interface (JNI).

### What is Grobid?

Grobid (GeneRation Of BIbliographic Data) is a machine learning library for extracting, parsing, and structuring raw documents, particularly scholarly papers in PDF format. It extracts structured XML/TEI encoded documents from PDF, including header information, full text, citation data, and more.

### Why Integrate Grobid with Rust?

Integrating Grobid with Rust offers several benefits:

- **Performance and Memory Safety**: Rust provides excellent performance, memory safety, and concurrency capabilities.
- **Ecosystem Integration**: Enables Grobid's document processing capabilities to be used within the Rust ecosystem.
- **Cross-Platform Applications**: Allows building robust cross-platform applications that leverage Grobid's document processing.
- **Custom Workflows**: Facilitates building custom document processing workflows combining Rust's strengths with Grobid's ML-based extraction.

### Overview of the JNI Approach

Java Native Interface (JNI) allows Java code to interact with native applications and libraries written in languages like C, C++, and Rust. In our case, we use JNI "inversely" - our Rust application loads and interacts with Java code. We use the `jni` crate to:

1. Initialize a JVM from Rust
2. Load Grobid's Java classes
3. Call Grobid's methods from Rust
4. Convert data between Java and Rust types

## 2. Understanding Grobid's Architecture for Embedding

Before integration, it's important to understand the key components of Grobid that we'll interact with:

- **GrobidFactory**: A singleton factory class that creates the Grobid Engine.
- **Engine**: The central class for processing documents, providing methods like `processHeader()`, `processFullText()`, etc.
- **GrobidModels**: Machine learning models used for various extraction tasks.
- **Configuration**: Settings that control Grobid's behavior, primarily set through the `grobid-home` directory.

The typical flow for embedding Grobid is:
1. Initialize the environment with paths to `grobid-home` and required libraries
2. Get a GrobidFactory instance
3. Create an Engine using the factory
4. Use the Engine to process documents

## 3. Java Native Interface (JNI) and the `jni` Crate

### Brief Explanation of JNI

JNI is a programming framework that enables Java code running in a JVM to call and be called by native applications and libraries. It serves as the bridge between Java and native code.

### Introduction to the `jni` Crate

The `jni` crate provides Rust bindings to JNI, allowing Rust applications to initialize a JVM and interact with Java objects. It handles the complexities of JNI, including memory management, threading, and type conversions.

### Key `jni` Crate Components:

- **JavaVM**: Represents the Java Virtual Machine. You typically create one per application.
- **JNIEnv**: The JNI environment providing methods to interact with Java. Each thread has its own `JNIEnv`.
- **JObject, JClass, JString**: Represent Java objects, classes, and strings respectively.
- **JValue**: Used to pass arguments to Java methods.
- **AttachGuard**: RAII guard that ensures the thread is detached from the JVM when it goes out of scope.
- **GlobalRef**: A global reference to a Java object that can be used across threads and JNI calls.

## 4. Setting Up the Rust Project for Grobid JNI Integration

### Prerequisites

Before starting the integration, ensure you have:
- Java Development Kit (JDK) 11 or higher installed
- JAVA_HOME environment variable properly set
- Grobid's resources (`grobid-core.jar` and `grobid-home`)
- Rust toolchain (stable 1.65.0+)

### `Cargo.toml`

Your `Cargo.toml` should include the following dependencies:

```toml
[dependencies]
jni = { version = "0.21", features = ["invocation"] }
once_cell = "1.19"
thiserror = "1.0"
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"], optional = true }

[features]
default = []
cli = []

[build-dependencies]
java-locator = "0.1"
# Additional build dependencies for downloading/managing Grobid resources
```

### `build.rs` (Build Script)

A build script is essential for JNI setup, helping locate the JVM, setting up library paths, and potentially managing Grobid resources. See the separate resources documentation for details.

## 5. Initializing the JVM and Grobid

Initializing Grobid in Rust requires two key steps: launching the JVM and initializing Grobid's Engine.

### Launching the JVM from Rust

Here's a simplified example of JVM initialization:

```rust
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

static JVM: OnceCell<JavaVM> = OnceCell::new();

pub fn init(base: &Path) -> Result<(), GrobidError> {
    if JVM.get().is_some() { return Ok(()); }
    
    // Paths for JVM initialization
    let runtime_dir = PathBuf::from(env!("JLINK_RUNTIME_PATH"));
    let jvm_lib = match std::env::consts::OS {
        "windows" => runtime_dir.join("bin/server/jvm.dll"),
        "macos"   => runtime_dir.join("lib/server/libjvm.dylib"),
        _         => runtime_dir.join("lib/server/libjvm.so"),
    };
    
    let classpath = PathBuf::from(env!("GROBID_JAR_PATH"));
    let grobid_home_path = PathBuf::from(env!("GROBID_HOME_PATH"));
    let lib_path = grobid_home_path.join("lib");
    
    // JVM args
    let class_path_arg = format!("-Djava.class.path={}", classpath.display());
    let grobid_home_arg = format!("-Dorg.grobid.home={}", grobid_home_path.display());
    let library_path_arg = format!("-Djava.library.path={}", lib_path.display());
    
    let args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option(&class_path_arg)
        .option(&grobid_home_arg)
        .option(&library_path_arg)
        .option("-Xmx1G")
        .build()?;
    
    // Start JVM
    let jvm_lib_path = jvm_lib.clone();
    let jvm = JavaVM::with_libjvm(args, move || Ok(jvm_lib_path))?;
    
    // Initialize Grobid (see next section)
    initialize_grobid_engine(&jvm)?;
    
    JVM.set(jvm).map_err(|_| GrobidError::JvmInitialization("JVM already initialized".to_string()))?;
    
    Ok(())
}
```

### JVM Arguments (`InitArgsBuilder`)

The critical JVM arguments for Grobid include:
- `-Djava.class.path`: Path to Grobid's JAR (and dependencies if not using a onejar)
- `-Dorg.grobid.home`: Path to the `grobid-home` directory
- `-Djava.library.path`: Path to native libraries (e.g., Wapiti) within `grobid-home/lib`
- `-Xmx1G`: Heap size (adjust based on your needs)

### Attaching the Current Thread

Each thread that interacts with the JVM must be attached to it and obtain its own `JNIEnv`:

```rust
fn with_env<F, R>(f: F) -> Result<R, GrobidError>
where
    F: FnOnce(&mut JNIEnv<'_>, JObject<'_>) -> Result<R, GrobidError>,
{
    let jvm = JVM.get().ok_or(GrobidError::NotInitialised)?;
    let mut guard = jvm.attach_current_thread()?;
    
    // Use the JNIEnv (via guard) to interact with Java...
    // ...
}
```

### Initializing Grobid (Factory and Engine)

After starting the JVM, we need to initialize Grobid:

```rust
use jni::objects::GlobalRef;
use std::sync::Mutex;

static ENGINE: OnceCell<Mutex<GlobalRef>> = OnceCell::new();

fn initialize_grobid_engine(jvm: &JavaVM) -> Result<(), GrobidError> {
    let mut env = jvm.attach_current_thread()?;
    
    // Get GrobidFactory instance
    let factory_cls = env.find_class("org/grobid/core/factory/GrobidFactory")?;
    let factory = env.call_static_method(
        factory_cls, 
        "getInstance", 
        "()Lorg/grobid/core/factory/GrobidFactory;", 
        &[]
    )?.l()?;
    
    // Create Grobid Engine
    let engine_obj = env.call_method(
        factory, 
        "createEngine", 
        "()Lorg/grobid/core/engines/Engine;", 
        &[]
    )?.l()?;
    
    // Store engine as a GlobalRef for future use
    let engine_global_ref = env.new_global_ref(engine_obj)?;
    ENGINE.set(Mutex::new(engine_global_ref))
        .map_err(|_| GrobidError::JvmInitialization("ENGINE already initialized".to_string()))?;
    
    Ok(())
}
```

## 6. Interacting with Grobid's Java API from Rust

Once Grobid is initialized, we can call its methods to process documents.

### Finding Java Classes (`env.find_class()`)

```rust
let file_cls = env.find_class("java/io/File")?;
```

### Creating Java Objects

```rust
// Create a Java File object
let j_path_str = env.new_string(pdf_path.to_string_lossy())?;
let j_file_obj = env.new_object(
    file_cls, 
    "(Ljava/lang/String;)V", 
    &[JValue::from(&j_path_str)]
)?;
```

### Calling Java Methods

```rust
// Call a method on the Engine object
let result = env.call_method(
    engine, 
    "processHeader", 
    "(Ljava/io/File;)Ljava/lang/String;", 
    &[JValue::from(&j_file_obj)]
)?.l()?;
```

### Understanding JNI Method Signatures

JNI method signatures follow the pattern: `(ParameterTypes)ReturnType`
- Simple types: `Z` (boolean), `B` (byte), `C` (char), `S` (short), `I` (int), `J` (long), `F` (float), `D` (double)
- Object types: `Lfully/qualified/ClassName;`
- Arrays: `[Type` (e.g., `[I` for int array, `[Ljava/lang/String;` for String array)
- Void: `V`

### Working with Java Strings

```rust
// Convert Java String to Rust String
let j_string = JString::from(j_string_obj);
let rust_string: String = env.get_string(&j_string)?.into();
```

### Example: Calling an Engine Method (Conceptual)

Here's a complete example of calling a Grobid method:

```rust
fn process_header(pdf_path: &Path) -> Result<String, GrobidError> {
    with_env(|env, engine| {
        // Create File object for the PDF
        let file_cls = env.find_class("java/io/File")?;
        let j_path_str = env.new_string(pdf_path.to_string_lossy())?;
        let j_file_obj = env.new_object(
            file_cls, 
            "(Ljava/lang/String;)V", 
            &[JValue::from(&j_path_str)]
        )?;
        
        // Call processHeader method
        let j_result_string_obj = env.call_method(
            engine,
            "processHeader",
            "(Ljava/io/File;)Ljava/lang/String;",
            &[JValue::from(&j_file_obj)],
        )?.l()?;
        
        // Convert result to Rust String
        let result_string = env.get_string(&JString::from(j_result_string_obj))?.into();
        Ok(result_string)
    })
}
```

## 7. Error Handling

Robust error handling is essential when working with JNI. A comprehensive error type should include:

```rust
#[derive(thiserror::Error, Debug)]
pub enum GrobidError {
    #[error("Grobid not initialised")] NotInitialised,
    #[error("JNI error: {0}")] Jni(#[from] jni::errors::Error),
    #[error("JVM initialization error: {0}")] JvmInitialization(String),
    #[error("Java exception: {0}")] Java(String),
    #[error("pdfalto failed: {0}")] PdfAlto(String),
}
```

### Common Errors and Troubleshooting:

- **ClassNotFoundException**: Check classpath and class names (use `/` not `.`)
- **NoSuchMethodError**: Verify method signatures with `javap -s ClassName`
- **UnsatisfiedLinkError**: Ensure native libraries are in the correct path and architecture
- **Crashes**: Often due to JNI misuse; enable `-Xcheck:jni` for debugging
- **Memory Leaks**: Check proper management of global references

## 8. Core JNI Advanced Topics

### Memory Management (JNI References)

- **Local References**: Valid only within a single native method call
  - Automatically freed when the native method returns
  - Limited in number per `JNIEnv`
- **Global References**: Valid across JNI calls and threads
  - Created with `env.new_global_ref()`
  - Must be explicitly freed with `env.delete_global_ref()`
- **Weak Global References**: Like global references but don't prevent garbage collection

### Threading

- `JavaVM` is thread-safe and can be shared between threads
- `JNIEnv` is thread-specific; each thread needs its own
- Always attach a thread before using JNI in it, and detach when done
- Use `AttachGuard` to automatically handle thread attachment/detachment

## 9. Core Integration Best Practices

- **Separation of Concerns**: Keep JNI code isolated in dedicated modules
- **Error Handling**: Properly check for and handle exceptions after JNI calls
- **Resource Management**: Carefully manage references, especially global ones
- **Initialization**: Initialize JVM and Grobid only once per application
- **Concurrency**: Understand thread safety of both JNI and Grobid
- **Signatures**: Double-check method signatures with `javap -s`
- **Testing**: Thoroughly test all paths, especially error conditions

For more advanced topics including debugging, resource management, packaging, and licensing, refer to the dedicated documents in this series.