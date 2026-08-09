//! A [`Prim`]: one individually-addressable 3D asset in a [`SolidDocument`].

use std::collections::HashMap;

use solid_rs::scene::Scene;
use solid_rs::value::Value;

use crate::doc::Props;
use crate::prims::{
    AnimationAsset, CameraAsset, LightAsset, MaterialAsset, MeshAsset, SkeletalMeshAsset,
    SkeletonAsset, TextureAsset,
};

/// Discriminates the payload of a [`Prim`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimKind {
    /// Static geometry ([`MeshAsset`]).
    Mesh,
    /// Bone hierarchy ([`SkeletonAsset`]).
    Skeleton,
    /// Skinned mesh ([`SkeletalMeshAsset`]).
    SkeletalMesh,
    /// PBR material ([`MaterialAsset`]).
    Material,
    /// Image + sampler ([`TextureAsset`]).
    Texture,
    /// Keyframe clip ([`AnimationAsset`]).
    Animation,
    /// Camera projection ([`CameraAsset`]).
    Camera,
    /// Light source ([`LightAsset`]).
    Light,
    /// Complete scene graph ([`Scene`]).
    Scene,
}

impl PrimKind {
    /// Serialised kind name (used as the `"kind"` field on disk).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mesh => "mesh",
            Self::Skeleton => "skeleton",
            Self::SkeletalMesh => "skeletal_mesh",
            Self::Material => "material",
            Self::Texture => "texture",
            Self::Animation => "animation",
            Self::Camera => "camera",
            Self::Light => "light",
            Self::Scene => "scene",
        }
    }

    /// Parses a serialised kind name.
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "mesh" => Self::Mesh,
            "skeleton" => Self::Skeleton,
            "skeletal_mesh" => Self::SkeletalMesh,
            "material" => Self::Material,
            "texture" => Self::Texture,
            "animation" => Self::Animation,
            "camera" => Self::Camera,
            "light" => Self::Light,
            "scene" => Self::Scene,
            _ => return None,
        })
    }
}

/// The payload of a [`Prim`].
#[derive(Debug, Clone)]
pub enum PrimData {
    /// Static geometry.
    Mesh(MeshAsset),
    /// Bone hierarchy.
    Skeleton(SkeletonAsset),
    /// Skinned mesh bound to a skeleton.
    SkeletalMesh(SkeletalMeshAsset),
    /// PBR material.
    Material(MaterialAsset),
    /// Image + sampler.
    Texture(TextureAsset),
    /// Keyframe clip.
    Animation(AnimationAsset),
    /// Camera projection.
    Camera(CameraAsset),
    /// Light source.
    Light(LightAsset),
    /// Complete `solid-rs` scene graph.
    Scene(Scene),
}

impl PrimData {
    /// The kind of this prim.
    pub fn kind(&self) -> PrimKind {
        match self {
            Self::Mesh(_) => PrimKind::Mesh,
            Self::Skeleton(_) => PrimKind::Skeleton,
            Self::SkeletalMesh(_) => PrimKind::SkeletalMesh,
            Self::Material(_) => PrimKind::Material,
            Self::Texture(_) => PrimKind::Texture,
            Self::Animation(_) => PrimKind::Animation,
            Self::Camera(_) => PrimKind::Camera,
            Self::Light(_) => PrimKind::Light,
            Self::Scene(_) => PrimKind::Scene,
        }
    }
}

/// One individually-addressable 3D asset inside a [`SolidDocument`].
///
/// Prims are referenced by their stable string [`id`](Prim::id), which is how
/// assets bind to each other (a material references a texture prim's ID, a
/// skeletal mesh references its skeleton's ID, …).
#[derive(Debug, Clone)]
pub struct Prim {
    /// Stable, unique identifier used for cross-prim references.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Arbitrary per-prim key/value properties.
    pub props: Props,
    /// The prim payload.
    pub data: PrimData,
}

impl Prim {
    /// Creates a prim with the given ID, name and payload.
    pub fn new(id: impl Into<String>, name: impl Into<String>, data: PrimData) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            props: Props::new(),
            data,
        }
    }

    /// The kind of this prim.
    pub fn kind(&self) -> PrimKind {
        self.data.kind()
    }

    /// Sets an arbitrary property, returning `&mut self` for chaining.
    pub fn with_prop(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.props.insert(key.into(), value.into());
        self
    }

    /// Returns the value of a property, or `None`.
    pub fn prop(&self, key: &str) -> Option<&Value> {
        self.props.get(key)
    }

    /// Creates a `mesh` prim.
    pub fn mesh(id: impl Into<String>, name: impl Into<String>, mesh: MeshAsset) -> Self {
        Self::new(id, name, PrimData::Mesh(mesh))
    }

    /// Creates a `skeleton` prim.
    pub fn skeleton(id: impl Into<String>, name: impl Into<String>, skel: SkeletonAsset) -> Self {
        Self::new(id, name, PrimData::Skeleton(skel))
    }

    /// Creates a `skeletal_mesh` prim.
    pub fn skeletal_mesh(
        id: impl Into<String>,
        name: impl Into<String>,
        skel_mesh: SkeletalMeshAsset,
    ) -> Self {
        Self::new(id, name, PrimData::SkeletalMesh(skel_mesh))
    }

    /// Creates a `material` prim.
    pub fn material(id: impl Into<String>, name: impl Into<String>, mat: MaterialAsset) -> Self {
        Self::new(id, name, PrimData::Material(mat))
    }

    /// Creates a `texture` prim.
    pub fn texture(id: impl Into<String>, name: impl Into<String>, tex: TextureAsset) -> Self {
        Self::new(id, name, PrimData::Texture(tex))
    }

    /// Creates an `animation` prim.
    pub fn animation(id: impl Into<String>, name: impl Into<String>, anim: AnimationAsset) -> Self {
        Self::new(id, name, PrimData::Animation(anim))
    }

    /// Creates a `camera` prim.
    pub fn camera(id: impl Into<String>, name: impl Into<String>, cam: CameraAsset) -> Self {
        Self::new(id, name, PrimData::Camera(cam))
    }

    /// Creates a `light` prim.
    pub fn light(id: impl Into<String>, name: impl Into<String>, light: LightAsset) -> Self {
        Self::new(id, name, PrimData::Light(light))
    }

    /// Creates a `scene` prim wrapping a complete `solid-rs` scene graph.
    pub fn scene(id: impl Into<String>, name: impl Into<String>, scene: Scene) -> Self {
        Self::new(id, name, PrimData::Scene(scene))
    }

    /// Checks every cross-prim reference of this prim against `known` (the set
    /// of valid prim IDs).
    pub(crate) fn validate(&self, known: &HashMap<String, ()>) -> crate::Result<()> {
        use solid_rs::SolidError;
        let err = |what: &str, target: &str| {
            SolidError::invalid_ref(format!(
                "prim '{}': {what} references unknown prim '{target}'",
                self.id
            ))
        };

        match &self.data {
            PrimData::Mesh(m) => {
                for p in &m.primitives {
                    if let Some(mat) = &p.material {
                        if !known.contains_key(mat) {
                            return Err(err("primitive", mat));
                        }
                    }
                }
            }
            PrimData::SkeletalMesh(sm) => {
                for p in &sm.mesh.primitives {
                    if let Some(mat) = &p.material {
                        if !known.contains_key(mat) {
                            return Err(err("primitive", mat));
                        }
                    }
                }
                if let Some(skel) = &sm.skeleton {
                    if !known.contains_key(skel) {
                        return Err(err("skeletal mesh", skel));
                    }
                }
            }
            PrimData::Material(m) => {
                for slot in m.texture_slots() {
                    if let Some(b) = slot {
                        if !known.contains_key(&b.texture) {
                            return Err(err("material texture slot", &b.texture));
                        }
                    }
                }
            }
            PrimData::Animation(a) => {
                if let Some(skel) = &a.skeleton {
                    if !known.contains_key(skel) {
                        return Err(err("animation", skel));
                    }
                }
                if let Some(mesh) = &a.mesh {
                    if !known.contains_key(mesh) {
                        return Err(err("animation", mesh));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
