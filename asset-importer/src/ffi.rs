//! Internal FFI helpers.
//!
//! These helpers centralize common pointer/length checks when borrowing slices from
//! Assimp-owned memory. They intentionally tie the returned slice lifetime to an
//! "owner" reference (usually `&self`) so callers cannot accidentally fabricate a
//! longer lifetime.

/// Borrow a slice from a raw pointer and element count.
///
/// Returns an empty slice when `ptr` is null or `len == 0`.
///
/// # Safety
/// Callers must ensure the memory behind `ptr` is valid for `len` elements of `T`
/// for at least as long as `owner` is alive.
pub(crate) unsafe fn slice_from_ptr_len<O: ?Sized, T>(
    owner: &O,
    ptr: *const T,
    len: usize,
) -> &[T] {
    let _ = owner;
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        // `from_raw_parts` requires `len * size_of::<T>() <= isize::MAX`.
        // Assimp scenes should satisfy this, but we defensively clamp to avoid UB
        // if a corrupted/malicious scene ever reports an insane length.
        let elem_size = std::mem::size_of::<T>();
        if elem_size != 0 && len > (isize::MAX as usize) / elem_size {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }
}

/// Borrow a slice from a raw pointer and element count, returning `None` when
/// `ptr` is null.
///
/// # Safety
/// Same as [`slice_from_ptr_len`].
pub(crate) unsafe fn slice_from_ptr_len_opt<O: ?Sized, T>(
    owner: &O,
    ptr: *const T,
    len: usize,
) -> Option<&[T]> {
    let _ = owner;
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { slice_from_ptr_len(owner, ptr, len) })
}

/// Mutably borrow a slice from a raw pointer and element count.
///
/// Returns an empty slice when `ptr` is null or `len == 0`.
///
/// # Safety
/// Callers must ensure the memory behind `ptr` is valid for `len` elements of `T`
/// for at least as long as `owner` is alive, and that no other references alias
/// this region while the returned slice is in use.
pub(crate) unsafe fn slice_from_mut_ptr_len<O: ?Sized, T>(
    owner: &mut O,
    ptr: *mut T,
    len: usize,
) -> &mut [T] {
    let _ = owner;
    if ptr.is_null() || len == 0 {
        &mut []
    } else {
        // `from_raw_parts_mut` requires `len * size_of::<T>() <= isize::MAX`.
        // As with immutable slices, clamp to avoid UB if a corrupted/malicious
        // scene ever reports an insane length.
        let elem_size = std::mem::size_of::<T>();
        if elem_size != 0 && len > (isize::MAX as usize) / elem_size {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(ptr, len) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_helpers_return_empty_on_insane_lengths() {
        let too_large = (isize::MAX as usize) + 1;

        // These pointers are intentionally invalid, but the helpers must not dereference them when
        // the length is clearly impossible for `from_raw_parts{,_mut}`.
        let p = 1usize as *const u8;
        let mut_p = 1usize as *mut u8;

        let owner = &();
        let s = unsafe { slice_from_ptr_len(owner, p, too_large) };
        assert!(s.is_empty());

        let mut owner = ();
        let s = unsafe { slice_from_mut_ptr_len(&mut owner, mut_p, too_large) };
        assert!(s.is_empty());
    }
}
