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
    ImportFailed { message: String },

    /// Export operation failed
    #[cfg(feature = "export")]
    #[error("Export failed: {message}")]
    ExportFailed { message: String },

    /// Invalid file path or file not found
    #[error("File error: {message}")]
    FileError { message: String },

    /// Invalid parameters or configuration
    #[error("Invalid parameter: {message}")]
    InvalidParameter { message: String },

    /// Memory allocation failed
    #[error("Memory allocation failed")]
    OutOfMemory,

    /// Unsupported file format
    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    /// I/O operation failed
    #[error("I/O error: {message}")]
    IoError { message: String },

    /// Invalid scene data
    #[error("Invalid scene: {message}")]
    InvalidScene { message: String },

    /// String conversion error (UTF-8)
    #[error("String conversion error: {0}")]
    StringConversion(#[from] std::str::Utf8Error),

    /// Null pointer error
    #[error("Null pointer encountered")]
    NullPointer,

    /// Generic error with custom message
    #[error("{message}")]
    Other { message: String },
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
            message: message.into(),
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

    /// Get the last error from Assimp
    pub fn from_assimp() -> Self {
        unsafe {
            let error_ptr = crate::sys::aiGetErrorString();
            if error_ptr.is_null() {
                Self::Other {
                    message: "Unknown Assimp error".to_string(),
                }
            } else {
                match CStr::from_ptr(error_ptr).to_str() {
                    Ok(error_str) => Self::Other {
                        message: error_str.to_string(),
                    },
                    Err(_) => Self::Other {
                        message: "Invalid UTF-8 in Assimp error message".to_string(),
                    },
                }
            }
        }
    }
}

/// Check if a pointer is null and return an error if it is
pub(crate) fn check_null_ptr<T>(ptr: *const T, context: &str) -> Result<*const T> {
    if ptr.is_null() {
        Err(Error::NullPointer)
    } else {
        Ok(ptr)
    }
}

/// Check if a mutable pointer is null and return an error if it is
pub(crate) fn check_null_ptr_mut<T>(ptr: *mut T, context: &str) -> Result<*mut T> {
    if ptr.is_null() {
        Err(Error::NullPointer)
    } else {
        Ok(ptr)
    }
}

/// Convert a C string to a Rust string, handling null pointers
pub(crate) fn c_str_to_string(ptr: *const std::os::raw::c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(Error::NullPointer);
    }

    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_string())
            .map_err(Error::from)
    }
}

/// Convert a C string to a Rust string, returning empty string for null pointers
pub(crate) fn c_str_to_string_or_empty(ptr: *const std::os::raw::c_char) -> String {
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
    fn test_null_pointer_check() {
        let null_ptr: *const i32 = std::ptr::null();
        let result = check_null_ptr(null_ptr, "test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NullPointer));
    }

    #[test]
    fn test_valid_pointer_check() {
        let value = 42i32;
        let ptr = &value as *const i32;
        let result = check_null_ptr(ptr, "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ptr);
    }
}
