//! Custom I/O system support
//!
//! This module provides integration with Assimp's custom file I/O system,
//! allowing you to implement custom file systems for loading assets from
//! memory, archives, or other sources.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Arc, Mutex};

use crate::{error::Result, sys};

/// Trait for custom file I/O implementations
pub trait FileSystem {
    /// Check if a file exists
    fn exists(&self, path: &str) -> bool;

    /// Open a file for reading
    fn open(&self, path: &str) -> Result<Box<dyn FileStream>>;

    /// Get the directory separator character
    fn separator(&self) -> char {
        std::path::MAIN_SEPARATOR
    }
}

/// Trait for file stream operations
pub trait FileStream {
    /// Read data from the stream
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Write data to the stream
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        // Default implementation for read-only streams
        let _ = buffer;
        Err(crate::error::Error::io_error(
            "Write not supported for read-only stream".to_string(),
        ))
    }

    /// Get the current position in the stream
    fn tell(&self) -> Result<u64>;

    /// Seek to a position in the stream
    fn seek(&mut self, position: u64) -> Result<()>;

    /// Get the size of the file
    fn size(&self) -> Result<u64>;

    /// Flush any pending writes (for write streams)
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Default file system implementation using std::fs
pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>> {
        let file =
            std::fs::File::open(path).map_err(|e| crate::error::Error::io_error(e.to_string()))?;
        Ok(Box::new(StdFileStream::new(file)))
    }
}

/// File stream implementation using std::fs::File
pub struct StdFileStream {
    file: std::fs::File,
}

impl StdFileStream {
    fn new(file: std::fs::File) -> Self {
        Self { file }
    }
}

impl FileStream for StdFileStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        self.file
            .read(buffer)
            .map_err(|e| crate::error::Error::io_error(e.to_string()))
    }

    fn tell(&self) -> Result<u64> {
        use std::io::Seek;
        let mut file = &self.file;
        file.stream_position()
            .map_err(|e| crate::error::Error::io_error(e.to_string()))
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(position))
            .map(|_| ())
            .map_err(|e| crate::error::Error::io_error(e.to_string()))
    }

    fn size(&self) -> Result<u64> {
        self.file
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crate::error::Error::io_error(e.to_string()))
    }
}

/// Memory-based file system for testing or embedded resources
pub struct MemoryFileSystem {
    files: std::collections::HashMap<String, Vec<u8>>,
}

impl MemoryFileSystem {
    /// Create a new memory file system
    pub fn new() -> Self {
        Self {
            files: std::collections::HashMap::new(),
        }
    }

    /// Add a file to the memory file system
    pub fn add_file<S: Into<String>>(&mut self, path: S, data: Vec<u8>) {
        self.files.insert(path.into(), data);
    }

    /// Get the number of files in the memory file system
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for MemoryFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MemoryFileSystem {
    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>> {
        if let Some(data) = self.files.get(path) {
            Ok(Box::new(MemoryFileStream::new(data.clone())))
        } else {
            Err(crate::error::Error::file_error(format!(
                "File not found: {}",
                path
            )))
        }
    }
}

/// Memory-based file stream
pub struct MemoryFileStream {
    data: Vec<u8>,
    position: usize,
}

impl MemoryFileStream {
    /// Create a new read-only memory file stream
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }

    /// Create a new writable memory file stream
    pub fn new_writable(initial_capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(initial_capacity),
            position: 0,
        }
    }

    /// Get the current data in the stream
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Take ownership of the data in the stream
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

impl FileStream for MemoryFileStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let available = self.data.len().saturating_sub(self.position);
        let to_read = buffer.len().min(available);

        if to_read > 0 {
            buffer[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
            self.position += to_read;
        }

        Ok(to_read)
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        // Extend the data vector if necessary
        let end_position = self.position + buffer.len();
        if end_position > self.data.len() {
            self.data.resize(end_position, 0);
        }

        // Write the data
        self.data[self.position..end_position].copy_from_slice(buffer);
        self.position = end_position;

        Ok(buffer.len())
    }

    fn tell(&self) -> Result<u64> {
        Ok(self.position as u64)
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        // Allow seeking beyond current data for write operations
        self.position = position as usize;
        Ok(())
    }

    fn size(&self) -> Result<u64> {
        Ok(self.data.len() as u64)
    }
}



/// Wrapper for integrating Rust FileSystem with Assimp's aiFileIO
pub struct AssimpFileIO {
    file_system: Arc<Mutex<dyn FileSystem>>,
}

impl AssimpFileIO {
    /// Create a new Assimp file I/O wrapper
    pub fn new(file_system: Arc<Mutex<dyn FileSystem>>) -> Self {
        Self { file_system }
    }

    /// Create the aiFileIO structure for use with Assimp
    pub fn create_ai_file_io(&self) -> sys::aiFileIO {
        sys::aiFileIO {
            OpenProc: Some(file_open_proc),
            CloseProc: Some(file_close_proc),
            UserData: Box::into_raw(Box::new(self.file_system.clone())) as *mut c_char,
        }
    }
}

/// Internal structure to hold file stream data
struct FileWrapper {
    stream: Box<dyn FileStream>,
}

/// C callback for opening files
extern "C" fn file_open_proc(
    file_io: *mut sys::aiFileIO,
    filename: *const c_char,
    mode: *const c_char,
) -> *mut sys::aiFile {
    if file_io.is_null() || filename.is_null() || mode.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        // Get the file system from user data
        let file_system_ptr = (*file_io).UserData as *mut Arc<Mutex<dyn FileSystem>>;
        let file_system = &*file_system_ptr;

        // Convert filename to Rust string
        let filename_cstr = CStr::from_ptr(filename);
        let filename_str = match filename_cstr.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        // Convert mode to Rust string (for now we only support read mode)
        let mode_cstr = CStr::from_ptr(mode);
        let mode_str = match mode_cstr.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        // Only support read mode for now
        if !mode_str.starts_with('r') {
            return ptr::null_mut();
        }

        // Open the file
        let stream = match file_system.lock() {
            Ok(fs) => match fs.open(filename_str) {
                Ok(stream) => stream,
                Err(_) => return ptr::null_mut(),
            },
            Err(_) => return ptr::null_mut(),
        };

        // Create file wrapper
        let wrapper = Box::new(FileWrapper { stream });

        // Create aiFile structure
        let ai_file = Box::new(sys::aiFile {
            ReadProc: Some(file_read_proc),
            WriteProc: Some(file_write_proc),
            TellProc: Some(file_tell_proc),
            FileSizeProc: Some(file_size_proc),
            SeekProc: Some(file_seek_proc),
            FlushProc: Some(file_flush_proc),
            UserData: Box::into_raw(wrapper) as *mut c_char,
        });

        Box::into_raw(ai_file)
    }
}

/// C callback for closing files
extern "C" fn file_close_proc(_file_io: *mut sys::aiFileIO, file: *mut sys::aiFile) {
    if !file.is_null() {
        unsafe {
            // Clean up the file wrapper
            let wrapper_ptr = (*file).UserData as *mut FileWrapper;
            if !wrapper_ptr.is_null() {
                let _ = Box::from_raw(wrapper_ptr);
            }

            // Clean up the aiFile
            let _ = Box::from_raw(file);
        }
    }
}

/// C callback for reading from files
extern "C" fn file_read_proc(
    file: *mut sys::aiFile,
    buffer: *mut c_char,
    size: usize,
    count: usize,
) -> usize {
    if file.is_null() || buffer.is_null() {
        return 0;
    }

    unsafe {
        let wrapper_ptr = (*file).UserData as *mut FileWrapper;
        if wrapper_ptr.is_null() {
            return 0;
        }

        let wrapper = &mut *wrapper_ptr;
        let total_bytes = size * count;
        let rust_buffer = std::slice::from_raw_parts_mut(buffer as *mut u8, total_bytes);

        match wrapper.stream.read(rust_buffer) {
            Ok(bytes_read) => bytes_read / size,
            Err(_) => 0,
        }
    }
}

/// C callback for writing to files
extern "C" fn file_write_proc(
    file: *mut sys::aiFile,
    buffer: *const c_char,
    size: usize,
    count: usize,
) -> usize {
    if file.is_null() || buffer.is_null() {
        return 0;
    }

    unsafe {
        let wrapper = &mut *((*file).UserData as *mut FileWrapper);
        let total_bytes = size * count;

        if total_bytes == 0 {
            return 0;
        }

        let data_slice = std::slice::from_raw_parts(buffer as *const u8, total_bytes);

        match wrapper.stream.write(data_slice) {
            Ok(bytes_written) => bytes_written / size,
            Err(_) => 0,
        }
    }
}

/// C callback for getting current file position
extern "C" fn file_tell_proc(file: *mut sys::aiFile) -> usize {
    if file.is_null() {
        return 0;
    }

    unsafe {
        let wrapper_ptr = (*file).UserData as *mut FileWrapper;
        if wrapper_ptr.is_null() {
            return 0;
        }

        let wrapper = &*wrapper_ptr;
        match wrapper.stream.tell() {
            Ok(pos) => pos as usize,
            Err(_) => 0,
        }
    }
}

/// C callback for getting file size
extern "C" fn file_size_proc(file: *mut sys::aiFile) -> usize {
    if file.is_null() {
        return 0;
    }

    unsafe {
        let wrapper_ptr = (*file).UserData as *mut FileWrapper;
        if wrapper_ptr.is_null() {
            return 0;
        }

        let wrapper = &*wrapper_ptr;
        match wrapper.stream.size() {
            Ok(size) => size as usize,
            Err(_) => 0,
        }
    }
}

/// C callback for seeking in files
extern "C" fn file_seek_proc(
    file: *mut sys::aiFile,
    offset: usize,
    origin: sys::aiOrigin::Type,
) -> sys::aiReturn::Type {
    if file.is_null() {
        return sys::aiReturn::aiReturn_FAILURE;
    }

    unsafe {
        let wrapper_ptr = (*file).UserData as *mut FileWrapper;
        if wrapper_ptr.is_null() {
            return sys::aiReturn::aiReturn_FAILURE;
        }

        let wrapper = &mut *wrapper_ptr;

        let new_position = match origin {
            sys::aiOrigin::aiOrigin_SET => offset as u64,
            sys::aiOrigin::aiOrigin_CUR => match wrapper.stream.tell() {
                Ok(current) => current + offset as u64,
                Err(_) => return sys::aiReturn::aiReturn_FAILURE,
            },
            sys::aiOrigin::aiOrigin_END => match wrapper.stream.size() {
                Ok(size) => size.saturating_sub(offset as u64),
                Err(_) => return sys::aiReturn::aiReturn_FAILURE,
            },
            _ => return sys::aiReturn::aiReturn_FAILURE,
        };

        match wrapper.stream.seek(new_position) {
            Ok(_) => sys::aiReturn::aiReturn_SUCCESS,
            Err(_) => sys::aiReturn::aiReturn_FAILURE,
        }
    }
}

/// C callback for flushing files (no-op for read-only streams)
extern "C" fn file_flush_proc(_file: *mut sys::aiFile) {
    // No-op for read-only file streams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_file_system() {
        let mut fs = MemoryFileSystem::new();
        let test_data = b"Hello, World!".to_vec();
        fs.add_file("test.txt", test_data.clone());

        assert!(fs.exists("test.txt"));
        assert!(!fs.exists("nonexistent.txt"));

        let mut stream = fs.open("test.txt").unwrap();
        assert_eq!(stream.size().unwrap(), test_data.len() as u64);

        let mut buffer = vec![0u8; test_data.len()];
        let bytes_read = stream.read(&mut buffer).unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(buffer, test_data);
    }
}
