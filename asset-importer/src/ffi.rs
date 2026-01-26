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
/// # Assumptions
/// This crate assumes Assimp returns valid pointers/lengths for scene-backed data.
/// The returned slice is tied to `owner` so it cannot outlive the safe wrapper type.
pub(crate) fn slice_from_ptr_len<O: ?Sized, T>(owner: &O, ptr: *const T, len: usize) -> &[T] {
    let _ = owner;
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        // `from_raw_parts` requires proper alignment for `T`. Assimp should return aligned
        // pointers for its allocations, but reject unaligned pointers to avoid UB when a
        // corrupted/malicious scene reports bogus addresses.
        let align = std::mem::align_of::<T>();
        if align > 1 && (ptr as usize) % align != 0 {
            return &[];
        }

        // `from_raw_parts` requires `len * size_of::<T>() <= isize::MAX`.
        // Assimp scenes should satisfy this, but we defensively clamp to avoid UB
        // if a corrupted/malicious scene ever reports an insane length.
        let elem_size = std::mem::size_of::<T>();
        if elem_size != 0 && len > (isize::MAX as usize) / elem_size {
            &[]
        } else {
            // SAFETY: The crate assumes `ptr` is valid for `len` elements of `T` when sourced
            // from Assimp, and we defensively clamp insane lengths above.
            unsafe { std::slice::from_raw_parts(ptr, len) }
        }
    }
}

/// Borrow a slice from a raw pointer and element count, returning `None` when
/// `ptr` is null.
///
/// # Assumptions
/// Same as [`slice_from_ptr_len`].
pub(crate) fn slice_from_ptr_len_opt<O: ?Sized, T>(
    owner: &O,
    ptr: *const T,
    len: usize,
) -> Option<&[T]> {
    let _ = owner;
    if ptr.is_null() {
        return None;
    }
    Some(slice_from_ptr_len(owner, ptr, len))
}

/// Read a pointer element from a `T**`-style pointer array.
///
/// Returns `None` when the base pointer is null, the index is out-of-bounds, or
/// the element pointer at `index` is null.
///
/// # Assumptions
/// Same as [`slice_from_ptr_len`], plus `base` must be valid for `len` elements.
pub(crate) fn ptr_array_get<O: ?Sized, T>(
    owner: &O,
    base: *const *mut T,
    len: usize,
    index: usize,
) -> Option<*mut T> {
    let slice = slice_from_ptr_len_opt(owner, base, len)?;
    let ptr = *slice.get(index)?;
    if ptr.is_null() { None } else { Some(ptr) }
}

/// Borrow a reference from a raw pointer.
///
/// Returns `None` when `ptr` is null or not properly aligned for `T`.
///
/// # Assumptions
/// This crate assumes Assimp returns valid pointers for scene-backed data. This helper ties the
/// returned reference lifetime to `owner` so it cannot outlive the safe wrapper type.
pub(crate) fn ref_from_ptr<O: ?Sized, T>(owner: &O, ptr: *const T) -> Option<&T> {
    let _ = owner;
    if ptr.is_null() {
        return None;
    }

    let align = std::mem::align_of::<T>();
    if align > 1 && (ptr as usize) % align != 0 {
        return None;
    }

    // SAFETY: The crate assumes pointers returned by Assimp are valid for `T`.
    Some(unsafe { &*ptr })
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
        // `from_raw_parts_mut` requires proper alignment for `T`. Reject unaligned pointers to
        // avoid UB on corrupted/malicious inputs.
        let align = std::mem::align_of::<T>();
        if align > 1 && (ptr as usize) % align != 0 {
            return &mut [];
        }

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
        let p = std::ptr::dangling::<u8>();
        let mut_p = std::ptr::dangling_mut::<u8>();

        let owner = &();
        let s = slice_from_ptr_len(owner, p, too_large);
        assert!(s.is_empty());

        let mut owner = ();
        let s = unsafe { slice_from_mut_ptr_len(&mut owner, mut_p, too_large) };
        assert!(s.is_empty());
    }

    #[test]
    fn slice_from_ptr_len_opt_handles_null_and_zero_len() {
        let owner = &();
        let arr = [1u8, 2u8];

        let got = slice_from_ptr_len_opt(owner, arr.as_ptr(), 0).unwrap();
        assert!(got.is_empty());

        let got = slice_from_ptr_len_opt(owner, arr.as_ptr(), arr.len()).unwrap();
        assert_eq!(got, &arr);

        assert_eq!(
            slice_from_ptr_len_opt(owner, std::ptr::null::<u8>(), 0),
            None
        );
        assert_eq!(
            slice_from_ptr_len_opt(owner, std::ptr::null::<u8>(), 2),
            None
        );
    }

    #[test]
    fn slice_from_ptr_len_rejects_unaligned_pointers() {
        let owner = &();
        let buf = [0u32; 2];
        let unaligned = unsafe { (buf.as_ptr() as *const u8).add(1) } as *const u32;

        let got = slice_from_ptr_len(owner, unaligned, 1);
        assert!(got.is_empty());

        let got_opt = slice_from_ptr_len_opt(owner, unaligned, 1).unwrap();
        assert!(got_opt.is_empty());
    }

    #[test]
    fn slice_from_mut_ptr_len_rejects_unaligned_pointers() {
        let mut owner = ();
        let mut buf = [0u32; 2];
        let unaligned = unsafe { (buf.as_mut_ptr() as *mut u8).add(1) } as *mut u32;

        let got = unsafe { slice_from_mut_ptr_len(&mut owner, unaligned, 1) };
        assert!(got.is_empty());
    }

    #[test]
    fn ptr_array_get_handles_bounds_and_nulls() {
        let mut a = 1u32;
        let mut b = 2u32;
        let arr: [*mut u32; 3] = [&mut a, std::ptr::null_mut(), &mut b];

        let owner = &arr;
        assert_eq!(
            ptr_array_get(owner, arr.as_ptr(), arr.len(), 0),
            Some(&mut a as *mut u32)
        );
        assert_eq!(ptr_array_get(owner, arr.as_ptr(), arr.len(), 1), None);
        assert_eq!(
            ptr_array_get(owner, arr.as_ptr(), arr.len(), 2),
            Some(&mut b as *mut u32)
        );
        assert_eq!(ptr_array_get(owner, arr.as_ptr(), arr.len(), 3), None);

        // Null base pointer should yield None even if len/index are non-zero.
        assert_eq!(
            ptr_array_get(owner, std::ptr::null::<*mut u32>(), 10, 0),
            None
        );
    }

    #[test]
    fn ref_from_ptr_rejects_null_and_unaligned_pointers() {
        let owner = &();
        assert!(ref_from_ptr::<_, u32>(owner, std::ptr::null()).is_none());

        let buf = [0u32; 2];
        let unaligned = unsafe { (buf.as_ptr() as *const u8).add(1) } as *const u32;
        assert!(ref_from_ptr(owner, unaligned).is_none());

        assert!(ref_from_ptr(owner, buf.as_ptr()).is_some());
    }
}
