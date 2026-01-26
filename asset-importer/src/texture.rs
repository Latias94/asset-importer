//! Texture and embedded texture support
//!
//! This module provides safe Rust wrappers for Assimp's texture functionality,
//! including support for embedded textures that are stored directly within
//! model files.

use crate::types::ai_string_to_string;
use crate::{
    error::{Error, Result},
    ffi,
    ptr::SharedPtr,
    scene::Scene,
    sys,
};
use std::borrow::Cow;

/// A texel (texture element) in ARGB8888 format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
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

#[cfg(feature = "raw-sys")]
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

#[cfg(test)]
mod layout_tests {
    use super::Texel;
    use crate::sys;

    #[test]
    fn test_texel_layout_matches_sys() {
        assert_eq!(
            std::mem::size_of::<Texel>(),
            std::mem::size_of::<sys::aiTexel>()
        );
        assert_eq!(
            std::mem::align_of::<Texel>(),
            std::mem::align_of::<sys::aiTexel>()
        );
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
    Texels(&'a [Texel]),
    /// Compressed raw bytes (when height == 0)
    Compressed(&'a [u8]),
}

/// An embedded texture within a 3D model file
///
/// Textures can be either:
/// 1. Uncompressed - stored as raw ARGB8888 texel data
/// 2. Compressed - stored in a standard format like PNG, JPEG, etc.
#[derive(Debug, Clone)]
pub struct Texture {
    #[allow(dead_code)]
    scene: Scene,
    texture_ptr: SharedPtr<sys::aiTexture>,
}

impl Texture {
    pub(crate) fn from_sys_ptr(scene: Scene, texture_ptr: *const sys::aiTexture) -> Result<Self> {
        let texture_ptr = SharedPtr::new(texture_ptr)
            .ok_or_else(|| Error::invalid_scene("Texture pointer is null"))?;
        Ok(Self { scene, texture_ptr })
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiTexture {
        self.texture_ptr.as_ptr()
    }

    /// Get the raw texture pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiTexture {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiTexture {
        self.texture_ptr.as_ref()
    }

    /// Get the width of the texture
    ///
    /// For uncompressed textures, this is the width in pixels.
    /// For compressed textures, this is the size of the compressed data in bytes.
    pub fn width(&self) -> u32 {
        self.raw().mWidth
    }

    /// Get the height of the texture
    ///
    /// For uncompressed textures, this is the height in pixels.
    /// For compressed textures, this is 0.
    pub fn height(&self) -> u32 {
        self.raw().mHeight
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
    pub fn data_ref(&self) -> Result<TextureDataRef<'_>> {
        let texture = self.raw();
        if self.is_compressed() {
            let size = self.width() as usize;
            if size == 0 {
                return Ok(TextureDataRef::Compressed(&[]));
            }
            let data_ptr = texture.pcData as *const u8;
            let Some(bytes) = ffi::slice_from_ptr_len_opt(self, data_ptr, size) else {
                return Err(Error::invalid_scene("Texture data is null"));
            };
            Ok(TextureDataRef::Compressed(bytes))
        } else {
            let width = self.width() as usize;
            let height = self.height() as usize;
            let Some(size) = width.checked_mul(height) else {
                return Err(Error::invalid_scene("Texture dimensions overflow"));
            };
            if size == 0 {
                return Ok(TextureDataRef::Texels(&[]));
            }
            let Some(texels) =
                ffi::slice_from_ptr_len_opt(self, texture.pcData as *const Texel, size)
            else {
                return Err(Error::invalid_scene("Texture data is null"));
            };
            Ok(TextureDataRef::Texels(texels))
        }
    }

    /// Get a borrowed view of the texture data as raw bytes (zero-copy).
    ///
    /// - Compressed textures return the compressed byte payload.
    /// - Uncompressed textures return the in-memory texel bytes (ARGB8888).
    #[cfg(feature = "bytemuck")]
    pub fn data_bytes_ref(&self) -> Result<&[u8]> {
        match self.data_ref()? {
            TextureDataRef::Compressed(bytes) => Ok(bytes),
            TextureDataRef::Texels(texels) => Ok(bytemuck::cast_slice(texels)),
        }
    }

    /// Get the format hint for the texture
    ///
    /// For uncompressed textures, this describes the channel layout (e.g., "rgba8888").
    /// For compressed textures, this is the file extension (e.g., "jpg", "png").
    pub fn format_hint_bytes(&self) -> &[u8] {
        let hint = &self.raw().achFormatHint;
        // Find the null terminator
        let len = hint.iter().position(|&c| c == 0).unwrap_or(hint.len());
        ffi::slice_from_ptr_len(self, hint.as_ptr() as *const u8, len)
    }

    /// Get the format hint as UTF-8 (lossy) without allocation.
    pub fn format_hint_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.format_hint_bytes())
    }

    /// Get the format hint for the texture (allocates).
    ///
    /// Prefer [`Texture::format_hint_str`] for a zero-allocation option.
    pub fn format_hint(&self) -> String {
        self.format_hint_str().into_owned()
    }

    /// Get the original filename of the texture
    pub fn filename(&self) -> Option<String> {
        let ai_string = &self.raw().mFilename;
        if ai_string.length == 0 {
            return None;
        }
        Some(ai_string_to_string(ai_string))
    }

    /// Get the original filename of the texture as UTF-8 (lossy) without allocation.
    pub fn filename_str(&self) -> Option<Cow<'_, str>> {
        let ai_string = &self.raw().mFilename;
        (ai_string.length != 0).then(|| crate::types::ai_string_to_str(ai_string))
    }

    /// Check if the texture format matches a given string
    ///
    /// This is useful for compressed textures to check the format.
    /// Example: `texture.check_format("jpg")` or `texture.check_format("png")`
    pub fn check_format(&self, format: &str) -> bool {
        if format.len() > 8 {
            return false;
        }

        self.format_hint_str().as_ref().eq_ignore_ascii_case(format)
    }

    /// Get the texture data
    pub fn data(&self) -> Result<TextureData> {
        match self.data_ref()? {
            TextureDataRef::Compressed(bytes) => Ok(TextureData::Compressed(bytes.to_vec())),
            TextureDataRef::Texels(texels) => Ok(TextureData::Texels(texels.to_vec())),
        }
    }

    /// Get the size of the texture data in bytes
    pub fn data_size(&self) -> usize {
        if self.is_compressed() {
            self.width() as usize
        } else {
            let w = self.width() as u64;
            let h = self.height() as u64;
            w.saturating_mul(h).saturating_mul(4) as usize // 4 bytes per texel (ARGB)
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
pub struct TextureIterator {
    scene: Scene,
    textures: Option<SharedPtr<*const sys::aiTexture>>,
    count: usize,
    index: usize,
}

impl TextureIterator {
    /// Create a new texture iterator
    pub(crate) fn new(scene: Scene, textures: *mut *mut sys::aiTexture, count: usize) -> Self {
        let textures_ptr = SharedPtr::new(textures as *const *const sys::aiTexture);
        Self {
            scene,
            textures: textures_ptr,
            count: if textures_ptr.is_some() { count } else { 0 },
            index: 0,
        }
    }
}

impl Iterator for TextureIterator {
    type Item = Texture;

    fn next(&mut self) -> Option<Self::Item> {
        let textures = self.textures?;
        let slice = crate::ffi::slice_from_ptr_len_opt(&(), textures.as_ptr(), self.count)?;
        while self.index < slice.len() {
            let texture_ptr = slice[self.index];
            self.index += 1;
            if texture_ptr.is_null() {
                continue;
            }
            // `from_sys_ptr` only fails on null pointers; keep the iterator robust anyway.
            if let Ok(tex) = Texture::from_sys_ptr(self.scene.clone(), texture_ptr) {
                return Some(tex);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (0, Some(remaining))
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
