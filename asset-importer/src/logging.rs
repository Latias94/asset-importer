//! Logging system integration with Assimp
//!
//! This module provides safe Rust wrappers around Assimp's logging functionality.
//!
//! ## Important Note
//!
//! Custom log streams using Assimp's callback mechanism have been removed due to
//! access violations and memory safety issues when crossing the FFI boundary.
//! The callback-based logging system was causing STATUS_ACCESS_VIOLATION errors
//! because of conflicts between Assimp's C callback mechanism and Rust's memory
//! management.
//!
//! ## Available Functionality
//!
//! - Verbose logging control (safe)
//! - Error message retrieval (safe)
//! - Basic logging level configuration (safe)
//!
//! ## Removed Functionality
//!
//! - Custom log streams (unsafe due to FFI callback issues)
//! - Real-time log message capture (unsafe)
//! - File/stdout/stderr stream attachment (unsafe)
//!
//! For applications that need detailed logging, consider:
//! 1. Using verbose logging with `enable_verbose_logging()`
//! 2. Checking error messages with `get_last_error_message()`
//! 3. Implementing application-level logging around import operations

use crate::{error::Result, sys};
use std::ffi::CStr;

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

// Note: Custom log streams have been removed due to FFI callback safety issues.
// The following types are kept for API compatibility but are no longer functional:

/// Trait for custom log stream implementations
///
/// **DEPRECATED**: This trait is no longer functional due to FFI callback safety issues.
/// Custom log streams have been removed to prevent access violations.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use verbose logging instead."
)]
pub trait LogStream: Send + Sync {
    /// Write a log message
    fn write(&mut self, message: &str);
}

/// A log stream that writes to stdout
///
/// **DEPRECATED**: This type is no longer functional.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use verbose logging instead."
)]
pub struct StdoutLogStream;

#[allow(deprecated)]
impl LogStream for StdoutLogStream {
    fn write(&mut self, message: &str) {
        print!("{}", message);
    }
}

/// A log stream that writes to stderr
///
/// **DEPRECATED**: This type is no longer functional.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use verbose logging instead."
)]
pub struct StderrLogStream;

#[allow(deprecated)]
impl LogStream for StderrLogStream {
    fn write(&mut self, message: &str) {
        eprint!("{}", message);
    }
}

/// Safe logger that only provides basic functionality without FFI callbacks
pub struct Logger {
    verbose_enabled: bool,
}

impl Logger {
    /// Create a new logger
    pub fn new() -> Self {
        Self {
            verbose_enabled: false,
        }
    }

    /// Attach a log stream
    ///
    /// **DEPRECATED**: This method is no longer functional due to FFI callback safety issues.
    /// It will return an error to maintain API compatibility.
    #[deprecated(
        note = "Custom log streams removed due to FFI safety issues. Use enable_verbose_logging instead."
    )]
    #[allow(deprecated)]
    pub fn attach_stream(
        &mut self,
        _stream: std::sync::Arc<std::sync::Mutex<dyn LogStream>>,
    ) -> Result<()> {
        Err(crate::error::Error::logging_error(
            "Custom log streams have been disabled due to FFI safety issues. Use enable_verbose_logging() instead.".to_string()
        ))
    }

    /// Detach a specific log stream
    ///
    /// **DEPRECATED**: This method is no longer functional.
    #[deprecated(note = "Custom log streams removed due to FFI safety issues.")]
    #[allow(deprecated)]
    pub fn detach_stream(
        &mut self,
        _stream: std::sync::Arc<std::sync::Mutex<dyn LogStream>>,
    ) -> Result<()> {
        Err(crate::error::Error::logging_error(
            "Custom log streams have been disabled due to FFI safety issues.".to_string(),
        ))
    }

    /// Detach all log streams
    ///
    /// **DEPRECATED**: This method is no longer functional.
    #[deprecated(note = "Custom log streams removed due to FFI safety issues.")]
    pub fn detach_all_streams(&mut self) {
        // No-op: no streams to detach
    }

    /// Enable or disable verbose logging
    pub fn enable_verbose_logging(&mut self, enable: bool) {
        self.verbose_enabled = enable;
        unsafe {
            sys::aiEnableVerboseLogging(if enable { 1 } else { 0 });
        }
    }

    /// Check if verbose logging is enabled
    pub fn is_verbose_enabled(&self) -> bool {
        self.verbose_enabled
    }

    /// Get the last error message from Assimp
    /// This is a safe way to get logging information without callbacks
    pub fn get_last_error(&self) -> Option<String> {
        unsafe {
            let error_ptr = sys::aiGetErrorString();
            if error_ptr.is_null() {
                None
            } else {
                match CStr::from_ptr(error_ptr).to_str() {
                    Ok(s) => Some(s.to_string()),
                    Err(_) => Some("Invalid UTF-8 in error message".to_string()),
                }
            }
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

/// Global logger instance
static GLOBAL_LOGGER: std::sync::OnceLock<std::sync::Mutex<Logger>> = std::sync::OnceLock::new();

/// Get the global logger instance
pub fn global_logger() -> &'static std::sync::Mutex<Logger> {
    GLOBAL_LOGGER.get_or_init(|| std::sync::Mutex::new(Logger::new()))
}

/// Convenience function to attach a stdout log stream
///
/// **DEPRECATED**: This function is no longer functional due to FFI callback safety issues.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use enable_verbose_logging instead."
)]
pub fn attach_stdout_stream() -> Result<()> {
    eprintln!("Warning: Custom log streams have been disabled due to FFI safety issues.");
    eprintln!("Use enable_verbose_logging() instead for safe logging.");
    Err(crate::error::Error::logging_error(
        "Custom log streams have been disabled due to FFI safety issues.".to_string(),
    ))
}

/// Convenience function to attach a stderr log stream
///
/// **DEPRECATED**: This function is no longer functional due to FFI callback safety issues.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use enable_verbose_logging instead."
)]
pub fn attach_stderr_stream() -> Result<()> {
    eprintln!("Warning: Custom log streams have been disabled due to FFI safety issues.");
    eprintln!("Use enable_verbose_logging() instead for safe logging.");
    Err(crate::error::Error::logging_error(
        "Custom log streams have been disabled due to FFI safety issues.".to_string(),
    ))
}

/// Convenience function to attach a file log stream
///
/// **DEPRECATED**: This function is no longer functional due to FFI callback safety issues.
#[deprecated(
    note = "Custom log streams removed due to FFI safety issues. Use enable_verbose_logging instead."
)]
pub fn attach_file_stream<P: AsRef<std::path::Path>>(_path: P) -> Result<()> {
    eprintln!("Warning: Custom log streams have been disabled due to FFI safety issues.");
    eprintln!("Use enable_verbose_logging() instead for safe logging.");
    Err(crate::error::Error::logging_error(
        "Custom log streams have been disabled due to FFI safety issues.".to_string(),
    ))
}

/// Convenience function to enable verbose logging
pub fn enable_verbose_logging(enable: bool) {
    if let Ok(mut logger) = global_logger().lock() {
        logger.enable_verbose_logging(enable);
    }
}

/// Check if verbose logging is enabled
pub fn is_verbose_logging_enabled() -> bool {
    global_logger()
        .lock()
        .map(|l| l.is_verbose_enabled())
        .unwrap_or(false)
}

/// Get the last error message from Assimp
pub fn get_last_error_message() -> Option<String> {
    global_logger().lock().ok().and_then(|l| l.get_last_error())
}

/// Detach all log streams (both default and custom).
///
/// **DEPRECATED**: This function is no longer functional.
#[deprecated(note = "Custom log streams removed due to FFI safety issues.")]
pub fn detach_all_streams() {
    // No-op: no streams to detach
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new();
        assert!(!logger.is_verbose_enabled());
    }

    #[test]
    fn test_verbose_logging_toggle() {
        let mut logger = Logger::new();

        // Test enabling
        logger.enable_verbose_logging(true);
        assert!(logger.is_verbose_enabled());

        // Test disabling
        logger.enable_verbose_logging(false);
        assert!(!logger.is_verbose_enabled());
    }

    #[test]
    fn test_global_logger() {
        enable_verbose_logging(true);
        assert!(is_verbose_logging_enabled());

        enable_verbose_logging(false);
        assert!(!is_verbose_logging_enabled());
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_functions_return_errors() {
        // Test that deprecated functions return appropriate errors
        let result = attach_stdout_stream();
        assert!(result.is_err());

        let result = attach_stderr_stream();
        assert!(result.is_err());

        let result = attach_file_stream("test.log");
        assert!(result.is_err());
    }
}
