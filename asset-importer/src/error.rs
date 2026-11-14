//! Error handling for asset importer operations

use std::ffi::CStr;
use thiserror::Error;

/// Result type alias for asset importer operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during asset import/export operations
#[derive(Error, Debug)]
pub enum Error {
    /// Import operation failed
    #[error("Import failed: {message}")]
    ImportFailed {
        /// Error message describing the import failure
        message: String,
    },

    /// Export operation failed
    #[cfg(feature = "export")]
    #[error("Export failed: {message}")]
    ExportFailed {
        /// Error message describing the export failure
        message: String,
    },

    /// Invalid file path or file not found
    #[error("File error: {message}")]
    FileError {
        /// Error message describing the file error
        message: String,
    },

    /// Invalid parameters or configuration
    #[error("Invalid parameter: {message}")]
    InvalidParameter {
        /// Error message describing the invalid parameter
        message: String,
    },

    /// Memory allocation failed
    #[error("Memory allocation failed")]
    OutOfMemory,

    /// Unsupported file format
    #[error("Unsupported format: {format}")]
    UnsupportedFormat {
        /// The unsupported format name
        format: String,
    },

    /// I/O operation failed
    #[error("I/O error: {source}")]
    IoError {
        /// The original I/O error, including its kind and source.
        #[from]
        source: std::io::Error,
    },

    /// Invalid scene data
    #[error("Invalid scene: {message}")]
    InvalidScene {
        /// Error message describing the scene validation error
        message: String,
    },

    /// String conversion error (UTF-8)
    #[error("String conversion error: {0}")]
    StringConversion(#[from] std::str::Utf8Error),

    /// Null pointer error
    #[error("Null pointer encountered")]
    NullPointer,

    /// Generic error with custom message
    #[error("{message}")]
    Other {
        /// Custom error message
        message: String,
    },
}

impl Error {
    /// Create a new import error
    pub fn import_failed<S: Into<String>>(message: S) -> Self {
        Self::ImportFailed {
            message: message.into(),
        }
    }

    /// Create a new export error
    #[cfg(feature = "export")]
    pub fn export_failed<S: Into<String>>(message: S) -> Self {
        Self::ExportFailed {
            message: message.into(),
        }
    }

    /// Create a new file error
    pub fn file_error<S: Into<String>>(message: S) -> Self {
        Self::FileError {
            message: message.into(),
        }
    }

    /// Create a new invalid parameter error
    pub fn invalid_parameter<S: Into<String>>(message: S) -> Self {
        Self::InvalidParameter {
            message: message.into(),
        }
    }

    /// Create a new unsupported format error
    pub fn unsupported_format<S: Into<String>>(format: S) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }

    /// Create a new I/O error
    pub fn io_error<S: Into<String>>(message: S) -> Self {
        Self::IoError {
            source: std::io::Error::other(message.into()),
        }
    }

    /// Create a new invalid scene error
    pub fn invalid_scene<S: Into<String>>(message: S) -> Self {
        Self::InvalidScene {
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }

    pub(crate) fn from_bridge() -> Self {
        Self::other(bridge_error_string_or_default())
    }

    pub(crate) fn import_failed_from_bridge() -> Self {
        Self::import_failed(bridge_error_string_or_default())
    }
}

pub(crate) fn bridge_error_string_or_default() -> String {
    bridge_error_string()
        .unwrap_or_else(|| "Assimp bridge operation failed without an error message".to_owned())
}

fn bridge_error_string() -> Option<String> {
    let bridge_ptr = unsafe { crate::sys::aiGetLastErrorStringRust() };
    if bridge_ptr.is_null() {
        return None;
    }

    let message = unsafe { CStr::from_ptr(bridge_ptr) }
        .to_string_lossy()
        .into_owned();
    (!message.is_empty()).then_some(message)
}

/// Convert a C string to a Rust string, returning empty string for null pointers
///
/// # Safety
/// A non-null pointer must reference a valid NUL-terminated string that remains
/// readable and unmodified for the duration of this call.
pub(crate) unsafe fn c_str_to_string_or_empty(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("").to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = Error::import_failed("Test import error");
        assert!(matches!(error, Error::ImportFailed { .. }));
        assert_eq!(error.to_string(), "Import failed: Test import error");
    }

    #[test]
    fn test_c_str_to_string_or_empty() {
        // Test with null pointer
        let null_ptr: *const std::os::raw::c_char = std::ptr::null();
        let result = unsafe { c_str_to_string_or_empty(null_ptr) };
        assert_eq!(result, "");

        // Test with valid C string
        let c_string = std::ffi::CString::new("test string").unwrap();
        let result = unsafe { c_str_to_string_or_empty(c_string.as_ptr()) };
        assert_eq!(result, "test string");
    }

    #[test]
    fn bridge_errors_are_local_to_the_calling_thread() {
        assert!(bridge_error_string().is_none());
        std::thread::spawn(|| {
            unsafe {
                let _ = crate::sys::aiImportFileFromMemoryRust(
                    std::ptr::null(),
                    0,
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    None,
                    std::ptr::null_mut(),
                );
            }
            assert!(
                Error::from_bridge()
                    .to_string()
                    .contains("Memory buffer is empty")
            );
        })
        .join()
        .unwrap();
        assert!(bridge_error_string().is_none());
    }
}
