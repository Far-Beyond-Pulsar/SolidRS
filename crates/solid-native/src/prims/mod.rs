//! Asset prim types stored inside a [`SolidDocument`](crate::doc::SolidDocument).
//!
//! Each module defines the payload of one prim kind.  Prims reference each
//! other by stable string IDs (e.g. a [`MaterialAsset`]'s texture slots point
//! at `TexturePrim` IDs, a [`SkeletalMeshAsset`] points at a `SkeletonPrim`
//! ID) so assets can be stored and loaded individually while still binding
//! together.

pub mod animation;
pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod skeletal_mesh;
pub mod skeleton;
pub mod texture;

pub use animation::{
    AnimChannelAsset, AnimTargetAsset, AnimationAsset, BoneProperty,
};
pub use camera::CameraAsset;
pub use light::LightAsset;
pub use material::{MaterialAsset, TextureBinding};
pub use mesh::{MeshAsset, PrimitiveAsset};
pub use skeletal_mesh::SkeletalMeshAsset;
pub use skeleton::{Bone, SkeletonAsset};
pub use texture::TextureAsset;
