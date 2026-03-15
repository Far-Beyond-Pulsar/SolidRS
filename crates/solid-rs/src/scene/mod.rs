//! The scene graph and all scene-object types.

pub mod animation;
pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod node;
pub mod scene;
pub mod skin;
pub mod texture;

pub use animation::{Animation, AnimationChannel, AnimationTarget, Interpolation};
pub use camera::{Camera, OrthographicCamera, PerspectiveCamera, Projection};
pub use light::{AreaLight, DirectionalLight, Light, LightBase, PointLight, SpotLight};
pub use material::{AlphaMode, Material, TextureRef, TextureTransform};
pub use mesh::{Mesh, MorphTarget};
pub use node::{Node, NodeId};
pub use scene::{Metadata, Scene};
pub use skin::Skin;
pub use texture::{FilterMode, Image, ImageSource, Sampler, Texture, WrapMode};
