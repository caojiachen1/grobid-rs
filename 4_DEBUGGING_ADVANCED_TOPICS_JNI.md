# Debugging and Advanced JNI Topics for Rust-Grobid Integration

## 1. Introduction

Integrating Java code into Rust via JNI can sometimes lead to complex issues. This document covers common debugging strategies, advanced JNI concepts like memory management and threading, and best practices relevant to a Rust-Grobid setup.

## 2. Debugging JNI Issues

### 2.1. Enabling JVM JNI Checks

The JVM provides an option to enable extensive JNI checks, which can help detect common JNI errors made by native code. This is highly recommended during development.
*   **How:** Add the option `-Xcheck:jni` to your `InitArgsBuilder` when launching the JVM.
    ```rust
    // In your JVM initialization logic:
    let jvm_args = InitArgsBuilder::new()
        // ... other options ...
        .option("-Xcheck:jni")
        .build()
        // ...
    ```
*   **What it does:** The JVM will perform more rigorous validation of JNI calls, such as checking for invalid pointers, incorrect use of local/global references, and pending exceptions. It will print detailed error messages if issues are found, often pinpointing the problematic JNI call.
*   **Performance:** This option incurs a significant performance overhead, so it should only be used for debugging and not in production builds.

### 2.2. Handling and Inspecting Java Exceptions

Java exceptions that occur during JNI calls do not automatically propagate as Rust panics or errors. You **must** explicitly check for them.

*   **The `exception_check()` pattern:** After any JNI call that might throw an exception (e.g., `call_method`, `new_object`, `find_class`):
    1.  Call `env.exception_check()?`. If it returns `true`, an exception is pending.
    2.  Get the exception object: `let exception_obj = env.exception_occurred()?`.
    3.  Print details (optional but helpful): `env.exception_describe()?`. This prints the Java stack trace to `stderr`.
    4.  **Clear the exception:** `env.exception_clear()?`. This is **critical**. If you don't clear it, most subsequent JNI calls will fail or behave unpredictably.
    5.  Convert the Java exception to a Rust error: Get the exception message (`exception_obj.call_method(&env, "toString", "()Ljava/lang/String;", &[])`), convert it to a Rust `String`, and return it as part of your custom Rust error type.

The `with_env` helper function in `grobid-rs/src/lib.rs` demonstrates a robust way to centralize this exception checking logic.

### 2.3. Common JNI Errors and Their Causes

*   **`ClassNotFoundException`**
    *   **Symptom:** `env.find_class(...)` fails or a Java call throws this.
    *   **Cause:** The JVM cannot find the specified class on its classpath.
        *   Incorrect `-Djava.class.path` JVM argument (missing JARs, wrong paths).
        *   Typo in the class name string (remember to use `/` instead of `.` as package separators, e.g., `"java/lang/String"`).
        *   The JAR containing the class is not actually on the classpath at runtime.
    *   **Fix:** Verify `java.class.path`. Ensure `grobid-core.jar` (and any other dependencies if not using a "onejar") is correctly specified. Use `javap` or examine JAR contents to confirm class names.

*   **`NoSuchMethodError`**
    *   **Symptom:** `env.call_method(...)` or `env.call_static_method(...)` fails.
    *   **Cause:** The JVM found the class but couldn't find a method matching the provided name **and JNI signature string**.
        *   Typo in the method name.
        *   Incorrect JNI method signature string (e.g., wrong parameter types, incorrect return type, mismatched `Lfully/qualified/ClassName;`).
    *   **Fix:** Double-check the method name. **Use `javap -s YourClass.class`** (run on the compiled `.class` file) to get the exact JNI signatures for all methods in a class. Ensure your signature string in Rust matches *exactly*.

*   **`UnsatisfiedLinkError`**
    *   **Symptom:** JVM fails to load a native library required by Java code (e.g., when Grobid tries to load the Wapiti JNI library).
    *   **Cause:** The path specified in `-Djava.library.path` is incorrect, or the native library (`.so`, `.dylib`, `.dll`) is missing from that path, is for the wrong architecture, or lacks execute permissions.
    *   **Fix:** Verify that `-Djava.library.path` points to the correct directory within `grobid-home` (e.g., `grobid-home/lib/lin-64/`). Ensure the required Wapiti library file exists there, is for the correct OS/architecture, and is executable.

*   **Crashes / Segmentation Faults / JVM Aborts**
    *   **Symptom:** The process crashes, often with minimal Rust-side error information before the crash.
    *   **Cause:** Usually due to more severe JNI misuse:
        *   Using a `JNIEnv` pointer in the wrong thread (each thread must attach and get its own `JNIEnv`).
        *   Using a local reference (`JObject<'local>`) after it has become invalid (e.g., after the `JNIEnv` it was tied to is detached, or after a local frame is popped).
        *   Passing invalid data to JNI functions (e.g., null pointers where not expected).
        *   Incorrectly managing global references (e.g., using a deleted one).
        *   Buffer overflows if manually constructing Java data structures from native memory without proper checks.
    *   **Fix:** Enable `-Xcheck:jni`. Review code for correct `JNIEnv` handling per thread. Scrutinize lifetimes of `JObject`s. Ensure global references are managed correctly. If using `unsafe` blocks for JNI, be extremely careful.

*   **`JNI DETECTED ERROR IN APPLICATION: use of invalid jobject` (or similar from `-Xcheck:jni`)**
    *   **Symptom:** JVM aborts with a JNI diagnostic message.
    *   **Cause:** Often using a local reference that has been freed or belongs to a different `JNIEnv` scope.
    *   **Fix:** Carefully review how `JObject`s (especially local ones) are passed around. Ensure they are not used beyond their valid lifetime (e.g., outside the `with_env` scope if created there, unless converted to a `GlobalRef`).

### 2.4. Logging

*   **Grobid Logging:** Grobid uses SLF4J. If an SLF4J binding (like `slf4j-simple.jar` or `logback-classic.jar`) and its configuration are on the classpath, Grobid will output logs. These can be invaluable for diagnosing Grobid-internal issues.
*   **Rust Logging:** Use the `log` crate and an implementation like `env_logger` in your Rust code.
*   **Correlating Logs:** Consistent timestamps and log formats can help correlate Rust-side activity with Java-side logs.

## 3. Advanced JNI Concepts

### 3.1. Memory Management: Local vs. Global References

The JVM tracks Java objects passed to native code using JNI references. Understanding the types of references is crucial to avoid memory leaks or crashes.

*   **Local References (`JObject<'local>` in `jni` crate):**
    *   **Scope:** Valid only within the duration of a single native method call (i.e., within the scope of the current `JNIEnv` attachment or a specific local frame).
    *   **Lifetime:** Automatically freed by the JVM when the native method returns to Java, or when the thread detaches (managed by `AttachGuard` in `jni-rs`), or when an explicit local frame created with `env.with_local_frame()` is exited.
    *   **Limit:** There's a limited number of local references that a `JNIEnv` can hold active at one time (e.g., HotSpot's default is often small, like 16 or 32, when calling a native method from Java; the limit is higher when Java is called from native code, but still finite). If you create many local references in a loop without freeing them or using local frames, you can exhaust this limit, leading to errors.
    *   **Usage:** Most `JObject`s returned by `jni` crate functions are local references by default.
    *   **Management:**
        *   `env.delete_local_ref(obj)`: Explicitly delete a local reference if it's no longer needed before its natural scope ends (rarely needed with `jni-rs` due to lifetimes and `AttachGuard`).
        *   `env.with_local_frame(capacity, || { ... })`: Creates a nested scope for local references. All local references created inside the closure are freed when the closure exits (except for any object explicitly returned from the closure and promoted).
        *   The `AttachGuard` obtained from `jvm.attach_current_thread()` effectively manages a top-level local frame for the duration of the attachment.

*   **Global References (`GlobalRef` in `jni` crate):**
    *   **Scope:** Valid across multiple native method calls and can be shared between different threads (if access is properly synchronized).
    *   **Lifetime:** Remain valid until explicitly freed by calling `env.delete_global_ref(global_ref_instance)`.
    *   **Creation:** `env.new_global_ref(local_jobject)?` creates a global reference from a local one.
    *   **Usage:** Essential for caching Java objects that need to live beyond a single JNI call or be accessed from different `JNIEnv` contexts. The Grobid `Engine` instance in `grobid-rs` is a prime example, stored in `static ENGINE: OnceCell<Mutex<GlobalRef>>`.
    *   **Memory Leaks:** Failure to call `env.delete_global_ref()` on `GlobalRef`s that are no longer needed will lead to memory leaks in the JVM (the Java objects will not be garbage collected).

*   **Weak Global References (`WeakRef`):** Similar to global references but do not prevent the underlying Java object from being garbage collected if there are no other strong references to it. Less common for this type of embedding but useful for caches that shouldn't keep objects alive indefinitely.

### 3.2. Threading Considerations

*   **`JavaVM` vs. `JNIEnv`:**
    *   `JavaVM`: Represents the entire JVM. There is one per process. It is thread-safe and can be shared among multiple Rust threads.
    *   `JNIEnv`: Represents the JNI environment for a *specific attached thread*. It is **not** thread-safe and **must not** be shared between Rust threads. Each Rust thread that needs to interact with Java must obtain its own `JNIEnv`.

*   **Attaching and Detaching Threads:**
    *   `jvm.attach_current_thread()?` (or `attach_current_thread_as_daemon()`): Attaches the calling Rust thread to the JVM and returns an `AttachGuard`. The `AttachGuard` dereferences to the thread-specific `JNIEnv`.
    *   When the `AttachGuard` goes out of scope (RAII), it automatically detaches the thread from the JVM.
    *   The `with_env` function in `grobid-rs/src/lib.rs` encapsulates this attach/detach logic, ensuring that each call through it operates with a correctly attached `JNIEnv`.

*   **Grobid `Engine` Thread Safety:**
    *   The Grobid `Engine` itself is generally designed to be thread-safe for concurrent processing requests (e.g., in a server environment). It often manages internal model instances or pools.
    *   When using a single `Engine` instance (via a `GlobalRef`) from multiple Rust threads:
        *   The `Mutex` around the `GlobalRef` in `grobid-rs` (`ENGINE: OnceCell<Mutex<GlobalRef>>`) serializes the *acquisition* of the `GlobalRef` to create a local `JObject` reference to the engine for a given `JNIEnv`.
        *   The actual calls to Grobid methods using this `JObject` are then made. If the `Engine`'s Java methods are internally thread-safe, multiple Rust threads can, in principle, call them concurrently after obtaining their local `JObject` reference to the engine via their respective `JNIEnv`s.
        *   However, the `with_env` pattern in `src/lib.rs` as currently structured (where the entire `f` closure, including the Grobid call, runs within the scope of the `Mutex` lock on the `GlobalRef`) effectively serializes all Grobid calls from Rust. This is a safe default but might be a bottleneck if true parallelism is desired at the Rust-to-Java call level. For higher throughput with a single engine, one might need to refine this to lock only for `GlobalRef::as_obj()` and then ensure the Java methods themselves are safe for concurrent calls from different `JNIEnv`s.

### 3.3. `JValue` and Method Arguments

When calling Java methods, arguments are passed as a slice of `JValue`. The `jni` crate provides several ways to convert Rust types and `JObject` wrappers into `JValue`:
*   `JValue::from(some_jobject_or_primitive_wrapper)`
*   `(&some_jstring).into()` or `some_jobject.into()` (leveraging `From` and `Into` trait implementations).
*   For primitive Java types, use the `JValue` constructors directly like `JValue::Int(value)`, `JValue::Bool(value as u8)` etc., though often the wrappers (`JInt`, `JBoolean`) are more convenient for type safety before converting to `JValue`.

Be meticulous about matching the types of `JValue`s in the slice to the types specified in the JNI method signature string.

## 4. Best Practices for Robust JNI Integration

*   **Isolate JNI Code:** Encapsulate all direct JNI interactions within a dedicated Rust module. Expose a safe, idiomatic Rust API from this module to the rest of your application. The `src/lib.rs` in `grobid-rs` with its `GrobidError`, `init`, `with_env`, and specific processing functions is a good example.
*   **Minimize `unsafe` Code:** Rely on the `jni` crate's safe wrappers as much as possible. `unsafe` blocks should be minimal and only used where absolutely necessary (e.g., `JObject::from_raw` if converting raw JNI types, but this should be rare when staying within `jni-rs` abstractions).
*   **Single `JavaVM`:** Initialize the `JavaVM` once per process and store it statically (e.g., in an `OnceCell`).
*   **Manage `GlobalRef`s Carefully:** For long-lived Java objects like the Grobid `Engine`, use `GlobalRef` and ensure they are deleted if the application supports re-initialization or shutdown of the Grobid service component (though for a typical CLI or single-run process, they live for the process duration).
*   **Comprehensive Error Handling:** Convert all `jni::errors::Error` and checked Java exceptions into a unified Rust error type.
*   **Resource Management:** Ensure paths to `grobid-home`, JARs, and native libraries are correctly configured and that these resources are accessible at runtime.
*   **Testing:** Test JNI integration thoroughly on all target platforms and architectures. Test error paths and exception handling.

By applying these debugging techniques and understanding these advanced JNI concepts, you can build more reliable and maintainable Rust applications that leverage the power of Java libraries like Grobid. 