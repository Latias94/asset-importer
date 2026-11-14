//! Custom I/O system support
//!
//! This module provides integration with Assimp's custom file I/O system,
//! allowing you to implement custom file systems for loading assets from
//! memory, archives, or other sources.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{error::Result, ffi, sys};

type FileSystemHandle = Arc<Mutex<dyn FileSystem>>;

struct FileSystemState {
    file_system: FileSystemHandle,
    failed: Arc<AtomicBool>,
}

#[inline]
unsafe fn file_system_ptr(file_io: *mut sys::aiFileIO) -> Option<*const FileSystemState> {
    if file_io.is_null() {
        return None;
    }
    let align = std::mem::align_of::<sys::aiFileIO>();
    if align > 1 && (file_io as usize) % align != 0 {
        return None;
    }
    let ptr = unsafe { (*file_io).UserData as *mut FileSystemState };
    if ptr.is_null() {
        return None;
    }

    let align = std::mem::align_of::<FileSystemState>();
    if align > 1 && (ptr as usize) % align != 0 {
        return None;
    }

    Some(ptr)
}

#[inline]
pub(crate) fn catch_unwind_or<R>(default: R, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            // Dropping a user-provided panic payload can also panic. Only leak the
            // second payload if necessary to keep unwinding inside Rust.
            if let Err(payload) =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload)))
            {
                std::mem::forget(payload);
            }
            default
        }
    }
}

/// Trait for custom file I/O implementations
pub trait FileSystem: std::fmt::Debug + Send + Sync {
    /// Check if a file exists
    fn exists(&self, path: &str) -> bool;

    /// Open a file for reading.
    ///
    /// Return `std::io::ErrorKind::NotFound` through [`crate::Error::from`] when a path
    /// does not exist. Assimp probes optional sidecar files; other errors fail the operation.
    fn open(&self, path: &str) -> Result<Box<dyn FileStream>>;

    /// Open a file with explicit mode (e.g., "rb", "wb", "ab", "r+b").
    /// Default implementation falls back to `open` for read-only modes.
    fn open_with_mode(&self, path: &str, mode: &str) -> Result<Box<dyn FileStream>> {
        if mode.starts_with('r') && !mode.contains('+') {
            self.open(path)
        } else {
            Err(crate::error::Error::io_error(format!(
                "Unsupported open mode: {}",
                mode
            )))
        }
    }

    /// Get the directory separator character
    fn separator(&self) -> char {
        std::path::MAIN_SEPARATOR
    }
}

/// Trait for file stream operations
pub trait FileStream: Send {
    /// Read data from the stream, returning the number of bytes read.
    /// Partial reads are supported; return zero at end of file.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Write data to the stream, returning the number of bytes written.
    /// Partial writes are supported; returning zero fails a nonempty write.
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
#[derive(Debug)]
pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>> {
        let file = std::fs::File::open(path).map_err(crate::error::Error::from)?;
        Ok(Box::new(StdFileStream::new(file)))
    }

    fn open_with_mode(&self, path: &str, mode: &str) -> Result<Box<dyn FileStream>> {
        use std::fs::OpenOptions;
        let mut options = OpenOptions::new();
        let update = mode.contains('+');
        match mode.as_bytes().first() {
            Some(b'r') => {
                options.read(true).write(update);
            }
            Some(b'w') => {
                options.read(update).write(true).truncate(true).create(true);
            }
            Some(b'a') => {
                options.read(update).append(true).create(true);
            }
            _ => {
                return Err(crate::error::Error::io_error(format!(
                    "Unsupported open mode: {mode}"
                )));
            }
        }

        let file = options.open(path).map_err(crate::error::Error::from)?;
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
        self.file.read(buffer).map_err(crate::error::Error::from)
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        use std::io::Write;
        self.file.write(buffer).map_err(crate::error::Error::from)
    }

    fn tell(&self) -> Result<u64> {
        use std::io::Seek;
        let mut file = &self.file;
        file.stream_position().map_err(crate::error::Error::from)
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(position))
            .map(|_| ())
            .map_err(crate::error::Error::from)
    }

    fn size(&self) -> Result<u64> {
        self.file
            .metadata()
            .map(|m| m.len())
            .map_err(crate::error::Error::from)
    }

    fn flush(&mut self) -> Result<()> {
        use std::io::Write;
        self.file.flush().map_err(crate::error::Error::from)
    }
}

/// Memory-based file system for testing or embedded resources
#[derive(Debug)]
pub struct MemoryFileSystem {
    files: std::collections::HashMap<String, Arc<[u8]>>,
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
        self.files.insert(path.into(), Arc::from(data));
    }

    /// Add a file from a shared byte buffer.
    pub fn add_file_shared<S: Into<String>>(&mut self, path: S, data: Arc<[u8]>) {
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
            Ok(Box::new(ReadOnlyMemoryFileStream::new(data.clone())))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {path}"),
            )
            .into())
        }
    }
}

/// Read-only memory file stream backed by a shared byte buffer.
#[derive(Clone)]
pub struct ReadOnlyMemoryFileStream {
    data: Arc<[u8]>,
    position: usize,
}

impl ReadOnlyMemoryFileStream {
    /// Create a new read-only memory file stream.
    pub fn new(data: Arc<[u8]>) -> Self {
        Self { data, position: 0 }
    }
}

impl FileStream for ReadOnlyMemoryFileStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let available = self.data.len().saturating_sub(self.position);
        let to_read = buffer.len().min(available);

        if to_read > 0 {
            buffer[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
            self.position += to_read;
        }

        Ok(to_read)
    }

    fn tell(&self) -> Result<u64> {
        Ok(self.position as u64)
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        let position = usize::try_from(position)
            .map_err(|_| crate::error::Error::io_error("Seek position too large".to_string()))?;
        if position > self.data.len() {
            return Err(crate::error::Error::io_error(
                "Seek position beyond end of file".to_string(),
            ));
        }
        self.position = position;
        Ok(())
    }

    fn size(&self) -> Result<u64> {
        Ok(self.data.len() as u64)
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
        let end_position = self
            .position
            .checked_add(buffer.len())
            .ok_or_else(|| crate::error::Error::io_error("Write position overflow".to_string()))?;
        if end_position > self.data.len() {
            self.data.resize(end_position, 0);
        }

        self.data[self.position..end_position].copy_from_slice(buffer);
        self.position = end_position;

        Ok(buffer.len())
    }

    fn tell(&self) -> Result<u64> {
        Ok(self.position as u64)
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        let position = usize::try_from(position)
            .map_err(|_| crate::error::Error::io_error("Seek position too large".to_string()))?;
        // Allow seeking beyond current data for write operations.
        self.position = position;
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

/// Owned aiFileIO wrapper which frees its `UserData` on drop.
///
/// Assimp borrows the callback table and `UserData`. Keep this owner alive until
/// the native importer/exporter and all of its streams have been destroyed.
/// The safe import API transfers this owner into the returned scene automatically.
#[derive(Debug)]
pub struct OwnedAiFileIO {
    // Assimp retains this callback table; its address must survive moves of the Rust owner.
    file_io: Box<sys::aiFileIO>,
    failed: Arc<AtomicBool>,
}

impl OwnedAiFileIO {
    pub(crate) fn new(file_system: Arc<Mutex<dyn FileSystem>>) -> Self {
        let failed = Arc::new(AtomicBool::new(false));
        let user_data = Box::into_raw(Box::new(FileSystemState {
            file_system,
            failed: failed.clone(),
        })) as *mut c_char;
        Self {
            file_io: Box::new(sys::aiFileIO {
                OpenProc: Some(file_open_proc),
                CloseProc: Some(file_close_proc),
                UserData: user_data,
            }),
            failed,
        }
    }

    pub(crate) fn has_error(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    /// Get a const pointer to the underlying `aiFileIO`.
    pub(crate) fn as_ptr_sys(&self) -> *const sys::aiFileIO {
        &*self.file_io as *const _
    }

    /// Get a const pointer to the underlying `aiFileIO` (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_ptr(&self) -> *const sys::aiFileIO {
        self.as_ptr_sys()
    }

    /// Get a mutable pointer to the underlying `aiFileIO` (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_mut_ptr(&mut self) -> *mut sys::aiFileIO {
        &mut *self.file_io as *mut _
    }
}

impl Drop for OwnedAiFileIO {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.file_io.UserData as *mut FileSystemState;
            if !ptr.is_null() {
                let align = std::mem::align_of::<FileSystemState>();
                if align <= 1 || (ptr as usize) % align == 0 {
                    drop(Box::from_raw(ptr));
                }
                self.file_io.UserData = ptr::null_mut();
            }
        }
    }
}

impl AssimpFileIO {
    /// Create a new Assimp file I/O wrapper
    pub fn new(file_system: Arc<Mutex<dyn FileSystem>>) -> Self {
        Self { file_system }
    }

    /// Create the aiFileIO structure for use with Assimp
    pub fn create_ai_file_io(&self) -> OwnedAiFileIO {
        OwnedAiFileIO::new(self.file_system.clone())
    }
}

/// Internal structure to hold file stream data
struct FileWrapper {
    stream: Mutex<Box<dyn FileStream>>,
    failed: Arc<AtomicBool>,
    writable: bool,
}

#[inline]
unsafe fn file_wrapper_ptr(file: *mut sys::aiFile) -> Option<*const FileWrapper> {
    if file.is_null() {
        return None;
    }
    let align = std::mem::align_of::<sys::aiFile>();
    if align > 1 && (file as usize) % align != 0 {
        return None;
    }
    let ptr = unsafe { (*file).UserData as *mut FileWrapper };
    if ptr.is_null() {
        return None;
    }

    let align = std::mem::align_of::<FileWrapper>();
    if align > 1 && (ptr as usize) % align != 0 {
        return None;
    }

    Some(ptr)
}

#[inline]
fn with_stream<R: Copy>(
    file: *mut sys::aiFile,
    default: R,
    f: impl FnOnce(&mut dyn FileStream) -> Option<R>,
) -> R {
    unsafe {
        let wrapper_ptr = file_wrapper_ptr(file).unwrap_or(std::ptr::null());
        if wrapper_ptr.is_null() {
            return default;
        }
        let wrapper = &*wrapper_ptr;
        let result = catch_unwind_or(None, || {
            let mut stream = wrapper.stream.lock().ok()?;
            f(&mut **stream)
        });
        result.unwrap_or_else(|| {
            wrapper.failed.store(true, Ordering::Relaxed);
            default
        })
    }
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

    catch_unwind_or(ptr::null_mut(), || unsafe {
        let file_system_ptr = file_system_ptr(file_io).unwrap_or(std::ptr::null());
        if file_system_ptr.is_null() {
            return ptr::null_mut();
        }
        let state = &*file_system_ptr;

        let filename_str = match CStr::from_ptr(filename).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        let mode_str = match CStr::from_ptr(mode).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        // A missing file is normal during format/sidecar probing. Panics and
        // poisoned locks are failures of the custom IO implementation.
        let opened = catch_unwind_or(None, || {
            let fs = state.file_system.lock().ok()?;
            Some(fs.open_with_mode(filename_str, mode_str))
        });
        let stream = match opened {
            Some(Ok(stream)) => stream,
            Some(Err(error)) => {
                if !matches!(&error, crate::Error::IoError { source }
                    if source.kind() == std::io::ErrorKind::NotFound)
                {
                    state.failed.store(true, Ordering::Relaxed);
                }
                return ptr::null_mut();
            }
            None => {
                state.failed.store(true, Ordering::Relaxed);
                return ptr::null_mut();
            }
        };

        let wrapper = Box::new(FileWrapper {
            stream: Mutex::new(stream),
            failed: state.failed.clone(),
            writable: !mode_str.starts_with('r') || mode_str.contains('+'),
        });

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
    })
}

/// C callback for closing files
extern "C" fn file_close_proc(_file_io: *mut sys::aiFileIO, file: *mut sys::aiFile) {
    if file.is_null() {
        return;
    }

    let align = std::mem::align_of::<sys::aiFile>();
    if align > 1 && (file as usize) % align != 0 {
        return;
    }

    catch_unwind_or((), || unsafe {
        // Own aiFile before running user code so a panicking stream destructor
        // cannot skip releasing the callback table.
        let mut file = Box::from_raw(file);
        if let Some(wrapper_ptr) = file_wrapper_ptr(&mut *file) {
            if (*wrapper_ptr).writable {
                with_stream(&mut *file, (), |stream| stream.flush().ok());
            }
            let wrapper = Box::from_raw(wrapper_ptr as *mut FileWrapper);
            file.UserData = ptr::null_mut();
            let failed = wrapper.failed.clone();
            if !catch_unwind_or(false, || {
                drop(wrapper);
                true
            }) {
                failed.store(true, Ordering::Relaxed);
            }
        }
    });
}

/// C callback for reading from files
extern "C" fn file_read_proc(
    file: *mut sys::aiFile,
    buffer: *mut c_char,
    size: usize,
    count: usize,
) -> usize {
    if file.is_null() || buffer.is_null() || size == 0 || count == 0 {
        return 0;
    }

    with_stream(file, 0, |stream| {
        let total_bytes = size.checked_mul(count)?;
        if total_bytes > isize::MAX as usize {
            return None;
        }
        let mut owner = buffer;
        let rust_buffer =
            unsafe { ffi::slice_from_mut_ptr_len(&mut owner, buffer as *mut u8, total_bytes) };

        let mut bytes_read = 0;
        while bytes_read < total_bytes {
            let read = stream.read(&mut rust_buffer[bytes_read..]).ok()?;
            if read == 0 {
                break;
            }
            if read > total_bytes - bytes_read {
                return None;
            }
            bytes_read += read;
        }
        Some(bytes_read / size)
    })
}

/// C callback for writing to files
extern "C" fn file_write_proc(
    file: *mut sys::aiFile,
    buffer: *const c_char,
    size: usize,
    count: usize,
) -> usize {
    if file.is_null() || buffer.is_null() || size == 0 || count == 0 {
        return 0;
    }

    with_stream(file, 0, |stream| {
        let total_bytes = size.checked_mul(count)?;
        if total_bytes > isize::MAX as usize {
            return None;
        }

        let owner = &buffer;
        let data_slice = ffi::slice_from_ptr_len(owner, buffer as *const u8, total_bytes);

        let mut bytes_written = 0;
        while bytes_written < total_bytes {
            let written = stream.write(&data_slice[bytes_written..]).ok()?;
            if written == 0 || written > total_bytes - bytes_written {
                return None;
            }
            bytes_written += written;
        }
        Some(bytes_written / size)
    })
}

/// C callback for getting current file position
extern "C" fn file_tell_proc(file: *mut sys::aiFile) -> usize {
    if file.is_null() {
        return 0;
    }

    with_stream(file, 0, |stream| usize::try_from(stream.tell().ok()?).ok())
}

/// C callback for getting file size
extern "C" fn file_size_proc(file: *mut sys::aiFile) -> usize {
    if file.is_null() {
        return 0;
    }

    with_stream(file, 0, |stream| usize::try_from(stream.size().ok()?).ok())
}

/// C callback for seeking in files
extern "C" fn file_seek_proc(
    file: *mut sys::aiFile,
    offset: usize,
    origin: sys::aiOrigin,
) -> sys::aiReturn {
    if file.is_null() {
        return sys::aiReturn::aiReturn_FAILURE;
    }

    with_stream(file, sys::aiReturn::aiReturn_FAILURE, |stream| {
        let offset = u64::try_from(offset).ok()?;

        let new_position = match origin {
            sys::aiOrigin::aiOrigin_SET => offset,
            sys::aiOrigin::aiOrigin_CUR => stream.tell().ok()?.checked_add(offset)?,
            sys::aiOrigin::aiOrigin_END => stream.size().ok()?.checked_sub(offset)?,
            _ => return None,
        };

        stream.seek(new_position).ok()?;
        Some(sys::aiReturn::aiReturn_SUCCESS)
    })
}

/// C callback for flushing files (no-op for read-only streams)
extern "C" fn file_flush_proc(_file: *mut sys::aiFile) {
    if _file.is_null() {
        return;
    }

    with_stream(_file, (), |stream| stream.flush().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_callbacks_reject_unaligned_pointers() {
        let buf = [0u64; 8];
        let unaligned_io = unsafe { (buf.as_ptr() as *const u8).add(1) } as *mut sys::aiFileIO;
        assert!(unsafe { file_system_ptr(unaligned_io) }.is_none());

        let unaligned_file = unsafe { (buf.as_ptr() as *const u8).add(1) } as *mut sys::aiFile;
        assert!(unsafe { file_wrapper_ptr(unaligned_file) }.is_none());

        // Hardening: close callback should not touch unaligned pointers.
        file_close_proc(std::ptr::null_mut(), unaligned_file);
    }

    #[test]
    fn owned_file_io_drop_rejects_unaligned_userdata_pointers() {
        let fs: Arc<Mutex<dyn FileSystem>> = Arc::new(Mutex::new(DefaultFileSystem));
        let mut owned = OwnedAiFileIO::new(fs);

        // Free the real userdata first to avoid a leak, then poison it with an unaligned pointer
        // and ensure Drop does not attempt to `Box::from_raw` it.
        let ptr = owned.file_io.UserData as *mut FileSystemState;
        unsafe {
            drop(Box::from_raw(ptr));
        }

        let buf = [0u64; 8];
        owned.file_io.UserData =
            unsafe { (buf.as_ptr() as *const u8).add(1) } as *mut std::os::raw::c_char;
        drop(owned);
    }

    #[test]
    fn test_memory_file_system() {
        let mut fs = MemoryFileSystem::new();
        let test_data = b"Hello, World!".to_vec();
        fs.add_file("test.txt", test_data.clone());

        assert!(fs.exists("test.txt"));
        assert!(!fs.exists("nonexistent.txt"));
        assert!(fs.open_with_mode("test.txt", "r+b").is_err());

        let mut stream = fs.open("test.txt").unwrap();
        assert_eq!(stream.size().unwrap(), test_data.len() as u64);

        let mut buffer = vec![0u8; test_data.len()];
        let bytes_read = stream.read(&mut buffer).unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(buffer, test_data);
    }

    #[test]
    fn memory_file_stream_write_rejects_position_overflow() {
        let mut stream = MemoryFileStream {
            data: Vec::new(),
            position: usize::MAX,
        };

        assert!(stream.write(&[1]).is_err());
    }

    #[test]
    fn default_file_system_preserves_update_mode_semantics() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "asset-importer-io-{}-{unique}.tmp",
            std::process::id()
        ));
        let path_str = path.to_str().unwrap();
        let fs = DefaultFileSystem;

        // r+ must not create a file that does not exist.
        assert!(fs.open_with_mode(path_str, "r+b").is_err());
        assert!(!path.exists());
        std::fs::write(&path, b"previous contents").unwrap();

        let mut stream = fs.open_with_mode(path_str, "w+b").unwrap();
        assert_eq!(stream.size().unwrap(), 0);
        assert_eq!(stream.write(b"new").unwrap(), 3);
        stream.flush().unwrap();
        stream.seek(0).unwrap();
        let mut buffer = [0; 3];
        assert_eq!(stream.read(&mut buffer).unwrap(), 3);
        assert_eq!(&buffer, b"new");
        drop(stream);

        let mut stream = fs.open_with_mode(path_str, "a+b").unwrap();
        stream.seek(0).unwrap();
        assert_eq!(stream.write(b"!").unwrap(), 1);
        stream.flush().unwrap();
        drop(stream);
        assert_eq!(std::fs::read(&path).unwrap(), b"new!");
        std::fs::remove_file(path).unwrap();
    }
}
