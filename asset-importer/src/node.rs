//! Scene node representation and hierarchy

use std::marker::PhantomData;

use crate::{
    error::Result,
    metadata::Metadata,
    ptr::SharedPtr,
    sys,
    types::{Matrix4x4, ai_string_to_str, ai_string_to_string, from_ai_matrix4x4},
};

/// A node in the scene hierarchy
#[derive(Clone, Copy)]
pub struct Node<'a> {
    node_ptr: SharedPtr<sys::aiNode>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Node<'a> {
    /// Create a Node from a raw Assimp node pointer
    ///
    /// # Safety
    /// Caller must ensure `node_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(node_ptr: *const sys::aiNode) -> Self {
        debug_assert!(!node_ptr.is_null());
        let node_ptr = unsafe { SharedPtr::new_unchecked(node_ptr) };
        Self {
            node_ptr,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiNode {
        self.node_ptr.as_ptr()
    }

    /// Get the raw node pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiNode {
        self.as_raw_sys()
    }

    /// Get the name of the node
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.node_ptr.as_ptr()).mName) }
    }

    /// Get the name of the node (zero-copy, lossy UTF-8).
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        unsafe { ai_string_to_str(&(*self.node_ptr.as_ptr()).mName) }
    }

    /// Get the transformation matrix of the node
    pub fn transformation(&self) -> Matrix4x4 {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            from_ai_matrix4x4(node.mTransformation)
        }
    }

    /// Get the parent node
    pub fn parent(&self) -> Option<Node<'a>> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mParent.is_null() {
                None
            } else {
                Some(Node::from_raw(node.mParent))
            }
        }
    }

    /// Get the number of child nodes
    pub fn num_children(&self) -> usize {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() {
                0
            } else {
                node.mNumChildren as usize
            }
        }
    }

    /// Get a child node by index
    pub fn child(&self, index: usize) -> Option<Node<'a>> {
        if index >= self.num_children() {
            return None;
        }

        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() {
                return None;
            }
            let child_ptr = *node.mChildren.add(index);
            if child_ptr.is_null() {
                None
            } else {
                Some(Node::from_raw(child_ptr))
            }
        }
    }

    /// Get an iterator over all child nodes
    pub fn children(&self) -> NodeIterator<'a> {
        NodeIterator {
            node_ptr: self.node_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Get the number of meshes attached to this node
    pub fn num_meshes(&self) -> usize {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mMeshes.is_null() {
                0
            } else {
                node.mNumMeshes as usize
            }
        }
    }

    /// Get a mesh index by index
    pub fn mesh_index(&self, index: usize) -> Option<usize> {
        if index >= self.num_meshes() {
            return None;
        }

        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mMeshes.is_null() {
                return None;
            }
            Some(*node.mMeshes.add(index) as usize)
        }
    }

    /// Get an iterator over all mesh indices
    pub fn mesh_indices(&self) -> MeshIndexIterator<'a> {
        MeshIndexIterator {
            node_ptr: self.node_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Get the raw mesh index array (zero-copy).
    pub fn mesh_indices_raw(&self) -> Option<&'a [u32]> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mMeshes.is_null() || node.mNumMeshes == 0 {
                None
            } else {
                Some(std::slice::from_raw_parts(
                    node.mMeshes,
                    node.mNumMeshes as usize,
                ))
            }
        }
    }

    /// Iterate mesh indices without allocation.
    pub fn mesh_indices_iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.mesh_indices_raw()
            .into_iter()
            .flat_map(|xs| xs.iter().map(|&x| x as usize))
    }

    /// Find a child node by name (recursive search)
    pub fn find_node(&self, name: &str) -> Option<Node<'a>> {
        if self.name_str().as_ref() == name {
            return Some(*self);
        }

        for child in self.children() {
            if let Some(found) = child.find_node(name) {
                return Some(found);
            }
        }

        None
    }
}

/// Iterator over child nodes
pub struct NodeIterator<'a> {
    node_ptr: SharedPtr<sys::aiNode>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() || node.mNumChildren == 0 {
                return None;
            }
            if self.index >= node.mNumChildren as usize {
                None
            } else {
                let child_ptr = *node.mChildren.add(self.index);
                self.index += 1;
                if child_ptr.is_null() {
                    None
                } else {
                    Some(Node::from_raw(child_ptr))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() {
                (0, Some(0))
            } else {
                let remaining = (node.mNumChildren as usize).saturating_sub(self.index);
                (remaining, Some(remaining))
            }
        }
    }
}

impl<'a> ExactSizeIterator for NodeIterator<'a> {}

/// Iterator over mesh indices in a node
pub struct MeshIndexIterator<'a> {
    node_ptr: SharedPtr<sys::aiNode>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for MeshIndexIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mMeshes.is_null() || node.mNumMeshes == 0 {
                return None;
            }
            if self.index >= node.mNumMeshes as usize {
                None
            } else {
                let mesh_index = *node.mMeshes.add(self.index) as usize;
                self.index += 1;
                Some(mesh_index)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mMeshes.is_null() {
                (0, Some(0))
            } else {
                let remaining = (node.mNumMeshes as usize).saturating_sub(self.index);
                (remaining, Some(remaining))
            }
        }
    }
}

impl<'a> ExactSizeIterator for MeshIndexIterator<'a> {}

impl<'a> Node<'a> {
    /// Get node metadata
    pub fn metadata(&self) -> Result<Metadata> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            Metadata::from_raw_sys(node.mMetaData)
        }
    }
}
