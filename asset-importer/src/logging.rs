//! Logging system integration with Assimp
//!
//! This module provides safe Rust wrappers around Assimp's logging functionality,
//! allowing you to capture and handle log messages from the Assimp library.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

use crate::{error::Result, sys};
use bitflags::bitflags;

/// Log levels supported by Assimp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Verbose debug information
    Debug,
    /// Informational messages
    Info,
    /// Warning messages
    Warn,
    /// Error messages
    Error,
}

/// Trait for custom log stream implementations
pub trait LogStream: Send + Sync {
    /// Write a log message
    fn write(&mut self, message: &str);
}

/// A log stream that writes to stdout
pub struct StdoutLogStream;

impl LogStream for StdoutLogStream {
    fn write(&mut self, message: &str) {
        print!("{}", message);
    }
}

/// A log stream that writes to stderr
pub struct StderrLogStream;

impl LogStream for StderrLogStream {
    fn write(&mut self, message: &str) {
        eprint!("{}", message);
    }
}

/// A log stream that writes to a file
pub struct FileLogStream {
    file: std::fs::File,
}

impl FileLogStream {
    /// Create a new file log stream
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| crate::error::Error::io_error(e.to_string()))?;

        // Write header
        writeln!(file, "=== Assimp Log Started ===")
            .map_err(|e| crate::error::Error::io_error(e.to_string()))?;

        Ok(Self { file })
    }
}

impl LogStream for FileLogStream {
    fn write(&mut self, message: &str) {
        use std::io::Write;
        let _ = self.file.write_all(message.as_bytes());
        let _ = self.file.flush();
    }
}

/// A log stream that collects messages in memory
pub struct MemoryLogStream {
    messages: Vec<String>,
}

impl MemoryLogStream {
    /// Create a new memory log stream
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Get all collected messages
    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    /// Clear all collected messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl Default for MemoryLogStream {
    fn default() -> Self {
        Self::new()
    }
}

impl LogStream for MemoryLogStream {
    fn write(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }
}

/// Internal structure to hold the Rust log stream
struct LogStreamWrapper {
    stream: Arc<Mutex<dyn LogStream>>,
}

/// Global logger instance
pub struct Logger {
    streams: Vec<OwnedLogStream>,
}

struct OwnedLogStream {
    ai_stream: sys::aiLogStream,
    wrapper_ptr: *mut LogStreamWrapper,
    stream: Arc<Mutex<dyn LogStream>>,
}

// SAFETY: OwnedLogStream is only used within the Logger which is protected by a Mutex
// The wrapper_ptr is only accessed from the main thread and is properly cleaned up
unsafe impl Send for OwnedLogStream {}
unsafe impl Sync for OwnedLogStream {}

impl Logger {
    /// Create a new logger
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// Attach a log stream
    pub fn attach_stream(&mut self, stream: Arc<Mutex<dyn LogStream>>) -> Result<()> {
        // Allocate wrapper and keep pointer for cleanup
        let wrapper = LogStreamWrapper {
            stream: stream.clone(),
        };
        let wrapper_ptr = Box::into_raw(Box::new(wrapper));

        let ai_stream = sys::aiLogStream {
            callback: Some(log_callback),
            user: wrapper_ptr as *mut c_char,
        };

        unsafe {
            sys::aiAttachLogStream(&ai_stream);
        }

        self.streams.push(OwnedLogStream {
            ai_stream,
            wrapper_ptr,
            stream,
        });
        Ok(())
    }

    /// Detach a log stream
    pub fn detach_stream(&mut self, stream: Arc<Mutex<dyn LogStream>>) -> Result<()> {
        // Find owned stream
        if let Some(pos) = self
            .streams
            .iter()
            .position(|s| Arc::ptr_eq(&s.stream, &stream))
        {
            let owned = self.streams.remove(pos);
            unsafe {
                // Detach this single stream
                sys::aiDetachLogStream(&owned.ai_stream);
                // Free wrapper
                let _ = Box::from_raw(owned.wrapper_ptr);
            }
        }
        Ok(())
    }

    /// Detach all log streams
    pub fn detach_all_streams(&mut self) {
        unsafe {
            sys::aiDetachAllLogStreams();
        }
        // Free all wrappers
        for s in self.streams.drain(..) {
            unsafe {
                let _ = Box::from_raw(s.wrapper_ptr);
            }
        }
    }

    /// Enable or disable verbose logging
    pub fn enable_verbose_logging(&self, enable: bool) {
        unsafe {
            sys::aiEnableVerboseLogging(if enable { 1 } else { 0 });
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        self.detach_all_streams();
    }
}

/// C callback function for log messages
extern "C" fn log_callback(message: *const c_char, user: *mut c_char) {
    if message.is_null() || user.is_null() {
        return;
    }

    unsafe {
        // Convert C string to Rust string
        let c_str = CStr::from_ptr(message);
        let msg = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return, // Invalid UTF-8
        };

        // Get the wrapper from user data
        let wrapper = &mut *(user as *mut LogStreamWrapper);

        // Write to the stream
        if let Ok(mut stream) = wrapper.stream.lock() {
            stream.write(msg);
        }
    }
}

/// Global logger instance
static GLOBAL_LOGGER: std::sync::OnceLock<std::sync::Mutex<Logger>> = std::sync::OnceLock::new();

/// Get the global logger instance
pub fn global_logger() -> &'static std::sync::Mutex<Logger> {
    GLOBAL_LOGGER.get_or_init(|| std::sync::Mutex::new(Logger::new()))
}

/// Convenience function to attach a stdout log stream
pub fn attach_stdout_stream() -> Result<()> {
    let stream = Arc::new(Mutex::new(StdoutLogStream));
    global_logger().lock().unwrap().attach_stream(stream)
}

/// Convenience function to attach a stderr log stream
pub fn attach_stderr_stream() -> Result<()> {
    let stream = Arc::new(Mutex::new(StderrLogStream));
    global_logger().lock().unwrap().attach_stream(stream)
}

// === Default log streams (Assimp-provided) ===

bitflags! {
    /// Predefined log stream destinations in Assimp
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DefaultLogStreams: u32 {
        /// Log to a file
        const FILE     = sys::aiDefaultLogStream::aiDefaultLogStream_FILE as u32;
        /// Log to standard output
        const STDOUT   = sys::aiDefaultLogStream::aiDefaultLogStream_STDOUT as u32;
        /// Log to standard error
        const STDERR   = sys::aiDefaultLogStream::aiDefaultLogStream_STDERR as u32;
        /// Log to the system debugger
        const DEBUGGER = sys::aiDefaultLogStream::aiDefaultLogStream_DEBUGGER as u32;
    }
}

/// Attach predefined log streams provided by Assimp.
/// For `FILE`, provide a file path; for others, `file_path` is ignored.
pub fn attach_default_streams(
    streams: DefaultLogStreams,
    file_path: Option<&std::path::Path>,
) -> Result<()> {
    unsafe {
        if streams.contains(DefaultLogStreams::FILE) {
            let path = file_path.ok_or_else(|| {
                crate::error::Error::invalid_parameter("file path required for FILE log stream")
            })?;
            let c_path = std::ffi::CString::new(path.to_string_lossy().as_ref())
                .map_err(|_| crate::error::Error::invalid_parameter("invalid file path"))?;
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_FILE,
                c_path.as_ptr(),
            );
            sys::aiAttachLogStream(&s);
        }
        if streams.contains(DefaultLogStreams::STDOUT) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_STDOUT,
                std::ptr::null(),
            );
            sys::aiAttachLogStream(&s);
        }
        if streams.contains(DefaultLogStreams::STDERR) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_STDERR,
                std::ptr::null(),
            );
            sys::aiAttachLogStream(&s);
        }
        if streams.contains(DefaultLogStreams::DEBUGGER) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_DEBUGGER,
                std::ptr::null(),
            );
            sys::aiAttachLogStream(&s);
        }
    }
    Ok(())
}

/// Detach predefined log streams. For `FILE`, provide the same path used when attaching.
pub fn detach_default_streams(streams: DefaultLogStreams, file_path: Option<&std::path::Path>) {
    unsafe {
        if streams.contains(DefaultLogStreams::FILE) {
            if let Some(path) = file_path {
                if let Ok(c_path) = std::ffi::CString::new(path.to_string_lossy().as_ref()) {
                    let s = sys::aiGetPredefinedLogStream(
                        sys::aiDefaultLogStream::aiDefaultLogStream_FILE,
                        c_path.as_ptr(),
                    );
                    let _ = sys::aiDetachLogStream(&s);
                }
            }
        }
        if streams.contains(DefaultLogStreams::STDOUT) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_STDOUT,
                std::ptr::null(),
            );
            let _ = sys::aiDetachLogStream(&s);
        }
        if streams.contains(DefaultLogStreams::STDERR) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_STDERR,
                std::ptr::null(),
            );
            let _ = sys::aiDetachLogStream(&s);
        }
        if streams.contains(DefaultLogStreams::DEBUGGER) {
            let s = sys::aiGetPredefinedLogStream(
                sys::aiDefaultLogStream::aiDefaultLogStream_DEBUGGER,
                std::ptr::null(),
            );
            let _ = sys::aiDetachLogStream(&s);
        }
    }
}

/// Convenience function to attach a file log stream
pub fn attach_file_stream<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
    let stream = Arc::new(Mutex::new(FileLogStream::new(path)?));
    global_logger().lock().unwrap().attach_stream(stream)
}

/// Convenience function to enable verbose logging
pub fn enable_verbose_logging(enable: bool) {
    global_logger()
        .lock()
        .unwrap()
        .enable_verbose_logging(enable);
}

/// Detach all log streams (both default and custom). This mirrors aiDetachAllLogStreams.
pub fn detach_all_streams() {
    global_logger().lock().unwrap().detach_all_streams();
}
