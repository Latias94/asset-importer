//! Post-processing steps for imported scenes

use crate::sys;
use bitflags::bitflags;

bitflags! {
    /// Post-processing steps that can be applied to imported scenes
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PostProcessSteps: u32 {
        /// Calculates the tangents and bitangents for the imported meshes.
        const CALC_TANGENT_SPACE = sys::aiPostProcessSteps_aiProcess_CalcTangentSpace as u32;

        /// Identifies and joins identical vertex data sets within all imported meshes.
        const JOIN_IDENTICAL_VERTICES = sys::aiPostProcessSteps_aiProcess_JoinIdenticalVertices as u32;

        /// Converts all the imported data to a left-handed coordinate space.
        const MAKE_LEFT_HANDED = sys::aiPostProcessSteps_aiProcess_MakeLeftHanded as u32;

        /// Triangulates all faces of all meshes.
        const TRIANGULATE = sys::aiPostProcessSteps_aiProcess_Triangulate as u32;

        /// Removes some parts of the data structure (animations, materials, light sources, cameras, textures, vertex components).
        const REMOVE_COMPONENT = sys::aiPostProcessSteps_aiProcess_RemoveComponent as u32;

        /// Generates normals for all faces of all meshes.
        const GEN_NORMALS = sys::aiPostProcessSteps_aiProcess_GenNormals as u32;

        /// Generates smooth normals for all vertices in the mesh.
        const GEN_SMOOTH_NORMALS = sys::aiPostProcessSteps_aiProcess_GenSmoothNormals as u32;

        /// Splits large meshes into smaller sub-meshes.
        const SPLIT_LARGE_MESHES = sys::aiPostProcessSteps_aiProcess_SplitLargeMeshes as u32;

        /// Removes the node graph and pre-transforms all vertices with the local transformation matrices of their nodes.
        const PRE_TRANSFORM_VERTICES = sys::aiPostProcessSteps_aiProcess_PreTransformVertices as u32;

        /// Limits the number of bones simultaneously affecting a single vertex.
        const LIMIT_BONE_WEIGHTS = sys::aiPostProcessSteps_aiProcess_LimitBoneWeights as u32;

        /// Validates the imported scene data structure.
        const VALIDATE_DATA_STRUCTURE = sys::aiPostProcessSteps_aiProcess_ValidateDataStructure as u32;

        /// Reorders triangles for better vertex cache locality.
        const IMPROVE_CACHE_LOCALITY = sys::aiPostProcessSteps_aiProcess_ImproveCacheLocality as u32;

        /// Searches for redundant/unreferenced materials and removes them.
        const REMOVE_REDUNDANT_MATERIALS = sys::aiPostProcessSteps_aiProcess_RemoveRedundantMaterials as u32;

        /// This step tries to determine which meshes have normal vectors that are facing inwards and inverts them.
        const FIX_INFACING_NORMALS = sys::aiPostProcessSteps_aiProcess_FixInfacingNormals as u32;

        /// This step generically populates aiBone::mArmature and aiBone::mNode generically.
        const POPULATE_ARMATURE_DATA = sys::aiPostProcessSteps_aiProcess_PopulateArmatureData as u32;

        /// Sorts triangles by primitive type (points, lines, triangles).
        const SORT_BY_PTYPE = sys::aiPostProcessSteps_aiProcess_SortByPType as u32;

        /// Searches for duplicate vertices and removes them.
        const FIND_DEGENERATES = sys::aiPostProcessSteps_aiProcess_FindDegenerates as u32;

        /// Searches for invalid data, such as zeroed normal vectors or invalid UV coords and removes/fixes them.
        const FIND_INVALID_DATA = sys::aiPostProcessSteps_aiProcess_FindInvalidData as u32;

        /// Converts non-UV mappings (such as spherical or cylindrical mapping) to proper texture coordinate channels.
        const GEN_UV_COORDS = sys::aiPostProcessSteps_aiProcess_GenUVCoords as u32;

        /// Applies per-texture UV transformations and bakes them into stand-alone vtexture coordinate channels.
        const TRANSFORM_UV_COORDS = sys::aiPostProcessSteps_aiProcess_TransformUVCoords as u32;

        /// Searches for instances of meshes and replaces them by references to one master.
        const FIND_INSTANCES = sys::aiPostProcessSteps_aiProcess_FindInstances as u32;

        /// Optimizes the scene hierarchy.
        const OPTIMIZE_MESHES = sys::aiPostProcessSteps_aiProcess_OptimizeMeshes as u32;

        /// Optimizes the scene graph.
        const OPTIMIZE_GRAPH = sys::aiPostProcessSteps_aiProcess_OptimizeGraph as u32;

        /// Flips all UV coordinates along the y-axis and adjusts material settings and bitangents accordingly.
        const FLIP_UVS = sys::aiPostProcessSteps_aiProcess_FlipUVs as u32;

        /// Flips face winding order from CCW to CW or vice versa.
        const FLIP_WINDING_ORDER = sys::aiPostProcessSteps_aiProcess_FlipWindingOrder as u32;

        /// Splits meshes with more than one primitive type in homogeneous sub-meshes.
        const SPLIT_BY_BONE_COUNT = sys::aiPostProcessSteps_aiProcess_SplitByBoneCount as u32;

        /// Removes bones losslessly or according to some threshold.
        const DEBONE = sys::aiPostProcessSteps_aiProcess_Debone as u32;

        /// Converts absolute morphing animations into relative ones.
        const GLOBAL_SCALE = sys::aiPostProcessSteps_aiProcess_GlobalScale as u32;

        /// Embeds textures into the scene.
        const EMBED_TEXTURES = sys::aiPostProcessSteps_aiProcess_EmbedTextures as u32;

        /// Forces the loader to ignore up-direction.
        const FORCE_GEN_NORMALS = sys::aiPostProcessSteps_aiProcess_ForceGenNormals as u32;

        /// Drops normals for all faces of all meshes.
        const DROP_NORMALS = sys::aiPostProcessSteps_aiProcess_DropNormals as u32;

        /// Generates bounding boxes for all meshes.
        const GEN_BOUNDING_BOXES = sys::aiPostProcessSteps_aiProcess_GenBoundingBoxes as u32;
    }
}

impl PostProcessSteps {
    /// Get the raw value for use with the C API
    pub fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Create from raw value
    pub fn from_raw(value: u32) -> Self {
        Self::from_bits_truncate(value)
    }

    /// Validate that the post-processing flags are compatible
    ///
    /// Some post-processing steps are mutually exclusive and cannot be used together.
    /// This function checks for such conflicts and returns an error if any are found.
    pub fn validate(&self) -> Result<(), String> {
        // Check for mutually exclusive flags
        if self.contains(PostProcessSteps::GEN_SMOOTH_NORMALS)
            && self.contains(PostProcessSteps::GEN_NORMALS)
        {
            return Err("GEN_SMOOTH_NORMALS and GEN_NORMALS are incompatible".to_string());
        }

        if self.contains(PostProcessSteps::OPTIMIZE_GRAPH)
            && self.contains(PostProcessSteps::PRE_TRANSFORM_VERTICES)
        {
            return Err("OPTIMIZE_GRAPH and PRE_TRANSFORM_VERTICES are incompatible".to_string());
        }

        Ok(())
    }

    /// Check if the flags are valid (same as validate but returns bool)
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

impl Default for PostProcessSteps {
    fn default() -> Self {
        // Common default post-processing steps
        Self::TRIANGULATE | Self::JOIN_IDENTICAL_VERTICES | Self::SORT_BY_PTYPE
    }
}

/// Preset combinations of post-processing steps for common use cases
impl PostProcessSteps {
    /// Fast preset with basic optimizations
    pub const FAST: Self = Self::from_bits_truncate(
        Self::TRIANGULATE.bits()
            | Self::JOIN_IDENTICAL_VERTICES.bits()
            | Self::SORT_BY_PTYPE.bits(),
    );

    /// Quality preset with more thorough processing
    pub const QUALITY: Self = Self::from_bits_truncate(
        Self::TRIANGULATE.bits()
            | Self::JOIN_IDENTICAL_VERTICES.bits()
            | Self::SORT_BY_PTYPE.bits()
            | Self::GEN_SMOOTH_NORMALS.bits()
            | Self::IMPROVE_CACHE_LOCALITY.bits()
            | Self::REMOVE_REDUNDANT_MATERIALS.bits()
            | Self::FIX_INFACING_NORMALS.bits()
            | Self::FIND_DEGENERATES.bits()
            | Self::FIND_INVALID_DATA.bits(),
    );

    /// Maximum quality preset with all optimizations
    pub const MAX_QUALITY: Self = Self::from_bits_truncate(
        Self::QUALITY.bits()
            | Self::CALC_TANGENT_SPACE.bits()
            | Self::GEN_UV_COORDS.bits()
            | Self::OPTIMIZE_MESHES.bits()
            | Self::OPTIMIZE_GRAPH.bits()
            | Self::VALIDATE_DATA_STRUCTURE.bits(),
    );

    /// Real-time rendering preset
    pub const REALTIME: Self = Self::from_bits_truncate(
        Self::TRIANGULATE.bits()
            | Self::JOIN_IDENTICAL_VERTICES.bits()
            | Self::SORT_BY_PTYPE.bits()
            | Self::GEN_NORMALS.bits()
            | Self::IMPROVE_CACHE_LOCALITY.bits()
            | Self::LIMIT_BONE_WEIGHTS.bits()
            | Self::SPLIT_LARGE_MESHES.bits(),
    );

    /// Preset for left-handed coordinate systems (like DirectX)
    pub const TARGET_REALTIME_LEFT_HANDED: Self = Self::from_bits_truncate(
        Self::REALTIME.bits()
            | Self::MAKE_LEFT_HANDED.bits()
            | Self::FLIP_UVS.bits()
            | Self::FLIP_WINDING_ORDER.bits(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postprocess_flags() {
        let steps = PostProcessSteps::TRIANGULATE | PostProcessSteps::JOIN_IDENTICAL_VERTICES;
        assert!(steps.contains(PostProcessSteps::TRIANGULATE));
        assert!(steps.contains(PostProcessSteps::JOIN_IDENTICAL_VERTICES));
        assert!(!steps.contains(PostProcessSteps::GEN_NORMALS));
    }

    #[test]
    fn test_presets() {
        let fast = PostProcessSteps::FAST;
        assert!(fast.contains(PostProcessSteps::TRIANGULATE));

        let quality = PostProcessSteps::QUALITY;
        assert!(quality.contains(PostProcessSteps::TRIANGULATE));
        assert!(quality.contains(PostProcessSteps::GEN_SMOOTH_NORMALS));

        let max_quality = PostProcessSteps::MAX_QUALITY;
        assert!(max_quality.contains(PostProcessSteps::CALC_TANGENT_SPACE));
    }

    #[test]
    fn test_raw_conversion() {
        let steps = PostProcessSteps::TRIANGULATE;
        let raw = steps.as_raw();
        let converted_back = PostProcessSteps::from_raw(raw);
        assert_eq!(steps, converted_back);
    }

    #[test]
    fn test_flag_validation() {
        // Valid combinations
        let valid_steps = PostProcessSteps::TRIANGULATE | PostProcessSteps::JOIN_IDENTICAL_VERTICES;
        assert!(valid_steps.is_valid());
        assert!(valid_steps.validate().is_ok());

        // Invalid combination: GEN_SMOOTH_NORMALS and GEN_NORMALS
        let invalid_steps1 = PostProcessSteps::GEN_SMOOTH_NORMALS | PostProcessSteps::GEN_NORMALS;
        assert!(!invalid_steps1.is_valid());
        assert!(invalid_steps1.validate().is_err());

        // Invalid combination: OPTIMIZE_GRAPH and PRE_TRANSFORM_VERTICES
        let invalid_steps2 =
            PostProcessSteps::OPTIMIZE_GRAPH | PostProcessSteps::PRE_TRANSFORM_VERTICES;
        assert!(!invalid_steps2.is_valid());
        assert!(invalid_steps2.validate().is_err());

        // Test presets are valid
        assert!(PostProcessSteps::FAST.is_valid());
        assert!(PostProcessSteps::QUALITY.is_valid());
        assert!(PostProcessSteps::REALTIME.is_valid());
    }
}
