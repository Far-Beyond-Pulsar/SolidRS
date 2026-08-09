//! Skeletal mesh asset: a skinned mesh bound to a skeleton prim.

use glam::Mat4;

use crate::prims::mesh::MeshAsset;

/// A mesh whose vertices carry [`solid_rs::geometry::SkinWeights`], bound to a
/// [`SkeletonAsset`](crate::prims::SkeletonAsset) prim by bone name.
#[derive(Debug, Clone, Default)]
pub struct SkeletalMeshAsset {
    /// Geometry; vertex `SkinWeights.joints` index into [`bones`](SkeletalMeshAsset::bones).
    pub mesh: MeshAsset,
    /// Bone names referenced by vertex weights.  `bones[i]` is the joint
    /// `i` stored in a vertex's skin weights.
    pub bones: Vec<String>,
    /// Prim ID of the [`SkeletonAsset`](crate::prims::SkeletonAsset) this
    /// mesh is skinned to.  Bone names must match skeleton bone names.
    pub skeleton: Option<String>,
    /// Inverse bind-pose matrices, one per bone in [`bones`](SkeletalMeshAsset::bones).
    /// Empty falls back to the skeleton's own matrices when resolving.
    pub inverse_bind_matrices: Vec<Mat4>,
}

impl SkeletalMeshAsset {
    /// Creates an empty skeletal mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a skeletal mesh from a `solid-rs` mesh (materials dropped) and
    /// a list of bone names.
    pub fn from_mesh(mesh: &solid_rs::scene::Mesh, bones: Vec<String>) -> Self {
        Self {
            mesh: MeshAsset::from_mesh(mesh),
            bones,
            skeleton: None,
            inverse_bind_matrices: Vec::new(),
        }
    }

    /// Recomputes the mesh bounding box from vertex positions.
    pub fn compute_bounds(&mut self) {
        self.mesh.compute_bounds();
    }

    /// Appends a bone name and returns its joint index.
    pub fn push_bone(&mut self, name: impl Into<String>) -> usize {
        self.bones.push(name.into());
        self.bones.len() - 1
    }

    /// Number of bones referenced by this skeletal mesh.
    pub fn joint_count(&self) -> usize {
        self.bones.len()
    }
}
