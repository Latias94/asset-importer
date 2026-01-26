//! Scene node representation and hierarchy

use crate::{
    error::Result,
    ffi,
    metadata::Metadata,
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::{Matrix4x4, ai_string_to_str, ai_string_to_string, from_ai_matrix4x4},
};

/// A node in the scene hierarchy
#[derive(Clone)]
pub struct Node {
    scene: Scene,
    node_ptr: SharedPtr<sys::aiNode>,
}

impl Node {
    pub(crate) fn from_sys_ptr(scene: Scene, node_ptr: *mut sys::aiNode) -> Option<Self> {
        let node_ptr = SharedPtr::new(node_ptr as *const sys::aiNode)?;
        Some(Self { scene, node_ptr })
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

    #[inline]
    fn raw(&self) -> &sys::aiNode {
        self.node_ptr.as_ref()
    }

    /// Get the name of the node
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the name of the node (zero-copy, lossy UTF-8).
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        ai_string_to_str(&self.raw().mName)
    }

    /// Get the transformation matrix of the node
    pub fn transformation(&self) -> Matrix4x4 {
        from_ai_matrix4x4(self.raw().mTransformation)
    }

    /// Get the parent node
    pub fn parent(&self) -> Option<Node> {
        let node = self.raw();
        Node::from_sys_ptr(self.scene.clone(), node.mParent)
    }

    /// Get the number of child nodes
    pub fn num_children(&self) -> usize {
        let node = self.raw();
        if node.mChildren.is_null() {
            0
        } else {
            node.mNumChildren as usize
        }
    }

    /// Get a child node by index
    pub fn child(&self, index: usize) -> Option<Node> {
        if index >= self.num_children() {
            return None;
        }

        let node = self.raw();
        let child_ptr =
            ffi::ptr_array_get(self, node.mChildren, node.mNumChildren as usize, index)?;
        Node::from_sys_ptr(self.scene.clone(), child_ptr)
    }

    /// Get an iterator over all child nodes
    pub fn children(&self) -> NodeIterator {
        NodeIterator {
            scene: self.scene.clone(),
            node_ptr: self.node_ptr,
            index: 0,
        }
    }

    /// Get the number of meshes attached to this node
    pub fn num_meshes(&self) -> usize {
        let node = self.raw();
        if node.mMeshes.is_null() {
            0
        } else {
            node.mNumMeshes as usize
        }
    }

    /// Get a mesh index by index
    pub fn mesh_index(&self, index: usize) -> Option<usize> {
        self.mesh_indices_raw().get(index).map(|&x| x as usize)
    }

    /// Get an iterator over all mesh indices
    pub fn mesh_indices(&self) -> MeshIndexIterator {
        MeshIndexIterator {
            scene: self.scene.clone(),
            node_ptr: self.node_ptr,
            index: 0,
        }
    }

    /// Get the raw mesh index array (zero-copy).
    pub fn mesh_indices_raw(&self) -> &[u32] {
        let node = self.raw();
        debug_assert!(node.mNumMeshes == 0 || !node.mMeshes.is_null());
        ffi::slice_from_ptr_len(self, node.mMeshes as *const u32, node.mNumMeshes as usize)
    }

    /// Get the raw mesh index array (zero-copy), returning `None` when absent.
    pub fn mesh_indices_raw_opt(&self) -> Option<&[u32]> {
        let node = self.raw();
        ffi::slice_from_ptr_len_opt(self, node.mMeshes as *const u32, node.mNumMeshes as usize)
    }

    /// Iterate mesh indices without allocation.
    pub fn mesh_indices_iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.mesh_indices_raw().iter().map(|&x| x as usize)
    }

    /// Find a child node by name (recursive search)
    pub fn find_node(&self, name: &str) -> Option<Node> {
        if self.name_str().as_ref() == name {
            return Some(self.clone());
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
pub struct NodeIterator {
    scene: Scene,
    node_ptr: SharedPtr<sys::aiNode>,
    index: usize,
}

impl NodeIterator {
    #[inline]
    fn node_ptr(&self) -> SharedPtr<sys::aiNode> {
        self.node_ptr
    }
}

impl Iterator for NodeIterator {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ptr = self.node_ptr();
        let node = node_ptr.as_ref();
        let children: &[*mut sys::aiNode] = ffi::slice_from_ptr_len_opt(
            node,
            node.mChildren as *const *mut sys::aiNode,
            node.mNumChildren as usize,
        )?;
        while self.index < children.len() {
            let index = self.index;
            self.index = index + 1;
            let child_ptr = children[index];
            if child_ptr.is_null() {
                continue;
            }
            return Node::from_sys_ptr(self.scene.clone(), child_ptr);
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let node_ptr = self.node_ptr();
        let node = node_ptr.as_ref();
        if node.mChildren.is_null() {
            (0, Some(0))
        } else {
            let remaining = (node.mNumChildren as usize).saturating_sub(self.index);
            (0, Some(remaining))
        }
    }
}

/// Iterator over mesh indices in a node
pub struct MeshIndexIterator {
    #[allow(dead_code)]
    scene: Scene,
    node_ptr: SharedPtr<sys::aiNode>,
    index: usize,
}

impl MeshIndexIterator {
    #[inline]
    fn node_ptr(&self) -> SharedPtr<sys::aiNode> {
        self.node_ptr
    }
}

impl Iterator for MeshIndexIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ptr = self.node_ptr();
        let node = node_ptr.as_ref();
        let indices: &[u32] = ffi::slice_from_ptr_len_opt(
            node,
            node.mMeshes as *const u32,
            node.mNumMeshes as usize,
        )?;
        if self.index >= indices.len() {
            return None;
        }
        let mesh_index = indices[self.index] as usize;
        self.index += 1;
        Some(mesh_index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let node_ptr = self.node_ptr();
        let node = node_ptr.as_ref();
        if node.mMeshes.is_null() {
            (0, Some(0))
        } else {
            let remaining = (node.mNumMeshes as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for MeshIndexIterator {}

impl Node {
    /// Get node metadata
    pub fn metadata(&self) -> Result<Metadata> {
        Metadata::from_sys_ptr(self.raw().mMetaData)
    }
}
