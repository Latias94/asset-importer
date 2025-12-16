//! Texture and embedded texture support
//!
//! This module provides safe Rust wrappers for Assimp's texture functionality,
//! including support for embedded textures that are stored directly within
//! model files.

use std::marker::PhantomData;

use crate::types::ai_string_to_string;
use crate::{
    error::{Error, Result},
    ptr::SharedPtr,
    sys,
};

/// A texel (texture element) in ARGB8888 format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Texel {
    /// Blue component (0-255)
    pub b: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Red component (0-255)
    pub r: u8,
    /// Alpha component (0-255)
    pub a: u8,
}

impl Texel {
    /// Create a new texel with the given RGBA values
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a texel from RGBA values as a tuple
    pub fn from_rgba(rgba: (u8, u8, u8, u8)) -> Self {
        Self::new(rgba.0, rgba.1, rgba.2, rgba.3)
    }

    /// Convert to RGBA tuple
    pub fn to_rgba(self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }

    /// Convert to normalized floating-point RGBA values (0.0-1.0)
    pub fn to_rgba_f32(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    /// Convert to glam Vec4 for easy integration with graphics libraries
    pub fn to_vec4(self) -> crate::types::Vector4D {
        let (r, g, b, a) = self.to_rgba_f32();
        crate::types::Vector4D::new(r, g, b, a)
    }
}

impl From<&sys::aiTexel> for Texel {
    fn from(texel: &sys::aiTexel) -> Self {
        Self {
            b: texel.b,
            g: texel.g,
            r: texel.r,
            a: texel.a,
        }
    }
}

/// Content of texture data
#[derive(Debug, Clone)]
pub enum TextureData {
    /// Uncompressed texture data as texels (when height > 0)
    Texels(Vec<Texel>),
    /// Compressed texture data as raw bytes (when height == 0)
    Compressed(Vec<u8>),
}

/// Borrowed view of texture data (zero-copy).
#[derive(Debug, Clone, Copy)]
pub enum TextureDataRef<'a> {
    /// Uncompressed texels (when height > 0)
    Texels(&'a [sys::aiTexel]),
    /// Compressed raw bytes (when height == 0)
    Compressed(&'a [u8]),
}

/// An embedded texture within a 3D model file
///
/// Textures can be either:
/// 1. Uncompressed - stored as raw ARGB8888 texel data
/// 2. Compressed - stored in a standard format like PNG, JPEG, etc.
#[derive(Debug)]
pub struct Texture<'a> {
    texture_ptr: SharedPtr<sys::aiTexture>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Texture<'a> {
    /// Create a texture wrapper from a raw Assimp texture pointer
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid and that the texture
    /// will not be freed while this Texture instance exists.
    pub(crate) unsafe fn from_raw(texture_ptr: *const sys::aiTexture) -> Result<Self> {
        let texture_ptr =
            SharedPtr::new(texture_ptr).ok_or_else(|| Error::invalid_scene("Texture pointer is null"))?;

        Ok(Self {
            texture_ptr,
            _marker: PhantomData,
        })
    }

    /// Get the raw texture pointer
    pub fn as_raw(&self) -> *const sys::aiTexture {
        self.texture_ptr.as_ptr()
    }

    /// Get the width of the texture
    ///
    /// For uncompressed textures, this is the width in pixels.
    /// For compressed textures, this is the size of the compressed data in bytes.
    pub fn width(&self) -> u32 {
        unsafe { (*self.texture_ptr.as_ptr()).mWidth }
    }

    /// Get the height of the texture
    ///
    /// For uncompressed textures, this is the height in pixels.
    /// For compressed textures, this is 0.
    pub fn height(&self) -> u32 {
        unsafe { (*self.texture_ptr.as_ptr()).mHeight }
    }

    /// Check if this is a compressed texture
    pub fn is_compressed(&self) -> bool {
        self.height() == 0
    }

    /// Check if this is an uncompressed texture
    pub fn is_uncompressed(&self) -> bool {
        self.height() > 0
    }

    /// Get a borrowed view of the texture data (zero-copy).
    pub fn data_ref(&self) -> Result<TextureDataRef<'a>> {
        unsafe {
            let texture = &*self.texture_ptr.as_ptr();
            if texture.pcData.is_null() {
                return Err(Error::invalid_scene("Texture data is null"));
            }

            if self.is_compressed() {
                let size = self.width() as usize;
                let data_ptr = texture.pcData as *const u8;
                Ok(TextureDataRef::Compressed(std::slice::from_raw_parts(
                    data_ptr, size,
                )))
            } else {
                let size = (self.width() * self.height()) as usize;
                Ok(TextureDataRef::Texels(std::slice::from_raw_parts(
                    texture.pcData, size,
                )))
            }
        }
    }

    /// Get the format hint for the texture
    ///
    /// For uncompressed textures, this describes the channel layout (e.g., "rgba8888").
    /// For compressed textures, this is the file extension (e.g., "jpg", "png").
    pub fn format_hint(&self) -> String {
        unsafe {
            let hint = &(*self.texture_ptr.as_ptr()).achFormatHint;
            // Find the null terminator
            let len = hint.iter().position(|&c| c == 0).unwrap_or(hint.len());
            // Convert i8 to u8 for string conversion
            let hint_bytes: Vec<u8> = hint[..len].iter().map(|&c| c as u8).collect();
            String::from_utf8_lossy(&hint_bytes).to_string()
        }
    }

    /// Get the original filename of the texture
    pub fn filename(&self) -> Option<String> {
        unsafe {
            let ai_string = &(*self.texture_ptr.as_ptr()).mFilename;
            if ai_string.length == 0 {
                return None;
            }
            Some(ai_string_to_string(ai_string))
        }
    }

    /// Check if the texture format matches a given string
    ///
    /// This is useful for compressed textures to check the format.
    /// Example: `texture.check_format("jpg")` or `texture.check_format("png")`
    pub fn check_format(&self, format: &str) -> bool {
        if format.len() > 8 {
            return false;
        }

        let hint = self.format_hint();
        hint.eq_ignore_ascii_case(format)
    }

    /// Get the texture data
    pub fn data(&self) -> Result<TextureData> {
        match self.data_ref()? {
            TextureDataRef::Compressed(bytes) => Ok(TextureData::Compressed(bytes.to_vec())),
            TextureDataRef::Texels(texels) => Ok(TextureData::Texels(
                texels.iter().map(Texel::from).collect(),
            )),
        }
    }

    /// Get the size of the texture data in bytes
    pub fn data_size(&self) -> usize {
        if self.is_compressed() {
            self.width() as usize
        } else {
            (self.width() * self.height() * 4) as usize // 4 bytes per texel (ARGB)
        }
    }

    /// Get texture dimensions as a tuple (width, height)
    ///
    /// For compressed textures, height will be 0.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Save the texture data to a file
    ///
    /// For compressed textures, this saves the raw compressed data.
    /// For uncompressed textures, this would need additional image encoding.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let data = self.data()?;

        match data {
            TextureData::Compressed(bytes) => {
                std::fs::write(path, bytes)
                    .map_err(|e| Error::file_error(format!("Failed to save texture: {}", e)))?;
            }
            TextureData::Texels(_) => {
                return Err(Error::invalid_parameter(
                    "Saving uncompressed textures requires image encoding library".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Iterator over textures in a scene
pub struct TextureIterator<'a> {
    textures: Option<SharedPtr<*mut sys::aiTexture>>,
    count: usize,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> TextureIterator<'a> {
    /// Create a new texture iterator
    ///
    /// # Safety
    /// The caller must ensure that the textures pointer and count are valid.
    pub(crate) unsafe fn new(textures: *mut *mut sys::aiTexture, count: usize) -> Self {
        Self {
            textures: SharedPtr::new(textures as *const *mut sys::aiTexture),
            count,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for TextureIterator<'a> {
    type Item = Texture<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let textures = self.textures?;
        if self.index >= self.count {
            return None;
        }

        unsafe {
            let texture_ptr = *textures.as_ptr().add(self.index);
            self.index += 1;

            Texture::from_raw(texture_ptr).ok()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for TextureIterator<'a> {
    fn len(&self) -> usize {
        self.count.saturating_sub(self.index)
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
