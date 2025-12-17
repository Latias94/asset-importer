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
    /// Create a Node from a raw Assimp node pointer
    ///
    /// # Safety
    /// Caller must ensure `node_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(scene: Scene, node_ptr: *const sys::aiNode) -> Self {
        debug_assert!(!node_ptr.is_null());
        let node_ptr = unsafe { SharedPtr::new_unchecked(node_ptr) };
        Self { scene, node_ptr }
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
    pub fn parent(&self) -> Option<Node> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mParent.is_null() {
                None
            } else {
                Some(Node::from_raw(self.scene.clone(), node.mParent))
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
    pub fn child(&self, index: usize) -> Option<Node> {
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
                Some(Node::from_raw(self.scene.clone(), child_ptr))
            }
        }
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
    pub fn mesh_indices(&self) -> MeshIndexIterator {
        MeshIndexIterator {
            scene: self.scene.clone(),
            node_ptr: self.node_ptr,
            index: 0,
        }
    }

    /// Get the raw mesh index array (zero-copy).
    pub fn mesh_indices_raw(&self) -> Option<&[u32]> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            ffi::slice_from_ptr_len_opt(self, node.mMeshes as *const u32, node.mNumMeshes as usize)
        }
    }

    /// Iterate mesh indices without allocation.
    pub fn mesh_indices_iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.mesh_indices_raw()
            .into_iter()
            .flat_map(|xs| xs.iter().map(|&x| x as usize))
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

impl Iterator for NodeIterator {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() || node.mNumChildren == 0 {
                return None;
            }
            while self.index < node.mNumChildren as usize {
                let child_ptr = *node.mChildren.add(self.index);
                self.index += 1;
                if child_ptr.is_null() {
                    continue;
                }
                return Some(Node::from_raw(self.scene.clone(), child_ptr));
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            if node.mChildren.is_null() {
                (0, Some(0))
            } else {
                let remaining = (node.mNumChildren as usize).saturating_sub(self.index);
                (0, Some(remaining))
            }
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

impl Iterator for MeshIndexIterator {
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

impl ExactSizeIterator for MeshIndexIterator {}

impl Node {
    /// Get node metadata
    pub fn metadata(&self) -> Result<Metadata> {
        unsafe {
            let node = &*self.node_ptr.as_ptr();
            Metadata::from_raw_sys(node.mMetaData)
        }
    }
}
