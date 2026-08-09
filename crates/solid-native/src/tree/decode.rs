//! Decodes a [`DocNode`] tree (written by [`super::encode`]) back into a
//! [`SolidDocument`].

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

use solid_rs::error::SolidError;
use solid_rs::geometry::{Aabb, SkinWeights, Topology, Transform, Vertex};
use solid_rs::parallel::Parallelism;
use solid_rs::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, Camera, FilterMode, Image,
    ImageSource, Interpolation, Light, LightBase, Material, Mesh, MorphTarget, Node, NodeId,
    OrthographicCamera, PerspectiveCamera, PointLight, SpotLight, Projection, Sampler, Scene,
    Skin, Texture, TextureRef, TextureTransform, WrapMode,
};
use solid_rs::value::Value;

use crate::doc::{Prim, PrimData, PrimKind, Props, SolidDocument};
use crate::prims::{
    AnimChannelAsset, AnimTargetAsset, AnimationAsset, Bone, BoneProperty, CameraAsset, LightAsset,
    MaterialAsset, MeshAsset, PrimitiveAsset, SkeletalMeshAsset, SkeletonAsset, TextureAsset,
    TextureBinding,
};
use crate::tree::{
    as_array, as_f32_array, as_i64, as_mat4, as_map, as_str, as_u32_array, as_vec2, as_vec3,
    as_vec4, field_bool_or, field_f32, field_i64, field_i64_or, field_opt_f32, field_opt_str,
    field_str, map_get, map_get_opt, node_to_value, DocNode,
};

type Result<T> = std::result::Result<T, SolidError>;

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(SolidError::parse(msg))
}

// ── Top level ────────────────────────────────────────────────────────────────

/// Decodes a tree into a [`SolidDocument`], running fully serially.
pub(crate) fn tree_to_document(n: &DocNode) -> Result<SolidDocument> {
    tree_to_document_with(n, &Parallelism::default())
}

/// Decodes a tree into a [`SolidDocument`], honouring the requested thread
/// count. Prim nodes (and the vertices inside meshes) are decoded in parallel
/// when `num_threads` does not force serial (`Some(1)`).
///
/// The resulting document is identical to [`tree_to_document`]: order is
/// preserved and only the decode work is parallelised.
pub(crate) fn tree_to_document_par(
    n: &DocNode,
    num_threads: Option<usize>,
) -> Result<SolidDocument> {
    tree_to_document_with(n, &Parallelism::from_num_threads(num_threads))
}

fn tree_to_document_with(n: &DocNode, par: &Parallelism) -> Result<SolidDocument> {
    let prims = par
        .map(as_array(map_get(n, "prims")?)?, |pn| prim_from_node(pn, par))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    Ok(SolidDocument {
        name: field_str(n, "name")?.to_owned(),
        props: props_from_node(map_get(n, "props")?)?,
        prims,
    })
}

fn props_from_node(n: &DocNode) -> Result<Props> {
    let mut props = Props::new();
    for (k, v) in as_map(n)? {
        props.insert(k.clone(), node_to_value(v));
    }
    Ok(props)
}

// ── Prims ────────────────────────────────────────────────────────────────────

fn prim_from_node(n: &DocNode, par: &Parallelism) -> Result<Prim> {
    let kind = PrimKind::from_str(field_str(n, "kind")?)
        .ok_or_else(|| SolidError::parse("unknown prim kind"))?;
    let id = field_str(n, "id")?.to_owned();
    let name = field_str(n, "name")?.to_owned();
    let props = props_from_node(map_get(n, "props")?)?;

    let data = match kind {
        PrimKind::Mesh => PrimData::Mesh(decode_mesh(n, par)?),
        PrimKind::Skeleton => PrimData::Skeleton(decode_skeleton(n)?),
        PrimKind::SkeletalMesh => PrimData::SkeletalMesh(decode_skeletal_mesh(n, par)?),
        PrimKind::Material => PrimData::Material(decode_material(n)?),
        PrimKind::Texture => PrimData::Texture(decode_texture(n)?),
        PrimKind::Animation => PrimData::Animation(decode_animation(n)?),
        PrimKind::Camera => PrimData::Camera(CameraAsset {
            projection: decode_projection(map_get(n, "projection")?)?,
        }),
        PrimKind::Light => PrimData::Light(decode_light(n)?),
        PrimKind::Scene => PrimData::Scene(decode_scene(n, par)?),
    };

    Ok(Prim {
        id,
        name,
        props,
        data,
    })
}

// ── Mesh ─────────────────────────────────────────────────────────────────────

fn decode_mesh(n: &DocNode, par: &Parallelism) -> Result<MeshAsset> {
    Ok(MeshAsset {
        vertices: decode_vertices(map_get(n, "vertices")?, par)?,
        primitives: as_array(map_get(n, "primitives")?)?
            .iter()
            .map(decode_primitive_asset)
            .collect::<Result<Vec<_>>>()?,
        morph_targets: decode_morphs(map_get(n, "morphs")?)?,
        morph_weights: as_f32_array(map_get(n, "morph_weights")?)?,
        bounds: decode_bounds(map_get_opt(n, "bounds")?)?,
    })
}

fn decode_primitive_asset(n: &DocNode) -> Result<PrimitiveAsset> {
    Ok(PrimitiveAsset {
        topology: parse_topology(field_str(n, "topology")?)?,
        indices: as_u32_array(map_get(n, "indices")?)?,
        material: match map_get_opt(n, "material")? {
            None => None,
            Some(v) => Some(as_str(v)?.to_owned()),
        },
    })
}

// ── Vertices (shared) ────────────────────────────────────────────────────────

/// Decodes a vertex array, running the per-vertex work in parallel when `par`
/// allows it.
pub(crate) fn decode_vertices(n: &DocNode, par: &Parallelism) -> Result<Vec<Vertex>> {
    par.map(as_array(n)?, decode_vertex).into_iter().collect()
}

fn decode_vertex(n: &DocNode) -> Result<Vertex> {
    let mut v = Vertex::new(Vec3::from_array(as_vec3(map_get(n, "p")?)?));
    if let Some(x) = map_get_opt(n, "n")? {
        v.normal = Some(Vec3::from_array(as_vec3(x)?));
    }
    if let Some(x) = map_get_opt(n, "t")? {
        v.tangent = Some(Vec4::from_array(as_vec4(x)?));
    }
    if let Some(c) = map_get_opt(n, "colors")? {
        for (slot, val) in v.colors.iter_mut().zip(as_array(c)?) {
            if !val.is_null() {
                *slot = Some(Vec4::from_array(as_vec4(val)?));
            }
        }
    }
    if let Some(u) = map_get_opt(n, "uvs")? {
        for (slot, val) in v.uvs.iter_mut().zip(as_array(u)?) {
            if !val.is_null() {
                *slot = Some(Vec2::from_array(as_vec2(val)?));
            }
        }
    }
    if let Some(sw) = map_get_opt(n, "sw")? {
        v.skin_weights = Some(SkinWeights {
            joints: as_u32_array(map_get(sw, "j")?)?
                .into_iter()
                .map(|j| j as u16)
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| SolidError::parse("skin weights need 4 joints"))?,
            weights: as_f32_array(map_get(sw, "w")?)
                .and_then(|w| {
                    w.try_into()
                        .map_err(|_| SolidError::parse("skin weights need 4 weights"))
                })?,
        });
    }
    Ok(v)
}

fn decode_morphs(n: &DocNode) -> Result<Vec<MorphTarget>> {
    as_array(n)?
        .iter()
        .map(|m| {
            Ok(MorphTarget {
                name: field_opt_str(m, "name")?.unwrap_or_default(),
                position_deltas: decode_v3_list(map_get_opt(m, "pd")?)?,
                normal_deltas: decode_v3_list(map_get_opt(m, "nd")?)?,
                tangent_deltas: decode_v3_list(map_get_opt(m, "td")?)?,
            })
        })
        .collect()
}

fn decode_v3_list(n: Option<&DocNode>) -> Result<Vec<Vec3>> {
    match n {
        None => Ok(Vec::new()),
        Some(v) => {
            let flat = as_f32_array(v)?;
            if flat.len() % 3 != 0 {
                return err("vec3 list must have a multiple of 3 floats");
            }
            Ok(flat
                .chunks_exact(3)
                .map(|c| Vec3::new(c[0], c[1], c[2]))
                .collect())
        }
    }
}

fn decode_bounds(n: Option<&DocNode>) -> Result<Option<Aabb>> {
    match n {
        None => Ok(None),
        Some(m) => Ok(Some(Aabb {
            min: Vec3::from_array(as_vec3(map_get(m, "min")?)?),
            max: Vec3::from_array(as_vec3(map_get(m, "max")?)?),
        })),
    }
}

// ── Skeleton / skeletal mesh ─────────────────────────────────────────────────

fn decode_skeleton(n: &DocNode) -> Result<SkeletonAsset> {
    let bones = as_array(map_get(n, "bones")?)?
        .iter()
        .map(|b| {
            Ok(Bone {
                name: field_str(b, "name")?.to_owned(),
                parent: decode_opt_index(map_get_opt(b, "parent")?)?,
                local_transform: Transform {
                    translation: Vec3::from_array(as_vec3(map_get(b, "t")?)?),
                    rotation: Quat::from_array(as_vec4(map_get(b, "r")?)?),
                    scale: Vec3::from_array(as_vec3(map_get(b, "s")?)?),
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(SkeletonAsset {
        bones,
        inverse_bind_matrices: decode_matrices(map_get_opt(n, "inverse_bind_matrices")?)?,
    })
}

fn decode_skeletal_mesh(n: &DocNode, par: &Parallelism) -> Result<SkeletalMeshAsset> {
    Ok(SkeletalMeshAsset {
        mesh: decode_mesh(n, par)?,
        bones: as_array(map_get(n, "bones")?)?
            .iter()
            .map(as_str)
            .map(|s| s.map(str::to_owned))
            .collect::<Result<Vec<_>>>()?,
        skeleton: field_opt_str(n, "skeleton")?,
        inverse_bind_matrices: decode_matrices(map_get_opt(n, "inverse_bind_matrices")?)?,
    })
}

fn decode_matrices(n: Option<&DocNode>) -> Result<Vec<Mat4>> {
    match n {
        None => Ok(Vec::new()),
        Some(v) => as_array(v)?.iter().map(as_mat4).collect(),
    }
}

fn decode_opt_index(n: Option<&DocNode>) -> Result<Option<usize>> {
    match n {
        None => Ok(None),
        Some(v) => {
            let x = as_i64(v)?;
            if x < 0 {
                Ok(None)
            } else {
                Ok(Some(x as usize))
            }
        }
    }
}

// ── Material ─────────────────────────────────────────────────────────────────

fn decode_material(n: &DocNode) -> Result<MaterialAsset> {
    Ok(MaterialAsset {
        base_color_factor: Vec4::from_array(as_vec4(map_get(n, "base_color_factor")?)?),
        base_color_texture: decode_binding_opt(map_get_opt(n, "base_color_texture")?)?,
        metallic_factor: field_f32(n, "metallic_factor")?,
        roughness_factor: field_f32(n, "roughness_factor")?,
        metallic_roughness_texture: decode_binding_opt(map_get_opt(
            n,
            "metallic_roughness_texture",
        )?)?,
        specular_color: Vec3::from_array(as_vec3(map_get(n, "specular_color")?)?),
        specular_color_texture: decode_binding_opt(map_get_opt(n, "specular_color_texture")?)?,
        specular_weight: field_f32(n, "specular_weight")?,
        specular_weight_texture: decode_binding_opt(map_get_opt(n, "specular_weight_texture")?)?,
        ior: field_f32(n, "ior")?,
        normal_texture: decode_binding_opt(map_get_opt(n, "normal_texture")?)?,
        normal_scale: field_f32(n, "normal_scale")?,
        occlusion_texture: decode_binding_opt(map_get_opt(n, "occlusion_texture")?)?,
        occlusion_strength: field_f32(n, "occlusion_strength")?,
        emissive_factor: Vec3::from_array(as_vec3(map_get(n, "emissive_factor")?)?),
        emissive_texture: decode_binding_opt(map_get_opt(n, "emissive_texture")?)?,
        alpha_mode: parse_alpha_mode(field_str(n, "alpha_mode")?)?,
        alpha_cutoff: field_f32(n, "alpha_cutoff")?,
        double_sided: field_bool_or(n, "double_sided", false)?,
    })
}

fn decode_binding_opt(n: Option<&DocNode>) -> Result<Option<TextureBinding>> {
    match n {
        None => Ok(None),
        Some(m) => Ok(Some(TextureBinding {
            texture: field_str(m, "texture")?.to_owned(),
            uv_channel: field_i64_or(m, "uv_channel", 0)? as usize,
            transform: decode_transform_opt(map_get_opt(m, "transform")?)?,
        })),
    }
}

fn decode_transform_opt(n: Option<&DocNode>) -> Result<Option<TextureTransform>> {
    match n {
        None => Ok(None),
        Some(m) => Ok(Some(TextureTransform {
            offset: Vec2::from_array(as_vec2(map_get(m, "offset")?)?),
            rotation: field_f32(m, "rotation")?,
            scale: Vec2::from_array(as_vec2(map_get(m, "scale")?)?),
        })),
    }
}

// ── Texture ──────────────────────────────────────────────────────────────────

fn decode_texture(n: &DocNode) -> Result<TextureAsset> {
    Ok(TextureAsset {
        name: field_opt_str(n, "asset_name")?.unwrap_or_default(),
        image: Image {
            name: field_opt_str(n, "image_name")?.unwrap_or_default(),
            source: decode_image_source(map_get(n, "source")?)?,
            extensions: solid_rs::extensions::Extensions::new(),
        },
        sampler: decode_sampler(map_get(n, "sampler")?)?,
        width: decode_opt_u32(map_get_opt(n, "width")?)?,
        height: decode_opt_u32(map_get_opt(n, "height")?)?,
    })
}

fn decode_opt_u32(n: Option<&DocNode>) -> Result<Option<u32>> {
    match n {
        None => Ok(None),
        Some(v) => Ok(Some(as_i64(v)? as u32)),
    }
}

fn decode_image_source(n: &DocNode) -> Result<ImageSource> {
    if let Some(uri) = map_get_opt(n, "uri")? {
        return Ok(ImageSource::Uri(as_str(uri)?.to_owned()));
    }
    if let Some(mime) = map_get_opt(n, "mime")? {
        return Ok(ImageSource::Embedded {
            mime_type: as_str(mime)?.to_owned(),
            data: match map_get_opt(n, "data")? {
                Some(DocNode::Bytes(b)) => b.clone(),
                _ => return err("embedded image 'data' must be bytes"),
            },
        });
    }
    err("image source needs 'uri' or 'mime'")
}

fn decode_sampler(n: &DocNode) -> Result<Sampler> {
    Ok(Sampler {
        mag_filter: parse_filter_mode(field_str(n, "mag")?)?,
        min_filter: parse_filter_mode(field_str(n, "min")?)?,
        wrap_s: parse_wrap_mode(field_str(n, "wrap_s")?)?,
        wrap_t: parse_wrap_mode(field_str(n, "wrap_t")?)?,
    })
}

// ── Animation ────────────────────────────────────────────────────────────────

fn decode_animation(n: &DocNode) -> Result<AnimationAsset> {
    let channels = as_array(map_get(n, "channels")?)?
        .iter()
        .map(decode_anim_channel_asset)
        .collect::<Result<Vec<_>>>()?;
    Ok(AnimationAsset {
        skeleton: field_opt_str(n, "skeleton")?,
        mesh: field_opt_str(n, "mesh")?,
        duration: field_opt_f32(n, "duration")?,
        channels,
    })
}

fn decode_anim_channel_asset(n: &DocNode) -> Result<AnimChannelAsset> {
    Ok(AnimChannelAsset {
        target: decode_anim_target_asset(map_get(n, "target")?)?,
        interpolation: parse_interpolation(field_str(n, "interpolation")?)?,
        times: as_f32_array(map_get(n, "times")?)?,
        values: as_f32_array(map_get(n, "values")?)?,
    })
}

fn decode_anim_target_asset(n: &DocNode) -> Result<AnimTargetAsset> {
    if let Some(b) = map_get_opt(n, "bone")? {
        return Ok(AnimTargetAsset::Bone {
            bone: field_i64(b, "index")? as usize,
            property: parse_bone_property(field_str(b, "property")?)?,
        });
    }
    if let Some(m) = map_get_opt(n, "morph")? {
        return Ok(AnimTargetAsset::MorphWeight {
            target_index: field_i64(m, "target")? as usize,
        });
    }
    if let Some(c) = map_get_opt(n, "custom")? {
        return Ok(AnimTargetAsset::Custom(as_str(c)?.to_owned()));
    }
    err("animation target must be 'bone', 'morph' or 'custom'")
}

// ── Camera / light ───────────────────────────────────────────────────────────

fn decode_projection(n: &DocNode) -> Result<Projection> {
    if let Some(p) = map_get_opt(n, "perspective")? {
        return Ok(Projection::Perspective(PerspectiveCamera {
            fov_y: field_f32(p, "fov_y")?,
            aspect_ratio: field_opt_f32(p, "aspect")?,
            z_near: field_f32(p, "near")?,
            z_far: field_opt_f32(p, "far")?,
        }));
    }
    if let Some(o) = map_get_opt(n, "orthographic")? {
        return Ok(Projection::Orthographic(OrthographicCamera {
            x_mag: field_f32(o, "x_mag")?,
            y_mag: field_f32(o, "y_mag")?,
            z_near: field_f32(o, "near")?,
            z_far: field_f32(o, "far")?,
        }));
    }
    err("projection must be 'perspective' or 'orthographic'")
}

fn decode_light(n: &DocNode) -> Result<LightAsset> {
    let color = Vec3::from_array(as_vec3(map_get(n, "color")?)?);
    let intensity = field_f32(n, "intensity")?;
    let range = field_opt_f32(n, "range")?;
    match field_str(n, "type")? {
        "directional" => Ok(LightAsset::Directional { color, intensity }),
        "point" => Ok(LightAsset::Point {
            color,
            intensity,
            range,
        }),
        "spot" => Ok(LightAsset::Spot {
            color,
            intensity,
            range,
            inner_cone_angle: field_f32(n, "inner_cone_angle")?,
            outer_cone_angle: field_f32(n, "outer_cone_angle")?,
        }),
        "area" => Ok(LightAsset::Area {
            color,
            intensity,
            width: field_f32(n, "width")?,
            height: field_f32(n, "height")?,
        }),
        other => err(format!("unknown light type '{other}'")),
    }
}

// ── Scene ────────────────────────────────────────────────────────────────────

fn decode_scene(n: &DocNode, par: &Parallelism) -> Result<Scene> {
    Ok(Scene {
        name: field_str(n, "name")?.to_owned(),
        roots: as_u32_array(map_get(n, "roots")?)?
            .into_iter()
            .map(NodeId)
            .collect(),
        nodes: as_array(map_get(n, "nodes")?)?
            .iter()
            .map(decode_node)
            .collect::<Result<Vec<_>>>()?,
        meshes: par
            .map(as_array(map_get(n, "meshes")?)?, |m| decode_scene_mesh(m, par))
            .into_iter()
            .collect::<Result<Vec<_>>>()?,
        materials: as_array(map_get(n, "materials")?)?
            .iter()
            .map(decode_scene_material)
            .collect::<Result<Vec<_>>>()?,
        textures: as_array(map_get(n, "textures")?)?
            .iter()
            .map(decode_scene_texture)
            .collect::<Result<Vec<_>>>()?,
        images: as_array(map_get(n, "images")?)?
            .iter()
            .map(decode_scene_image)
            .collect::<Result<Vec<_>>>()?,
        cameras: as_array(map_get(n, "cameras")?)?
            .iter()
            .map(decode_scene_camera)
            .collect::<Result<Vec<_>>>()?,
        lights: as_array(map_get(n, "lights")?)?
            .iter()
            .map(decode_scene_light)
            .collect::<Result<Vec<_>>>()?,
        skins: as_array(map_get(n, "skins")?)?
            .iter()
            .map(decode_scene_skin)
            .collect::<Result<Vec<_>>>()?,
        animations: as_array(map_get(n, "animations")?)?
            .iter()
            .map(decode_scene_animation)
            .collect::<Result<Vec<_>>>()?,
        extensions: solid_rs::extensions::Extensions::new(),
        metadata: decode_metadata(map_get_opt(n, "metadata")?)?,
    })
}

fn decode_node(n: &DocNode) -> Result<Node> {
    Ok(Node {
        id: NodeId(field_i64(n, "id")? as u32),
        name: field_str(n, "name")?.to_owned(),
        transform: Transform {
            translation: Vec3::from_array(as_vec3(map_get(n, "t")?)?),
            rotation: Quat::from_array(as_vec4(map_get(n, "r")?)?),
            scale: Vec3::from_array(as_vec3(map_get(n, "s")?)?),
        },
        children: as_u32_array(map_get(n, "children")?)?
            .into_iter()
            .map(NodeId)
            .collect(),
        parent: decode_opt_node_id(map_get_opt(n, "parent")?)?,
        mesh: decode_opt_usize(map_get_opt(n, "mesh")?)?,
        camera: decode_opt_usize(map_get_opt(n, "camera")?)?,
        light: decode_opt_usize(map_get_opt(n, "light")?)?,
        skin: decode_opt_usize(map_get_opt(n, "skin")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_opt_node_id(n: Option<&DocNode>) -> Result<Option<NodeId>> {
    match n {
        None => Ok(None),
        Some(v) => {
            let x = as_i64(v)?;
            if x < 0 {
                Ok(None)
            } else {
                Ok(Some(NodeId(x as u32)))
            }
        }
    }
}

fn decode_opt_usize(n: Option<&DocNode>) -> Result<Option<usize>> {
    match n {
        None => Ok(None),
        Some(v) => {
            let x = as_i64(v)?;
            if x < 0 {
                Ok(None)
            } else {
                Ok(Some(x as usize))
            }
        }
    }
}

fn decode_scene_mesh(n: &DocNode, par: &Parallelism) -> Result<Mesh> {
    Ok(Mesh {
        name: field_str(n, "name")?.to_owned(),
        vertices: decode_vertices(map_get(n, "vertices")?, par)?,
        primitives: as_array(map_get(n, "primitives")?)?
            .iter()
            .map(|p| {
                Ok(solid_rs::geometry::Primitive {
                    topology: parse_topology(field_str(p, "topology")?)?,
                    indices: as_u32_array(map_get(p, "indices")?)?,
                    material_index: decode_opt_usize(map_get_opt(p, "material")?)?,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        morph_targets: decode_morphs(map_get(n, "morphs")?)?,
        morph_weights: as_f32_array(map_get(n, "morph_weights")?)?,
        bounds: decode_bounds(map_get_opt(n, "bounds")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_material(n: &DocNode) -> Result<Material> {
    Ok(Material {
        name: field_str(n, "name")?.to_owned(),
        base_color_factor: Vec4::from_array(as_vec4(map_get(n, "base_color_factor")?)?),
        base_color_texture: decode_texref_opt(map_get_opt(n, "base_color_texture")?)?,
        metallic_factor: field_f32(n, "metallic_factor")?,
        roughness_factor: field_f32(n, "roughness_factor")?,
        metallic_roughness_texture: decode_texref_opt(map_get_opt(
            n,
            "metallic_roughness_texture",
        )?)?,
        specular_color: Vec3::from_array(as_vec3(map_get(n, "specular_color")?)?),
        specular_color_texture: decode_texref_opt(map_get_opt(n, "specular_color_texture")?)?,
        specular_weight: field_f32(n, "specular_weight")?,
        specular_weight_texture: decode_texref_opt(map_get_opt(n, "specular_weight_texture")?)?,
        ior: field_f32(n, "ior")?,
        normal_texture: decode_texref_opt(map_get_opt(n, "normal_texture")?)?,
        normal_scale: field_f32(n, "normal_scale")?,
        occlusion_texture: decode_texref_opt(map_get_opt(n, "occlusion_texture")?)?,
        occlusion_strength: field_f32(n, "occlusion_strength")?,
        emissive_factor: Vec3::from_array(as_vec3(map_get(n, "emissive_factor")?)?),
        emissive_texture: decode_texref_opt(map_get_opt(n, "emissive_texture")?)?,
        alpha_mode: parse_alpha_mode(field_str(n, "alpha_mode")?)?,
        alpha_cutoff: field_f32(n, "alpha_cutoff")?,
        double_sided: field_bool_or(n, "double_sided", false)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_texref_opt(n: Option<&DocNode>) -> Result<Option<TextureRef>> {
    match n {
        None => Ok(None),
        Some(m) => Ok(Some(TextureRef {
            texture_index: field_i64(m, "texture")? as usize,
            uv_channel: field_i64_or(m, "uv_channel", 0)? as usize,
            transform: decode_transform_opt(map_get_opt(m, "transform")?)?,
        })),
    }
}

fn decode_scene_texture(n: &DocNode) -> Result<Texture> {
    Ok(Texture {
        name: field_str(n, "name")?.to_owned(),
        image_index: field_i64(n, "image")? as usize,
        sampler: decode_sampler(map_get(n, "sampler")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_image(n: &DocNode) -> Result<Image> {
    Ok(Image {
        name: field_str(n, "name")?.to_owned(),
        source: decode_image_source(map_get(n, "source")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_camera(n: &DocNode) -> Result<Camera> {
    Ok(Camera {
        name: field_str(n, "name")?.to_owned(),
        projection: decode_projection(map_get(n, "projection")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_light(n: &DocNode) -> Result<Light> {
    let base = LightBase {
        name: field_str(n, "name")?.to_owned(),
        color: Vec3::from_array(as_vec3(map_get(n, "color")?)?),
        intensity: field_f32(n, "intensity")?,
    };
    let range = field_opt_f32(n, "range")?;
    let light = match field_str(n, "type")? {
        "directional" => Light::Directional(solid_rs::scene::DirectionalLight {
            base,
            extensions: solid_rs::extensions::Extensions::new(),
        }),
        "point" => Light::Point(PointLight {
            base,
            range,
            extensions: solid_rs::extensions::Extensions::new(),
        }),
        "spot" => Light::Spot(SpotLight {
            base,
            range,
            inner_cone_angle: field_f32(n, "inner_cone_angle")?,
            outer_cone_angle: field_f32(n, "outer_cone_angle")?,
            extensions: solid_rs::extensions::Extensions::new(),
        }),
        "area" => Light::Area(solid_rs::scene::AreaLight {
            base,
            width: field_f32(n, "width")?,
            height: field_f32(n, "height")?,
            extensions: solid_rs::extensions::Extensions::new(),
        }),
        other => return err(format!("unknown light type '{other}'")),
    };
    Ok(light)
}

fn decode_scene_skin(n: &DocNode) -> Result<Skin> {
    Ok(Skin {
        name: field_str(n, "name")?.to_owned(),
        skeleton_root: decode_opt_node_id(map_get_opt(n, "skeleton_root")?)?,
        joints: as_u32_array(map_get(n, "joints")?)?
            .into_iter()
            .map(NodeId)
            .collect(),
        inverse_bind_matrices: decode_matrices(map_get_opt(n, "inverse_bind_matrices")?)?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_animation(n: &DocNode) -> Result<Animation> {
    Ok(Animation {
        name: field_str(n, "name")?.to_owned(),
        channels: as_array(map_get(n, "channels")?)?
            .iter()
            .map(decode_scene_channel)
            .collect::<Result<Vec<_>>>()?,
        extensions: solid_rs::extensions::Extensions::new(),
    })
}

fn decode_scene_channel(n: &DocNode) -> Result<AnimationChannel> {
    Ok(AnimationChannel {
        target: decode_scene_anim_target(map_get(n, "target")?)?,
        interpolation: parse_interpolation(field_str(n, "interpolation")?)?,
        times: as_f32_array(map_get(n, "times")?)?,
        values: as_f32_array(map_get(n, "values")?)?,
    })
}

fn decode_scene_anim_target(n: &DocNode) -> Result<AnimationTarget> {
    if let Some(id) = map_get_opt(n, "translation")? {
        return Ok(AnimationTarget::Translation(NodeId(as_i64(id)? as u32)));
    }
    if let Some(id) = map_get_opt(n, "rotation")? {
        return Ok(AnimationTarget::Rotation(NodeId(as_i64(id)? as u32)));
    }
    if let Some(id) = map_get_opt(n, "scale")? {
        return Ok(AnimationTarget::Scale(NodeId(as_i64(id)? as u32)));
    }
    if let Some(m) = map_get_opt(n, "morph")? {
        return Ok(AnimationTarget::MorphWeight {
            node_id: NodeId(field_i64(m, "node")? as u32),
            target_index: field_i64(m, "target")? as usize,
        });
    }
    err("scene animation target must be 'translation', 'rotation', 'scale' or 'morph'")
}

fn decode_metadata(n: Option<&DocNode>) -> Result<solid_rs::scene::Metadata> {
    let mut meta = solid_rs::scene::Metadata::default();
    let Some(m) = n else { return Ok(meta) };
    meta.generator = field_opt_str(m, "generator")?;
    meta.copyright = field_opt_str(m, "copyright")?;
    meta.source_format = field_opt_str(m, "source_format")?;
    if let Some(extra) = map_get_opt(m, "extra")? {
        for (k, v) in as_map(extra)? {
            meta.extra.insert(k.clone(), node_to_value(v));
        }
    }
    Ok(meta)
}

// ── Enum string parsers ──────────────────────────────────────────────────────

fn parse_topology(s: &str) -> Result<Topology> {
    match s {
        "triangle_list" => Ok(Topology::TriangleList),
        "triangle_strip" => Ok(Topology::TriangleStrip),
        "line_list" => Ok(Topology::LineList),
        "line_strip" => Ok(Topology::LineStrip),
        "point_list" => Ok(Topology::PointList),
        "quad_list" => Ok(Topology::QuadList),
        "polygon" => Ok(Topology::Polygon),
        _ => err(format!("unknown topology '{s}'")),
    }
}

fn parse_interpolation(s: &str) -> Result<Interpolation> {
    match s {
        "linear" => Ok(Interpolation::Linear),
        "step" => Ok(Interpolation::Step),
        "cubic_spline" => Ok(Interpolation::CubicSpline),
        _ => err(format!("unknown interpolation '{s}'")),
    }
}

fn parse_alpha_mode(s: &str) -> Result<AlphaMode> {
    match s {
        "opaque" => Ok(AlphaMode::Opaque),
        "mask" => Ok(AlphaMode::Mask),
        "blend" => Ok(AlphaMode::Blend),
        _ => err(format!("unknown alpha mode '{s}'")),
    }
}

fn parse_wrap_mode(s: &str) -> Result<WrapMode> {
    match s {
        "repeat" => Ok(WrapMode::Repeat),
        "mirrored_repeat" => Ok(WrapMode::MirroredRepeat),
        "clamp_to_edge" => Ok(WrapMode::ClampToEdge),
        _ => err(format!("unknown wrap mode '{s}'")),
    }
}

fn parse_filter_mode(s: &str) -> Result<FilterMode> {
    match s {
        "nearest" => Ok(FilterMode::Nearest),
        "linear" => Ok(FilterMode::Linear),
        "nearest_mipmap_nearest" => Ok(FilterMode::NearestMipmapNearest),
        "linear_mipmap_nearest" => Ok(FilterMode::LinearMipmapNearest),
        "nearest_mipmap_linear" => Ok(FilterMode::NearestMipmapLinear),
        "linear_mipmap_linear" => Ok(FilterMode::LinearMipmapLinear),
        _ => err(format!("unknown filter mode '{s}'")),
    }
}

fn parse_bone_property(s: &str) -> Result<BoneProperty> {
    match s {
        "translation" => Ok(BoneProperty::Translation),
        "rotation" => Ok(BoneProperty::Rotation),
        "scale" => Ok(BoneProperty::Scale),
        _ => err(format!("unknown bone property '{s}'")),
    }
}

// Silence unused warning for as_map used only for props in some builds.
#[allow(unused)]
fn _keep(_: HashMap<String, Value>) {}
