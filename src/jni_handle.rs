use jni::{objects::*, JNIEnv};
use std::ops::{Deref, DerefMut};
use tracing::{error, trace, warn};
use crate::{GrobidError, JVM, ENGINE};

/// Safe JNI Handle for Grobid operations.
///
/// This struct provides a safer interface to JNI calls by automatically
/// handling thread attachment, context classloader setup, and exception handling.
/// It implements `Deref<Target=JNIEnv>` and `DerefMut<Target=JNIEnv>` for easy access 
/// to JNI environment methods.
pub struct JniHandle {
    /// JNI environment 
    env: JNIEnv<'static>,
}

impl JniHandle {
    /// Create a new JNI handle by attaching to the current thread
    pub fn attach() -> Result<Self, GrobidError> {
        trace!("Attaching JNI handle to current thread");
        
        let jvm = match JVM.get() {
            Some(jvm) => jvm,
            None => {
                error!("Cannot attach JNI handle: Grobid not initialized");
                return Err(GrobidError::NotInitialised);
            }
        };
        
        let guard = match jvm.attach_current_thread() {
            Ok(guard) => {
                trace!("Successfully attached to JVM thread");
                guard
            },
            Err(e) => {
                error!("Failed to attach to JVM thread: {}", e);
                return Err(GrobidError::Jni(e));
            }
        };
        
        // This is safe because the JVM is a global static and outlives all JNI operations
        // Note: this assumes the AttachGuard and JNIEnv have compatible layouts which may not be guaranteed
        let env = unsafe { std::mem::transmute_copy(&guard) };
        
        // Don't drop the guard as it would detach the thread
        std::mem::forget(guard);
        
        let mut handle = Self { env };
        
        // Set up thread context classloader for JSONIC compatibility
        handle.setup_context_classloader()?;
        
        Ok(handle)
    }
    
    /// Set up the thread's context classloader
    fn setup_context_classloader(&mut self) -> Result<(), GrobidError> {
        let thread_cls = self.env.find_class("java/lang/Thread").map_err(GrobidError::Jni)?;
        let current_thread = self.env.call_static_method(thread_cls, "currentThread", "()Ljava/lang/Thread;", &[])
            .map_err(GrobidError::Jni)?.l().map_err(GrobidError::Jni)?;
        
        let system_cls = self.env.find_class("java/lang/ClassLoader").map_err(GrobidError::Jni)?;
        let system_classloader = self.env.call_static_method(system_cls, "getSystemClassLoader", "()Ljava/lang/ClassLoader;", &[])
            .map_err(GrobidError::Jni)?.l().map_err(GrobidError::Jni)?;
        
        self.env.call_method(current_thread, "setContextClassLoader", "(Ljava/lang/ClassLoader;)V", &[JValue::Object(&system_classloader)])
            .map_err(GrobidError::Jni)?;
            
        Ok(())
    }
    
    /// Get the Grobid engine object that can be used for method calls
    pub fn engine(&self) -> Result<JObject<'static>, GrobidError> {
        let engine_obj = ENGINE.get().ok_or(GrobidError::NotInitialised)?;
        let locked_engine_gref = engine_obj.lock().unwrap();
        let raw_engine_ptr = (*locked_engine_gref).as_raw();
        let engine_jobject = self.env.new_local_ref(unsafe { JObject::from_raw(raw_engine_ptr) })?;
        
        // Create a new local reference that will be valid for the duration of this call
        Ok(unsafe { std::mem::transmute_copy(&engine_jobject) })
    }
    
    /// Detect and handle a Java exception, converting it to a GrobidError.
    /// Returns true if an exception was handled, false otherwise.
    pub fn handle_exception(&mut self) -> Result<bool, GrobidError> {
        if self.env.exception_check().map_err(GrobidError::Jni)? {
            let exception = self.env.exception_occurred()?;
            
            // Print details to stderr for debugging
            self.env.exception_describe().ok();
            self.env.exception_clear().ok();
            
            // Convert exception to a meaningful error message
            let msg_obj = self.env.call_method(exception, "toString", "()Ljava/lang/String;", &[]);
            let java_msg = match msg_obj {
                Ok(msg_jval) => match msg_jval.l() {
                    Ok(msg_l) => self.env.get_string(&JString::from(msg_l))
                        .map(|s| s.into())
                        .unwrap_or_else(|_| "Failed to get exception message".to_string()),
                    Err(_) => "Exception object was null or not a String".to_string(),
                },
                Err(_) => "Failed to call toString on exception object".to_string(),
            };
            
            // Log the exception with detailed information
            error!("Java exception occurred: {}", java_msg);
            
            // Return the error
            return Err(GrobidError::Java(java_msg));
        }
        
        Ok(false)
    }
}

impl Deref for JniHandle {
    type Target = JNIEnv<'static>;
    
    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

impl DerefMut for JniHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.env
    }
}

impl Drop for JniHandle {
    fn drop(&mut self) {
        // Clear any pending Java exceptions to prevent JVM crashes
        if let Ok(has_exception) = self.env.exception_check() {
            if has_exception {
                // Log the exception cleanup
                warn!("Cleaning up unhandled Java exception in JniHandle::drop");
                self.env.exception_describe().ok();
                self.env.exception_clear().ok();
            }
        }
        
        trace!("Dropping JNI handle resources");
        // The thread will remain attached to the JVM
        // This is generally fine as most apps keep the JVM attached for their lifetime
    }
}