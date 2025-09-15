//! Scene node representation and hierarchy

use crate::{
    error::{c_str_to_string_or_empty, Result},
    metadata::Metadata,
    sys,
    types::{from_ai_matrix4x4, Matrix4x4},
};

/// A node in the scene hierarchy
pub struct Node {
    node_ptr: *const sys::aiNode,
}

impl Node {
    /// Create a Node from a raw Assimp node pointer
    pub(crate) fn from_raw(node_ptr: *const sys::aiNode) -> Self {
        Self { node_ptr }
    }

    /// Get the raw node pointer
    pub fn as_raw(&self) -> *const sys::aiNode {
        self.node_ptr
    }

    /// Get the name of the node
    pub fn name(&self) -> String {
        unsafe {
            let node = &*self.node_ptr;
            c_str_to_string_or_empty(node.mName.data.as_ptr() as *const i8)
        }
    }

    /// Get the transformation matrix of the node
    pub fn transformation(&self) -> Matrix4x4 {
        unsafe {
            let node = &*self.node_ptr;
            from_ai_matrix4x4(node.mTransformation)
        }
    }

    /// Get the parent node
    pub fn parent(&self) -> Option<Node> {
        unsafe {
            let node = &*self.node_ptr;
            if node.mParent.is_null() {
                None
            } else {
                Some(Node::from_raw(node.mParent))
            }
        }
    }

    /// Get the number of child nodes
    pub fn num_children(&self) -> usize {
        unsafe { (*self.node_ptr).mNumChildren as usize }
    }

    /// Get a child node by index
    pub fn child(&self, index: usize) -> Option<Node> {
        if index >= self.num_children() {
            return None;
        }

        unsafe {
            let node = &*self.node_ptr;
            let child_ptr = *node.mChildren.add(index);
            if child_ptr.is_null() {
                None
            } else {
                Some(Node::from_raw(child_ptr))
            }
        }
    }

    /// Get an iterator over all child nodes
    pub fn children(&self) -> NodeIterator {
        NodeIterator {
            node_ptr: self.node_ptr,
            index: 0,
        }
    }

    /// Get the number of meshes attached to this node
    pub fn num_meshes(&self) -> usize {
        unsafe { (*self.node_ptr).mNumMeshes as usize }
    }

    /// Get a mesh index by index
    pub fn mesh_index(&self, index: usize) -> Option<usize> {
        if index >= self.num_meshes() {
            return None;
        }

        unsafe {
            let node = &*self.node_ptr;
            Some(*node.mMeshes.add(index) as usize)
        }
    }

    /// Get an iterator over all mesh indices
    pub fn mesh_indices(&self) -> MeshIndexIterator {
        MeshIndexIterator {
            node_ptr: self.node_ptr,
            index: 0,
        }
    }

    /// Find a child node by name (recursive search)
    pub fn find_node(&self, name: &str) -> Option<Node> {
        if self.name() == name {
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

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            node_ptr: self.node_ptr,
        }
    }
}

impl Copy for Node {}

/// Iterator over child nodes
pub struct NodeIterator {
    node_ptr: *const sys::aiNode,
    index: usize,
}

impl Iterator for NodeIterator {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let node = &*self.node_ptr;
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
            let node = &*self.node_ptr;
            let remaining = (node.mNumChildren as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for NodeIterator {}

/// Iterator over mesh indices in a node
pub struct MeshIndexIterator {
    node_ptr: *const sys::aiNode,
    index: usize,
}

impl Iterator for MeshIndexIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let node = &*self.node_ptr;
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
            let node = &*self.node_ptr;
            let remaining = (node.mNumMeshes as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for MeshIndexIterator {}

impl Node {
    /// Get node metadata
    pub fn metadata(&self) -> Result<Metadata> {
        unsafe {
            let node = &*self.node_ptr;
            Metadata::from_raw(node.mMetaData)
        }
    }
}
