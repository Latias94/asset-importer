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
    let matrix_count = props
        .iter()
        .filter(|(_, v)| matches!(v, PropertyValue::Matrix(_)))
        .count();

    let mut ffi_props = Vec::with_capacity(props.len());
    let mut name_bufs: Vec<CString> = Vec::with_capacity(props.len());
    let mut value_str_bufs: Vec<CString> = Vec::new();
    let mut matrices: Vec<sys::aiMatrix4x4> = Vec::with_capacity(matrix_count);

    for (name, value) in props {
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
                let idx = matrices.len() - 1;
                let matrix_ptr = unsafe { matrices.as_ptr().add(idx) };
                p.matrix_value = matrix_ptr.cast::<std::ffi::c_void>().cast_mut();
            }
        }

        name_bufs.push(c_name);
        ffi_props.push(p);
    }

    Ok(BridgePropertyBuffers {
        ffi_props,
        _name_bufs: name_bufs,
        _value_str_bufs: value_str_bufs,
        _matrices: matrices,
    })
}
