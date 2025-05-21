# Advanced Topics and Debugging for grobid-rs

## 1. Debugging JNI Issues

### 1.1. Enabling JVM JNI Checks

The JVM provides built-in validation for JNI calls, which can identify common errors:

```rust
// Add this option when initializing the JVM
let args = InitArgsBuilder::new()
    .version(JNIVersion::V8)
    // Other options...
    .option("-Xcheck:jni")
    .build()?;
```

This flag has a performance impact and should only be used during development. It will:
- Validate argument types and count
- Check for invalid object references
- Verify thread attachment status
- Validate memory operations
- Print detailed error messages to stderr

### 1.2. Handling and Inspecting Java Exceptions

Java exceptions that occur during JNI calls must be explicitly checked and handled:

```rust
// After any JNI call that might throw an exception
if env.exception_check()? {
    // Get the exception object
    let exception = env.exception_occurred()?;
    
    // Print the stack trace to stderr (optional)
    env.exception_describe()?;
    
    // CRITICAL: Clear the exception
    env.exception_clear()?;
    
    // Get the exception message and convert to Rust
    let msg_obj = env.call_method(exception, "toString", "()Ljava/lang/String;", &[])?;
    let error_msg: String = env.get_string(&JString::from(msg_obj.l()?))?.into();
    
    return Err(GrobidError::Java(error_msg));
}
```

The `with_env` helper in `grobid-rs` centralizes this exception handling logic:

```rust
fn with_env<F, R>(f: F) -> Result<R, GrobidError>
where
    F: FnOnce(&mut JNIEnv<'_>, JObject<'_>) -> Result<R, GrobidError>,
{
    // Get JVM and attach current thread
    let jvm = JVM.get().ok_or(GrobidError::NotInitialised)?;
    let mut guard = jvm.attach_current_thread().map_err(GrobidError::Jni)?;
    let env_mut_ref = &mut guard;

    // Get engine reference
    let eng_ref = ENGINE.get().ok_or(GrobidError::NotInitialised)?;
    let locked_engine_gref = eng_ref.lock().unwrap();
    let engine_jobject = env_mut_ref.new_local_ref(unsafe { 
        JObject::from_raw((*locked_engine_gref).as_raw()) 
    })?;
    
    // Call user function
    let out = f(env_mut_ref, engine_jobject)?;
    
    // Check for exceptions
    if guard.exception_check().map_err(GrobidError::Jni)? {
        // Exception handling as shown above
        // ...
    }
    
    Ok(out)
}
```

### 1.3. Common JNI Errors and Their Causes

#### ClassNotFoundException
- **Symptom:** `env.find_class(...)` fails or Java throws this exception
- **Causes:**
  - Incorrect `-Djava.class.path` JVM argument
  - Typo in class name (use `/` instead of `.` for package separators)
  - JAR missing from classpath
- **Fix:** Verify classpath, check class name spelling, ensure JAR is available

#### NoSuchMethodError
- **Symptom:** `env.call_method(...)` or `env.call_static_method(...)` fails
- **Causes:**
  - Typo in method name
  - Incorrect JNI method signature
  - Method doesn't exist in the class or has different parameters
- **Fix:** Use `javap -s YourClass.class` to get exact JNI signatures

#### UnsatisfiedLinkError
- **Symptom:** JVM fails to load native libraries
- **Causes:**
  - Incorrect `-Djava.library.path`
  - Missing or incompatible native library files
  - Wrong architecture (e.g., trying to load x86 library on ARM)
- **Fix:** Verify library path points to correct directory with appropriate platform libraries

#### JVM Crashes / Segmentation Faults
- **Symptom:** Process terminates unexpectedly with minimal error information
- **Causes:**
  - Using `JNIEnv` from the wrong thread
  - Using invalid references (deleted or from different threads)
  - Passing null pointers where not expected
- **Fix:** Enable `-Xcheck:jni`, review thread management, audit reference handling

### 1.4. Logging and Diagnostics

- **JVM Diagnostics:** Add `-XX:+HeapDumpOnOutOfMemoryError` to create heap dumps if OOM occurs
- **Thread Dumps:** When hung, use `jcmd <pid> Thread.print` to diagnose deadlocks
- **Grobid Logging:** Configure SLF4J by providing binding JAR (like `logback-classic.jar`) and config file
- **Rust Logging:** Use the `log` crate with `env_logger` or other implementation
- **Exception Serialization:** When capturing Java exceptions, include the entire stack trace:

```rust
fn get_stack_trace(env: &mut JNIEnv, exception: JObject) -> Result<String, jni::errors::Error> {
    let sw_class = env.find_class("java/io/StringWriter")?;
    let sw = env.new_object(sw_class, "()V", &[])?;
    
    let pw_class = env.find_class("java/io/PrintWriter")?;
    let pw = env.new_object(pw_class, "(Ljava/io/Writer;)V", &[JValue::Object(&sw)])?;
    
    env.call_method(
        exception,
        "printStackTrace",
        "(Ljava/io/PrintWriter;)V",
        &[JValue::Object(&pw)]
    )?;
    
    let string_result = env.call_method(sw, "toString", "()Ljava/lang/String;", &[])?;
    let java_string = JString::from(string_result.l()?);
    let rust_string: String = env.get_string(&java_string)?.into();
    
    Ok(rust_string)
}
```

## 2. Advanced JNI Concepts

### 2.1. Memory Management: Local vs. Global References

#### Local References
- **Lifetime:** Valid only within a single native method call (scope of current `JNIEnv`)
- **Management:** Automatically freed when the `AttachGuard` is dropped
- **Limit:** Limited in number per thread (typically 16-32 by default)
- **Usage:** When references are only needed within a single function
- **Creation:** Most JNI functions like `find_class`, `new_object`, etc. return local references

For functions creating many local references in loops, use local frames:

```rust
env.with_local_frame(16, |env| {
    for item in items {
        // Each local reference created here is freed when the closure exits
        let obj = env.new_object(/* ... */)?;
        // ...
    }
    Ok(()) // Can return a local reference to promote it to the parent frame
})?;
```

#### Global References
- **Lifetime:** Valid across multiple JNI calls and threads until explicitly deleted
- **Management:** Must be manually created and deleted
- **Usage:** For long-lived objects like the Grobid Engine
- **Creation:** `env.new_global_ref(obj)`
- **Deletion:** `env.delete_global_ref(global_ref)`

Example from grobid-rs:
```rust
// Create and store a global reference to the Engine
let engine_obj = env.call_method(factory, "createEngine", "()Lorg/grobid/core/engines/Engine;", &[])?;
let engine_global_ref = env.new_global_ref(engine_obj)?;
ENGINE.set(Mutex::new(engine_global_ref))
    .map_err(|_| GrobidError::JvmInitialization("ENGINE already initialized".to_string()))?;

// Use the global reference later
let engine_ref = ENGINE.get().ok_or(GrobidError::NotInitialised)?;
let locked_engine_gref = engine_ref.lock().unwrap();
```

### 2.2. Threading Considerations

- **JavaVM:** Thread-safe, one per process, can be shared between Rust threads
- **JNIEnv:** Thread-specific, must not be shared between threads
- **Thread Attachment:** Every thread interacting with JNI needs its own `JNIEnv`

```rust
// For new threads that need JNI access
std::thread::spawn(move || {
    let jvm = JVM.get().expect("JVM not initialized");
    let guard = jvm.attach_current_thread().expect("Failed to attach thread");
    
    // Use guard (which derefs to JNIEnv) for JNI calls
    // ...
    
    // guard is automatically detached when dropped
});
```

For thread pooling with JNI:
```rust
let pool = ThreadPoolBuilder::new()
    .after_start(|_| {
        // Attach thread to JVM when it starts
        let jvm = JVM.get().unwrap();
        THREAD_ENV.with(|env| {
            *env.borrow_mut() = Some(jvm.attach_current_thread().unwrap());
        });
    })
    .before_stop(|_| {
        // Detachment is automatic with AttachGuard, but this
        // explicitly shows the thread is done with JNI
        THREAD_ENV.with(|env| {
            env.borrow_mut().take();
        });
    })
    .build()
    .unwrap();
```

### 2.3. Advanced Method Calls and JValue

#### Overloaded Methods

Java supports method overloading. To call the correct version, the signature must exactly match:

```rust
// Calling String.substring(int)
env.call_method(string_obj, "substring", "(I)Ljava/lang/String;", &[JValue::Int(5)])?;

// Calling String.substring(int, int)
env.call_method(string_obj, "substring", "(II)Ljava/lang/String;", 
    &[JValue::Int(5), JValue::Int(10)])?;
```

#### Working with Arrays

```rust
// Create a new Java array of Strings
let string_array = env.new_object_array(
    3, // length
    env.find_class("java/lang/String")?, // element class
    JObject::null(), // initial element (null)
)?;

// Set array elements
for (i, &s) in ["one", "two", "three"].iter().enumerate() {
    let j_string = env.new_string(s)?;
    env.set_object_array_element(string_array, i as jsize, j_string)?;
}

// Pass the array to a Java method
env.call_method(
    some_java_obj,
    "processStrings",
    "([Ljava/lang/String;)V",
    &[JValue::Object(&string_array)]
)?;
```

#### Primitive Arrays

For primitive arrays, use specialized methods:

```rust
// Create a new int array
let int_array = env.new_int_array(5)?;

// Set all elements
env.set_int_array_region(int_array, 0, &[1, 2, 3, 4, 5])?;

// Get elements
let mut buffer = [0; 5];
env.get_int_array_region(int_array, 0, &mut buffer)?;
```

## 3. Performance Optimization

### 3.1. Minimizing JNI Overhead

JNI calls have overhead. Minimize them with these techniques:

1. **Batch Operations:** Pass multiple items in a single JNI call instead of making separate calls
2. **Cache JNI References:** Avoid repeatedly looking up the same classes and methods
3. **Use Direct Buffers:** For large data transfers, use `java.nio.ByteBuffer` with direct memory access
4. **Reduce String Conversions:** String conversion between Java and Rust is expensive

### 3.2. Thread Pooling for Concurrent Processing

Process multiple documents concurrently with a thread pool:

```rust
pub fn process_batch(files: &[PathBuf], num_threads: usize) -> Result<Vec<String>, GrobidError> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        // Thread setup as shown in 2.2
        .build()
        .unwrap();
    
    let results = pool.install(|| {
        files.par_iter()
            .map(|file| process_header(file))
            .collect::<Result<Vec<_>, _>>()
    })?;
    
    Ok(results)
}
```

### 3.3. Memory Optimization

1. **Tune JVM Heap:** Set appropriate `-Xmx` based on document size and batch processing needs
2. **Clean Local References:** In loops processing many objects, use `with_local_frame`
3. **Limit Concurrent Processing:** Set thread count based on available memory
4. **GC Hints:** For long-running processes, consider occasional `System.gc()` calls:

```rust
fn suggest_gc(env: &mut JNIEnv) -> Result<(), jni::errors::Error> {
    let system_class = env.find_class("java/lang/System")?;
    env.call_static_method(system_class, "gc", "()V", &[])?;
    Ok(())
}
```

## 4. Robustness Strategies

### 4.1. Error Recovery

Implement graceful recovery from transient failures:

```rust
fn process_with_retry<F, T>(operation: F, max_attempts: usize) -> Result<T, GrobidError>
where
    F: Fn() -> Result<T, GrobidError>,
{
    let mut attempts = 0;
    let mut last_error = None;
    
    while attempts < max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if is_transient_error(&e) {
                    attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                    last_error = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    Err(last_error.unwrap_or(GrobidError::Other("Unknown error".to_string())))
}

fn is_transient_error(error: &GrobidError) -> bool {
    matches!(error, 
        GrobidError::Jni(jni::errors::Error::JniCall(_)) |
        GrobidError::Java(e) if e.contains("OutOfMemoryError") |
        GrobidError::PdfAlto(_)
    )
}
```

### 4.2. Cleanup on Termination

Ensure resources are properly released if the application terminates unexpectedly:

```rust
fn setup_signal_handlers() {
    ctrlc::set_handler(move || {
        println!("Received termination signal, cleaning up...");
        cleanup_resources();
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
}

fn cleanup_resources() {
    // Delete any temporary files
    if let Ok(temp_dir) = std::env::var("GROBID_TMP_DIR") {
        let _ = std::fs::remove_dir_all(temp_dir);
    }
    
    // Force JVM termination if needed
    // Note: This is generally not recommended as it doesn't allow
    // for clean shutdown, but can be a last resort
    if let Some(jvm) = JVM.get() {
        let _ = unsafe { jvm.destroy() };
    }
}
```

### 4.3. Monitoring and Health Checks

For long-running services, implement health monitoring:

```rust
pub fn check_grobid_health() -> bool {
    // Verify JVM is running
    if JVM.get().is_none() {
        return false;
    }
    
    // Try a simple processing task
    let test_data = "A simple string to process";
    match with_env(|env, engine| {
        // Convert test_data to Java String
        let j_test_data = env.new_string(test_data)?;
        
        // Try to call a simple method
        let result = env.call_method(
            engine,
            "processDate", 
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&j_test_data)]
        )?;
        
        // If we get here without exception, health check passed
        let _ = result.l()?;
        Ok(true)
    }) {
        Ok(_) => true,
        Err(e) => {
            log::error!("Health check failed: {}", e);
            false
        }
    }
}
```

## 5. Advanced Configuration

### 5.1. Customizing Grobid Behavior

Create a configuration builder to expose Grobid options:

```rust
pub struct GrobidConfig {
    consolidate_citations: i32,  // 0, 1, or 2
    include_raw_citations: bool,
    include_raw_affiliations: bool,
    include_tei_coordinates: bool,
    segment_sentences: bool,
    start_page: Option<i32>,
    end_page: Option<i32>,
    generate_ids: bool,
}

impl Default for GrobidConfig {
    fn default() -> Self {
        Self {
            consolidate_citations: 0,
            include_raw_citations: false,
            include_raw_affiliations: false,
            include_tei_coordinates: false,
            segment_sentences: false,
            start_page: None,
            end_page: None,
            generate_ids: false,
        }
    }
}

impl GrobidConfig {
    pub fn builder() -> GrobidConfigBuilder {
        GrobidConfigBuilder::default()
    }
    
    pub fn to_analysis_config(&self, env: &mut JNIEnv) -> Result<JObject, GrobidError> {
        // Create GrobidAnalysisConfig.Builder
        let builder_class = env.find_class("org/grobid/core/engines/config/GrobidAnalysisConfig$Builder")?;
        let builder = env.new_object(builder_class, "()V", &[])?;
        
        // Set options
        if self.consolidate_citations > 0 {
            env.call_method(
                builder, 
                "consolidateCitations", 
                "(I)Lorg/grobid/core/engines/config/GrobidAnalysisConfig$Builder;",
                &[JValue::Int(self.consolidate_citations)]
            )?;
        }
        
        // More options...
        
        // Build and return the config
        let config = env.call_method(
            builder,
            "build",
            "()Lorg/grobid/core/engines/config/GrobidAnalysisConfig;",
            &[]
        )?.l()?;
        
        Ok(config)
    }
}

// Usage:
let config = GrobidConfig::builder()
    .consolidate_citations(1)
    .include_tei_coordinates(true)
    .build();

let result = fulltext_to_tei_with_config(pdf_path, &config)?;
```

### 5.2. Hot-Patching Grobid

Support dynamic extension of the Grobid classpath:

```rust
fn init_with_extra_jars(base: &Path, extra_jars: &[PathBuf]) -> Result<(), GrobidError> {
    if JVM.get().is_some() { return Ok(()); }
    
    // Build classpath including main JAR and extras
    let main_jar = PathBuf::from(env!("GROBID_JAR_PATH"));
    let mut classpath = main_jar.to_string_lossy().to_string();
    
    let path_separator = if cfg!(windows) { ";" } else { ":" };
    
    for jar in extra_jars {
        if jar.exists() && jar.extension().map_or(false, |e| e == "jar") {
            classpath.push_str(path_separator);
            classpath.push_str(&jar.to_string_lossy());
        }
    }
    
    let class_path_arg = format!("-Djava.class.path={}", classpath);
    
    // Continue with JVM initialization as before
    // ...
}
```

## 6. Security Considerations

### 6.1. Sandboxing External Tools

Securely run pdfalto to protect against malicious PDFs:

```rust
fn run_pdfalto_sandboxed(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError> {
    let bin = determine_pdfalto_path(grobid_home)?;
    let out_xml = pdf.with_extension("alto.xml");
    
    #[cfg(target_os = "linux")]
    {
        // Use bubblewrap (bwrap) on Linux for sandboxing
        let temp_dir = tempfile::tempdir()?;
        
        let status = Command::new("bwrap")
            .args(&[
                "--ro-bind", "/lib", "/lib",
                "--ro-bind", "/lib64", "/lib64",
                "--ro-bind", "/usr", "/usr",
                "--ro-bind", &bin.to_string_lossy(), "/pdfalto",
                "--ro-bind", &pdf.to_string_lossy(), "/input.pdf",
                "--bind", &temp_dir.path().to_string_lossy(), "/tmp",
                "--bind", &out_xml.parent().unwrap().to_string_lossy(), "/output",
                "--chdir", "/",
                "--unshare-all",
                "--die-with-parent",
                "/pdfalto", "--inputFile", "/input.pdf", 
                "--outputFile", &format!("/output/{}", out_xml.file_name().unwrap().to_string_lossy())
            ])
            .status()?;
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux platforms, run directly but clean up carefully
        let status = Command::new(&bin)
            .arg("--inputFile").arg(pdf)
            .arg("--outputFile").arg(&out_xml)
            .status()?;
    }
    
    // Check status and return
    // ...
}
```

### 6.2. Input Validation

Validate PDFs before processing:

```rust
fn validate_pdf(path: &Path) -> Result<(), GrobidError> {
    // Check file size
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_PDF_SIZE {
        return Err(GrobidError::InvalidInput(format!(
            "PDF exceeds maximum size ({} > {} bytes)",
            metadata.len(), MAX_PDF_SIZE
        )));
    }
    
    // Check file header for PDF signature
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 4];
    if file.read(&mut buffer)? != 4 || &buffer != b"%PDF" {
        return Err(GrobidError::InvalidInput(
            "File does not appear to be a valid PDF".to_string()
        ));
    }
    
    Ok(())
}
```

### 6.3. Memory Limits and Watchdog

Prevent resource exhaustion:

```rust
fn process_with_watchdog<F, T>(operation: F, timeout_secs: u64) -> Result<T, GrobidError>
where
    F: FnOnce() -> Result<T, GrobidError> + Send + 'static,
    T: Send + 'static,
{
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    
    let handle = std::thread::spawn(move || {
        let operation_result = operation();
        *result_clone.lock().unwrap() = Some(operation_result);
    });
    
    // Wait for completion or timeout
    if handle.join().is_ok() {
        // Thread completed
        match Arc::try_unwrap(result).unwrap().into_inner().unwrap() {
            Some(result) => result,
            None => Err(GrobidError::Other("Operation completed without result".to_string())),
        }
    } else {
        // Thread panicked or timed out
        Err(GrobidError::Timeout("Operation took too long to complete".to_string()))
    }
}
```

## 7. Advanced Grobid API Usage

### 7.1. Citation Context Extraction

Extract contexts around citations:

```rust
pub fn extract_citation_contexts(pdf: &Path, config: &GrobidConfig) -> Result<Vec<CitationContext>, GrobidError> {
    with_env(|env, engine| {
        // Create File and GrobidAnalysisConfig
        let file_obj = create_file_object(env, pdf)?;
        let config_obj = config.to_analysis_config(env)?;
        
        // Process document to get BiblioDocument
        let doc = env.call_method(
            engine,
            "processFullText",
            "(Ljava/io/File;Lorg/grobid/core/engines/config/GrobidAnalysisConfig;)Lorg/grobid/core/document/Document;",
            &[JValue::Object(&file_obj), JValue::Object(&config_obj)]
        )?.l()?;
        
        // Extract citations with context
        let citations = env.call_method(
            engine,
            "processCitationPatentST36",
            "(Lorg/grobid/core/document/Document;)Ljava/util/List;",
            &[JValue::Object(&doc)]
        )?.l()?;
        
        // Convert Java citation list to Rust structures
        let size = env.call_method(citations, "size", "()I", &[])?.i()?;
        let mut results = Vec::with_capacity(size as usize);
        
        for i in 0..size {
            let citation = env.call_method(
                citations, 
                "get", 
                "(I)Ljava/lang/Object;", 
                &[JValue::Int(i)]
            )?.l()?;
            
            // Extract fields and add to results
            // ...
        }
        
        Ok(results)
    })
}
```

### 7.2. Affiliation and Address Parsing

Parse affiliations and addresses:

```rust
pub fn parse_affiliation(text: &str) -> Result<AffiliationInfo, GrobidError> {
    with_env(|env, engine| {
        let j_text = env.new_string(text)?;
        
        let result = env.call_method(
            engine,
            "processAffiliation",
            "(Ljava/lang/String;)Lorg/grobid/core/data/Affiliation;",
            &[JValue::Object(&j_text)]
        )?.l()?;
        
        // Extract fields from the Affiliation object
        let institution = get_field_as_string(env, result, "institution")?;
        let department = get_field_as_string(env, result, "department")?;
        let country = get_field_as_string(env, result, "country")?;
        
        Ok(AffiliationInfo {
            institution,
            department,
            country,
            // Other fields...
        })
    })
}

fn get_field_as_string(env: &mut JNIEnv, obj: JObject, field_name: &str) -> Result<Option<String>, GrobidError> {
    let field = env.get_field(obj, field_name, "Ljava/lang/String;")?;
    
    if field.l()?.is_null() {
        return Ok(None);
    }
    
    let string_obj = field.l()?;
    let java_string = JString::from(string_obj);
    Ok(Some(env.get_string(&java_string)?.into()))
}
```

### 7.3. PDF Layout Analysis

Extract document structure and layout information:

```rust
pub fn segment_pdf(pdf: &Path) -> Result<DocumentSegmentation, GrobidError> {
    with_env(|env, engine| {
        // Create File object
        let file_obj = create_file_object(env, pdf)?;
        
        // Get DocumentSource
        let source_class = env.find_class("org/grobid/core/document/DocumentSource")?;
        let source = env.new_object(
            source_class,
            "(Ljava/io/File;)V",
            &[JValue::Object(&file_obj)]
        )?;
        
        // Get DocumentParser
        let parser_class = env.find_class("org/grobid/core/document/DocumentParser")?;
        let parser = env.new_object(parser_class, "()V", &[])?;
        
        // Process the document
        let doc = env.call_method(
            parser,
            "processing", 
            "(Lorg/grobid/core/document/DocumentSource;Z)Lorg/grobid/core/document/Document;",
            &[JValue::Object(&source), JValue::Bool(1)]  // 1 = true for generateTeiIds
        )?.l()?;
        
        // Extract segmentation information
        // ...
        
        Ok(DocumentSegmentation {
            // Populate with extracted data
        })
    })
}
```

## 8. Summary

This document has covered advanced aspects of the grobid-rs library, focusing on debugging, JNI concepts, performance optimization, robustness, security, and advanced Grobid API usage. By leveraging these techniques, you can create reliable, secure, and efficient applications that fully utilize Grobid's document processing capabilities from Rust.