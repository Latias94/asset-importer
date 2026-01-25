use std::ffi::CString;

use crate::{
    error::{Error, Result},
    importer::PropertyValue,
    sys,
    types::to_ai_matrix4x4,
};

pub(crate) struct BridgePropertyBuffers {
    pub(crate) ffi_props: Vec<sys::aiRustProperty>,
    _name_bufs: Vec<CString>,
    _value_str_bufs: Vec<CString>,
    _matrices: Vec<sys::aiMatrix4x4>,
}

pub(crate) fn build_rust_properties(
    props: &[(String, PropertyValue)],
) -> Result<BridgePropertyBuffers> {
    let mut ffi_props = Vec::with_capacity(props.len());
    let mut name_bufs: Vec<CString> = Vec::with_capacity(props.len());
    let mut value_str_bufs: Vec<CString> = Vec::new();
    let mut matrices: Vec<sys::aiMatrix4x4> = Vec::new();
    let mut matrix_ptr_fixes: Vec<(usize, usize)> = Vec::new();

    for (prop_index, (name, value)) in props.iter().enumerate() {
        let c_name = CString::new(name.as_str())
            .map_err(|_| Error::invalid_parameter("Invalid property name"))?;
        let mut p = sys::aiRustProperty {
            name: c_name.as_ptr(),
            kind: sys::aiRustPropertyKind::aiRustPropertyKind_Integer, // default, will set below
            int_value: 0,
            float_value: 0.0,
            string_value: std::ptr::null(),
            matrix_value: std::ptr::null_mut(),
        };

        match value {
            PropertyValue::Integer(v) => {
                p.kind = sys::aiRustPropertyKind::aiRustPropertyKind_Integer;
                p.int_value = *v;
            }
            PropertyValue::Boolean(v) => {
                p.kind = sys::aiRustPropertyKind::aiRustPropertyKind_Boolean;
                p.int_value = if *v { 1 } else { 0 };
            }
            PropertyValue::Float(v) => {
                p.kind = sys::aiRustPropertyKind::aiRustPropertyKind_Float;
                p.float_value = *v;
            }
            PropertyValue::String(s) => {
                p.kind = sys::aiRustPropertyKind::aiRustPropertyKind_String;
                let c_val = CString::new(s.as_str())
                    .map_err(|_| Error::invalid_parameter("Invalid property string value"))?;
                p.string_value = c_val.as_ptr();
                value_str_bufs.push(c_val);
            }
            PropertyValue::Matrix(m) => {
                p.kind = sys::aiRustPropertyKind::aiRustPropertyKind_Matrix4x4;
                matrices.push(to_ai_matrix4x4(*m));
                let matrix_index = matrices.len() - 1;
                matrix_ptr_fixes.push((prop_index, matrix_index));
            }
        }

        name_bufs.push(c_name);
        ffi_props.push(p);
    }

    // Patch matrix pointers after all matrix values are stored, so pointers are stable even if the
    // matrices Vec had to grow during collection.
    for (prop_index, matrix_index) in matrix_ptr_fixes {
        let matrix = matrices
            .get(matrix_index)
            .expect("matrix index should be in-bounds");
        let matrix_ptr = std::ptr::from_ref(matrix);
        let prop = ffi_props
            .get_mut(prop_index)
            .expect("prop index should be in-bounds");
        prop.matrix_value = matrix_ptr.cast::<std::ffi::c_void>().cast_mut();
    }

    Ok(BridgePropertyBuffers {
        ffi_props,
        _name_bufs: name_bufs,
        _value_str_bufs: value_str_bufs,
        _matrices: matrices,
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use super::*;

    #[test]
    fn property_buffers_keep_c_strings_alive() {
        let props = vec![
            ("a".to_string(), PropertyValue::Integer(7)),
            ("b".to_string(), PropertyValue::String("hello".to_string())),
            ("c".to_string(), PropertyValue::Boolean(true)),
        ];

        let buffers = build_rust_properties(&props).unwrap();

        let p0 = &buffers.ffi_props[0];
        assert_eq!(unsafe { CStr::from_ptr(p0.name) }.to_str().unwrap(), "a");
        assert_eq!(p0.int_value, 7);

        let p1 = &buffers.ffi_props[1];
        assert_eq!(unsafe { CStr::from_ptr(p1.name) }.to_str().unwrap(), "b");
        assert_eq!(
            unsafe { CStr::from_ptr(p1.string_value) }.to_str().unwrap(),
            "hello"
        );

        let p2 = &buffers.ffi_props[2];
        assert_eq!(unsafe { CStr::from_ptr(p2.name) }.to_str().unwrap(), "c");
        assert_eq!(p2.int_value, 1);
    }

    #[test]
    fn matrix_properties_point_to_stable_storage() {
        let m0 = crate::types::Matrix4x4::from_cols(
            crate::types::Vector4D::new(1.0, 2.0, 3.0, 4.0),
            crate::types::Vector4D::new(5.0, 6.0, 7.0, 8.0),
            crate::types::Vector4D::new(9.0, 10.0, 11.0, 12.0),
            crate::types::Vector4D::new(13.0, 14.0, 15.0, 16.0),
        );
        let m1 = crate::types::Matrix4x4::from_cols(
            crate::types::Vector4D::new(0.5, 0.25, 0.125, 0.0),
            crate::types::Vector4D::new(1.0, 0.0, 0.0, 0.0),
            crate::types::Vector4D::new(0.0, 1.0, 0.0, 0.0),
            crate::types::Vector4D::new(0.0, 0.0, 1.0, 0.0),
        );

        // Interleave matrices with other properties to exercise pointer patching order.
        let props = vec![
            ("m0".to_string(), PropertyValue::Matrix(m0)),
            ("i".to_string(), PropertyValue::Integer(42)),
            ("m1".to_string(), PropertyValue::Matrix(m1)),
        ];

        let buffers = build_rust_properties(&props).unwrap();

        let p0 = &buffers.ffi_props[0];
        assert_eq!(
            p0.kind,
            sys::aiRustPropertyKind::aiRustPropertyKind_Matrix4x4
        );
        assert!(!p0.matrix_value.is_null());
        let got0 = unsafe { *(p0.matrix_value as *const sys::aiMatrix4x4) };
        assert_eq!(got0, to_ai_matrix4x4(m0));

        let p2 = &buffers.ffi_props[2];
        assert_eq!(
            p2.kind,
            sys::aiRustPropertyKind::aiRustPropertyKind_Matrix4x4
        );
        assert!(!p2.matrix_value.is_null());
        let got1 = unsafe { *(p2.matrix_value as *const sys::aiMatrix4x4) };
        assert_eq!(got1, to_ai_matrix4x4(m1));
    }
}
