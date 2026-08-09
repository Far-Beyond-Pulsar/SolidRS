//! Mesh asset: a vertex buffer plus indexed draw calls and morph targets.

use solid_rs::geometry::{Aabb, Topology, Vertex};
use solid_rs::scene::{Mesh, MorphTarget};

/// One indexed draw call within a [`MeshAsset`].
///
/// Unlike [`solid_rs::geometry::Primitive`], the material is referenced by the
/// **prim ID** of a material prim rather than by an index into a scene
/// material array, so a mesh prim can be saved and loaded standalone.
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveAsset {
    /// How the indices are interpreted.
    pub topology: Topology,
    /// Indices into the owning mesh's vertex buffer.
    pub indices: Vec<u32>,
    /// Prim ID of the material prim to use, or `None`.
    pub material: Option<String>,
}

impl PrimitiveAsset {
    /// Creates a `TriangleList` primitive.
    pub fn triangles(indices: Vec<u32>, material: Option<String>) -> Self {
        Self {
            topology: Topology::TriangleList,
            indices,
            material,
        }
    }

    /// Creates a `LineList` primitive.
    pub fn lines(indices: Vec<u32>, material: Option<String>) -> Self {
        Self {
            topology: Topology::LineList,
            indices,
            material,
        }
    }

    /// Creates a `PointList` primitive.
    pub fn points(indices: Vec<u32>, material: Option<String>) -> Self {
        Self {
            topology: Topology::PointList,
            indices,
            material,
        }
    }
}

/// Static mesh geometry.
#[derive(Debug, Clone, Default)]
pub struct MeshAsset {
    /// Interleaved vertex buffer shared by all primitives.
    pub vertices: Vec<Vertex>,
    /// One or more indexed draw calls.
    pub primitives: Vec<PrimitiveAsset>,
    /// Blend shapes / shape keys.
    pub morph_targets: Vec<MorphTarget>,
    /// Initial blend weights (same length as `morph_targets`, or empty).
    pub morph_weights: Vec<f32>,
    /// Cached axis-aligned bounding box.
    pub bounds: Option<Aabb>,
}

impl MeshAsset {
    /// Creates an empty mesh asset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recomputes [`bounds`](MeshAsset::bounds) from vertex positions.
    pub fn compute_bounds(&mut self) {
        self.bounds = Aabb::from_points(self.vertices.iter().map(|v| v.position));
    }

    /// Number of unique vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Total number of indices across all primitives.
    pub fn total_indices(&self) -> usize {
        self.primitives.iter().map(|p| p.indices.len()).sum()
    }

    /// Returns `true` if the mesh has no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Converts a `solid-rs` mesh, dropping any material assignments
    /// (they become `None` — assign prim IDs after construction).
    pub fn from_mesh(mesh: &Mesh) -> Self {
        Self {
            vertices: mesh.vertices.clone(),
            primitives: mesh
                .primitives
                .iter()
                .map(|p| PrimitiveAsset {
                    topology: p.topology,
                    indices: p.indices.clone(),
                    material: None,
                })
                .collect(),
            morph_targets: mesh.morph_targets.clone(),
            morph_weights: mesh.morph_weights.clone(),
            bounds: mesh.bounds,
        }
    }

    /// Converts to a `solid-rs` mesh, resolving each primitive's material prim
    /// ID through `resolve` (returns `None` when the ID cannot be resolved).
    pub fn to_mesh(&self, resolve: &impl Fn(&str) -> Option<usize>) -> Mesh {
        let mut mesh = Mesh::new("");
        mesh.vertices = self.vertices.clone();
        mesh.primitives = self
            .primitives
            .iter()
            .map(|p| solid_rs::geometry::Primitive {
                topology: p.topology,
                indices: p.indices.clone(),
                material_index: p.material.as_deref().and_then(resolve),
            })
            .collect();
        mesh.morph_targets = self.morph_targets.clone();
        mesh.morph_weights = self.morph_weights.clone();
        mesh.bounds = self.bounds;
        mesh
    }
}
