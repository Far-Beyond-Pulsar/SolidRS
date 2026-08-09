//! Material asset: a PBR material whose texture slots reference texture prims
//! by ID.

use glam::{Vec3, Vec4};

use solid_rs::scene::{AlphaMode, Material, TextureRef, TextureTransform};

/// A reference from a material slot to a [`TextureAsset`](crate::prims::TextureAsset)
/// prim, by prim ID.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureBinding {
    /// Prim ID of the texture prim.
    pub texture: String,
    /// UV channel to sample (0 = primary set).
    pub uv_channel: usize,
    /// Optional affine UV transform applied before sampling.
    pub transform: Option<TextureTransform>,
}

impl TextureBinding {
    /// Creates a plain binding with no UV transform.
    pub fn new(texture: impl Into<String>) -> Self {
        Self {
            texture: texture.into(),
            uv_channel: 0,
            transform: None,
        }
    }
}

/// Canonical physically based material.  Mirrors
/// [`solid_rs::scene::Material`] but references textures by prim ID so the
/// asset stands alone.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialAsset {
    /// Linear RGBA base-colour multiplier.
    pub base_color_factor: Vec4,
    /// Optional base-colour texture.
    pub base_color_texture: Option<TextureBinding>,
    /// 0 = dielectric, 1 = metallic.
    pub metallic_factor: f32,
    /// 0 = smooth (mirror), 1 = rough.
    pub roughness_factor: f32,
    /// Optional combined metallic (B) / roughness (G) texture.
    pub metallic_roughness_texture: Option<TextureBinding>,
    /// Linear RGB specular tint for explicit dielectric workflows.
    pub specular_color: Vec3,
    /// Optional specular-colour texture.
    pub specular_color_texture: Option<TextureBinding>,
    /// Scalar multiplier for explicit specular response.
    pub specular_weight: f32,
    /// Optional specular-strength texture.
    pub specular_weight_texture: Option<TextureBinding>,
    /// Index of refraction for the dielectric interface.
    pub ior: f32,
    /// Optional tangent-space normal map.
    pub normal_texture: Option<TextureBinding>,
    /// Normal-map scale factor.
    pub normal_scale: f32,
    /// Optional ambient occlusion texture (R channel).
    pub occlusion_texture: Option<TextureBinding>,
    /// Occlusion strength multiplier.
    pub occlusion_strength: f32,
    /// Linear RGB emissive colour multiplier.
    pub emissive_factor: Vec3,
    /// Optional emissive texture.
    pub emissive_texture: Option<TextureBinding>,
    /// Alpha blending strategy.
    pub alpha_mode: AlphaMode,
    /// Alpha threshold for [`AlphaMode::Mask`].
    pub alpha_cutoff: f32,
    /// Whether back faces should be rendered.
    pub double_sided: bool,
}

impl Default for MaterialAsset {
    fn default() -> Self {
        Self {
            base_color_factor: Vec4::ONE,
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            specular_color: Vec3::ONE,
            specular_color_texture: None,
            specular_weight: 1.0,
            specular_weight_texture: None,
            ior: 1.5,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive_factor: Vec3::ZERO,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

impl MaterialAsset {
    /// Creates a default material.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a simple untextured material with the given base colour.
    pub fn solid_color(color: Vec4) -> Self {
        Self {
            base_color_factor: color,
            ..Default::default()
        }
    }

    /// Converts to a `solid-rs` material, resolving each texture prim ID to a
    /// scene texture index through `resolve`.  Unresolvable slots are dropped.
    pub fn to_solid(&self, resolve: &impl Fn(&str) -> Option<usize>) -> Material {
        Material {
            name: String::new(),
            base_color_factor: self.base_color_factor,
            base_color_texture: binding_to_ref(&self.base_color_texture, resolve),
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            metallic_roughness_texture: binding_to_ref(&self.metallic_roughness_texture, resolve),
            specular_color: self.specular_color,
            specular_color_texture: binding_to_ref(&self.specular_color_texture, resolve),
            specular_weight: self.specular_weight,
            specular_weight_texture: binding_to_ref(&self.specular_weight_texture, resolve),
            ior: self.ior,
            normal_texture: binding_to_ref(&self.normal_texture, resolve),
            normal_scale: self.normal_scale,
            occlusion_texture: binding_to_ref(&self.occlusion_texture, resolve),
            occlusion_strength: self.occlusion_strength,
            emissive_factor: self.emissive_factor,
            emissive_texture: binding_to_ref(&self.emissive_texture, resolve),
            alpha_mode: self.alpha_mode,
            alpha_cutoff: self.alpha_cutoff,
            double_sided: self.double_sided,
            extensions: solid_rs::extensions::Extensions::new(),
        }
    }

    /// Converts from a `solid-rs` material, mapping each texture index to a
    /// texture prim ID through `resolve`.  Unresolvable slots are dropped.
    pub fn from_solid(m: &Material, resolve: &impl Fn(usize) -> Option<String>) -> Self {
        Self {
            base_color_factor: m.base_color_factor,
            base_color_texture: ref_to_binding(&m.base_color_texture, resolve),
            metallic_factor: m.metallic_factor,
            roughness_factor: m.roughness_factor,
            metallic_roughness_texture: ref_to_binding(&m.metallic_roughness_texture, resolve),
            specular_color: m.specular_color,
            specular_color_texture: ref_to_binding(&m.specular_color_texture, resolve),
            specular_weight: m.specular_weight,
            specular_weight_texture: ref_to_binding(&m.specular_weight_texture, resolve),
            ior: m.ior,
            normal_texture: ref_to_binding(&m.normal_texture, resolve),
            normal_scale: m.normal_scale,
            occlusion_texture: ref_to_binding(&m.occlusion_texture, resolve),
            occlusion_strength: m.occlusion_strength,
            emissive_factor: m.emissive_factor,
            emissive_texture: ref_to_binding(&m.emissive_texture, resolve),
            alpha_mode: m.alpha_mode,
            alpha_cutoff: m.alpha_cutoff,
            double_sided: m.double_sided,
        }
    }

    /// Iterates over every texture slot (for validation and re-indexing).
    pub fn texture_slots(&self) -> impl Iterator<Item = &Option<TextureBinding>> {
        [
            &self.base_color_texture,
            &self.metallic_roughness_texture,
            &self.specular_color_texture,
            &self.specular_weight_texture,
            &self.normal_texture,
            &self.occlusion_texture,
            &self.emissive_texture,
        ]
        .into_iter()
    }
}

fn binding_to_ref(
    b: &Option<TextureBinding>,
    resolve: &impl Fn(&str) -> Option<usize>,
) -> Option<TextureRef> {
    b.as_ref().and_then(|b| {
        let idx = resolve(&b.texture)?;
        Some(TextureRef {
            texture_index: idx,
            uv_channel: b.uv_channel,
            transform: b.transform.clone(),
        })
    })
}

fn ref_to_binding(
    r: &Option<TextureRef>,
    resolve: &impl Fn(usize) -> Option<String>,
) -> Option<TextureBinding> {
    r.as_ref().and_then(|r| {
        let id = resolve(r.texture_index)?;
        Some(TextureBinding {
            texture: id,
            uv_channel: r.uv_channel,
            transform: r.transform.clone(),
        })
    })
}
