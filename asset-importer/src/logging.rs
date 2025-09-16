//! Logging system integration with Assimp
//!
//! This module provides safe Rust wrappers around Assimp's logging functionality,
//! allowing you to capture and handle log messages from the Assimp library.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

use crate::{error::Result, sys};

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
    streams: Vec<LogStreamWrapper>,
}

impl Logger {
    /// Create a new logger
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// Attach a log stream
    pub fn attach_stream(&mut self, stream: Arc<Mutex<dyn LogStream>>) -> Result<()> {
        let wrapper = LogStreamWrapper { stream };

        // Create the C callback structure
        let ai_stream = sys::aiLogStream {
            callback: Some(log_callback),
            user: Box::into_raw(Box::new(wrapper)) as *mut c_char,
        };

        // Attach to Assimp
        unsafe {
            sys::aiAttachLogStream(&ai_stream);
        }

        Ok(())
    }

    /// Detach a log stream
    pub fn detach_stream(&mut self, stream: Arc<Mutex<dyn LogStream>>) -> Result<()> {
        // Find and remove the stream
        self.streams
            .retain(|wrapper| !Arc::ptr_eq(&wrapper.stream, &stream));

        // Note: We can't easily detach individual streams from Assimp
        // without keeping track of the aiLogStream structures
        // For now, we'll just remove from our list

        Ok(())
    }

    /// Detach all log streams
    pub fn detach_all_streams(&mut self) {
        unsafe {
            sys::aiDetachAllLogStreams();
        }
        self.streams.clear();
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

/// Convenience function to attach a file log stream
pub fn attach_file_stream<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
    let stream = Arc::new(Mutex::new(FileLogStream::new(path)?));
    global_logger().lock().unwrap().attach_stream(stream)
}

/// Convenience function to enable verbose logging
pub fn enable_verbose_logging(enable: bool) {
    global_logger().lock().unwrap().enable_verbose_logging(enable);
}

/// Convenience function to detach all log streams
pub fn detach_all_streams() {
    global_logger().lock().unwrap().detach_all_streams();
}
