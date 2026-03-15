//! Geometry primitives: vertices, faces, transforms, and bounding volumes.

pub mod bounds;
pub mod face;
pub mod transform;
pub mod vertex;

pub use bounds::Aabb;
pub use face::{Primitive, Topology};
pub use transform::Transform;
pub use vertex::{SkinWeights, Vertex, MAX_COLOR_CHANNELS, MAX_UV_CHANNELS};
