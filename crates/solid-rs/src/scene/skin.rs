//! Skeletal skin data for vertex skinning.

use glam::Mat4;

use crate::extensions::Extensions;
use crate::scene::node::NodeId;

/// Describes a skeleton for vertex skinning.
///
/// A skin maps joints (nodes in the scene graph) to vertex influences via
/// [`SkinWeights`](crate::geometry::SkinWeights) stored on each
/// [`Vertex`](crate::geometry::Vertex).
///
/// The inverse-bind matrix for joint `i` is the matrix that transforms
/// geometry from mesh space into joint-`i` space at bind pose.
#[derive(Debug, Clone)]
pub struct Skin {
    /// Human-readable name (e.g. `"ArmatureSkin"`).
    pub name: String,

    /// The scene node whose coordinate space is used as the root of the
    /// skeleton hierarchy. `None` if the format does not specify one.
    pub skeleton_root: Option<NodeId>,

    /// Ordered list of joint node IDs.  The index into this list corresponds
    /// to the joint indices in [`SkinWeights`](crate::geometry::SkinWeights).
    pub joints: Vec<NodeId>,

    /// Inverse bind-pose matrices, one per joint.
    /// If the vector is empty, assume identity for all joints.
    pub inverse_bind_matrices: Vec<Mat4>,

    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Skin {
    /// Creates an empty skin with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            skeleton_root: None,
            joints: Vec::new(),
            inverse_bind_matrices: Vec::new(),
            extensions: Extensions::new(),
        }
    }

    /// Returns the number of joints in this skin.
    #[inline]
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Returns the inverse bind matrix for joint `index`, or the identity
    /// matrix if the vector is absent or too short.
    pub fn inverse_bind_matrix(&self, index: usize) -> Mat4 {
        self.inverse_bind_matrices
            .get(index)
            .copied()
            .unwrap_or(Mat4::IDENTITY)
    }
}
