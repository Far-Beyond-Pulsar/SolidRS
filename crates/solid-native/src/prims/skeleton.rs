//! Skeleton asset: a bone hierarchy with inverse bind matrices.

use glam::Mat4;

use solid_rs::geometry::Transform;

/// A single bone in a [`SkeletonAsset`].
///
/// `parent` is an index into the owning skeleton's `bones` list, so bones must
/// be listed parent-before-child (topological order).
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// Bone name.
    pub name: String,
    /// Index of the parent bone in the owning skeleton, or `None` for roots.
    pub parent: Option<usize>,
    /// Local transform relative to the parent bone.
    pub local_transform: Transform,
}

impl Bone {
    /// Creates a root bone with an identity local transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            local_transform: Transform::IDENTITY,
        }
    }

    /// Creates a bone whose parent is `parent`.
    pub fn child_of(name: impl Into<String>, parent: usize) -> Self {
        Self {
            name: name.into(),
            parent: Some(parent),
            local_transform: Transform::IDENTITY,
        }
    }
}

/// A joint hierarchy ready for vertex skinning.
#[derive(Debug, Clone, Default)]
pub struct SkeletonAsset {
    /// Bones in parent-before-child order.  The index of a bone is the joint
    /// index used by [`SkeletalMeshAsset`](crate::prims::SkeletalMeshAsset)
    /// vertex weights.
    pub bones: Vec<Bone>,
    /// Inverse bind-pose matrices, one per bone.  Empty means identity.
    pub inverse_bind_matrices: Vec<Mat4>,
}

impl SkeletonAsset {
    /// Creates an empty skeleton.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a bone and returns its joint index.
    pub fn push_bone(&mut self, bone: Bone) -> usize {
        self.bones.push(bone);
        self.bones.len() - 1
    }

    /// Returns the joint index of the bone with `name`, or `None`.
    pub fn bone_index(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }

    /// Number of bones.
    pub fn joint_count(&self) -> usize {
        self.bones.len()
    }
}
