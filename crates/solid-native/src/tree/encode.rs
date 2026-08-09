//! Encodes a [`SolidDocument`](crate::doc::SolidDocument) into the shared
//! [`DocNode`] tree schema.  Both the `.slda` and `.sldb` codecs serialise
//! this tree, so the schema is defined here exactly once.

use glam::{Mat4, Vec2, Vec3, Vec4};

use solid_rs::geometry::{Aabb, SkinWeights, Topology, Vertex};
use solid_rs::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, FilterMode, Image, ImageSource,
    Interpolation, Light, Material, Mesh, MorphTarget, Node, Projection, Sampler, Skin, Texture,
    TextureRef, TextureTransform, WrapMode,
};

use crate::doc::{Prim, PrimData, Props, SolidDocument};
use crate::prims::{
    AnimChannelAsset, AnimTargetAsset, AnimationAsset, BoneProperty, LightAsset, MaterialAsset,
    MeshAsset, PrimitiveAsset, SkeletalMeshAsset, SkeletonAsset, TextureAsset, TextureBinding,
};
use crate::tree::{m, value_to_node, DocNode};

// ── Value helpers ────────────────────────────────────────────────────────────

fn i(v: i64) -> DocNode {
    DocNode::Int(v)
}
fn f(v: f32) -> DocNode {
    DocNode::Float(v as f64)
}
fn s(v: impl Into<String>) -> DocNode {
    DocNode::String(v.into())
}
fn v2(v: Vec2) -> DocNode {
    DocNode::Vec2(v.to_array())
}
fn v3(v: Vec3) -> DocNode {
    DocNode::Vec3(v.to_array())
}
fn v4(v: Vec4) -> DocNode {
    DocNode::Vec4(v.to_array())
}
fn f32a(v: Vec<f32>) -> DocNode {
    DocNode::F32Array(v)
}
fn u32a(v: Vec<u32>) -> DocNode {
    DocNode::U32Array(v)
}
fn mat4(m: Mat4) -> DocNode {
    DocNode::F32Array(m.to_cols_array().to_vec())
}
fn opt(v: Option<DocNode>) -> DocNode {
    v.unwrap_or(DocNode::Null)
}

fn push_opt(pairs: &mut Vec<(String, DocNode)>, key: &'static str, v: Option<DocNode>) {
    if let Some(v) = v {
        pairs.push((key.to_string(), v));
    }
}

// ── Top level ────────────────────────────────────────────────────────────────

/// Encodes a whole document into the shared tree schema.
///
/// Validates the document first (duplicate prim IDs and dangling
/// cross-prim references) so invalid documents fail at write time.
pub(crate) fn document_to_tree(doc: &SolidDocument) -> crate::Result<DocNode> {
    doc.validate()?;
    Ok(m![
        "name" => s(&doc.name),
        "props" => props_to_node(&doc.props),
        "prims" => DocNode::Array(doc.prims.iter().map(prim_to_node).collect()),
    ])
}

fn props_to_node(props: &Props) -> DocNode {
    let mut pairs: Vec<(String, DocNode)> = props
        .iter()
        .map(|(k, v)| (k.clone(), value_to_node(v)))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    DocNode::Map(pairs)
}

// ── Prims ────────────────────────────────────────────────────────────────────

fn prim_to_node(p: &Prim) -> DocNode {
    let mut pairs = vec![
        ("kind".to_string(), s(p.kind().as_str())),
        ("id".to_string(), s(&p.id)),
        ("name".to_string(), s(&p.name)),
        ("props".to_string(), props_to_node(&p.props)),
    ];
    match &p.data {
        PrimData::Mesh(m) => pairs.extend(mesh_asset_pairs(m)),
        PrimData::Skeleton(sk) => pairs.extend(skeleton_pairs(sk)),
        PrimData::SkeletalMesh(sm) => pairs.extend(skeletal_mesh_pairs(sm)),
        PrimData::Material(mat) => pairs.extend(material_asset_pairs(mat)),
        PrimData::Texture(tex) => pairs.extend(texture_pairs(tex)),
        PrimData::Animation(anim) => pairs.extend(animation_pairs(anim)),
        PrimData::Camera(cam) => {
            pairs.push(("projection".to_string(), encode_projection(&cam.projection)))
        }
        PrimData::Light(light) => pairs.extend(light_asset_pairs(light)),
        PrimData::Scene(scene) => pairs.extend(scene_pairs(scene)),
    }
    DocNode::Map(pairs)
}

// ── Mesh (asset) ─────────────────────────────────────────────────────────────

fn mesh_asset_pairs(m: &MeshAsset) -> Vec<(String, DocNode)> {
    vec![
        ("vertices".to_string(), encode_vertices(&m.vertices)),
        (
            "primitives".to_string(),
            DocNode::Array(m.primitives.iter().map(encode_primitive_asset).collect()),
        ),
        ("morphs".to_string(), encode_morphs(&m.morph_targets)),
        ("morph_weights".to_string(), f32a(m.morph_weights.clone())),
        ("bounds".to_string(), encode_bounds(m.bounds)),
    ]
}

fn encode_primitive_asset(p: &PrimitiveAsset) -> DocNode {
    m![
        "topology" => s(topology_str(p.topology)),
        "indices" => u32a(p.indices.clone()),
        "material" => opt(p.material.as_ref().map(|m| s(m))),
    ]
}

// ── Vertices (shared) ────────────────────────────────────────────────────────

pub(crate) fn encode_vertices(verts: &[Vertex]) -> DocNode {
    DocNode::Array(verts.iter().map(encode_vertex).collect())
}

fn encode_vertex(v: &Vertex) -> DocNode {
    let mut pairs = vec![("p".to_string(), v3(v.position))];
    if let Some(n) = v.normal {
        pairs.push(("n".to_string(), v3(n)));
    }
    if let Some(t) = v.tangent {
        pairs.push(("t".to_string(), v4(t.to_array())));
    }
    if v.colors.iter().any(Option::is_some) {
        pairs.push((
            "colors".to_string(),
            DocNode::Array(
                v.colors
                    .iter()
                    .map(|c| c.map(|c| v4(c.to_array())).unwrap_or(DocNode::Null))
                    .collect(),
            ),
        ));
    }
    if v.uvs.iter().any(Option::is_some) {
        pairs.push((
            "uvs".to_string(),
            DocNode::Array(
                v.uvs
                    .iter()
                    .map(|u| u.map(|u| v2(u.to_array())).unwrap_or(DocNode::Null))
                    .collect(),
            ),
        ));
    }
    if let Some(sw) = &v.skin_weights {
        pairs.push((
            "sw".to_string(),
            m![
                "j" => u32a(sw.joints.iter().map(|&j| j as u32).collect()),
                "w" => f32a(sw.weights.to_vec()),
            ],
        ));
    }
    DocNode::Map(pairs)
}

fn encode_morphs(morphs: &[MorphTarget]) -> DocNode {
    DocNode::Array(
        morphs
            .iter()
            .map(|mt| {
                let mut pairs = vec![("name".to_string(), s(&mt.name))];
                if !mt.position_deltas.is_empty() {
                    pairs.push(("pd".to_string(), f32a(flatten_v3(&mt.position_deltas))));
                }
                if !mt.normal_deltas.is_empty() {
                    pairs.push(("nd".to_string(), f32a(flatten_v3(&mt.normal_deltas))));
                }
                if !mt.tangent_deltas.is_empty() {
                    pairs.push(("td".to_string(), f32a(flatten_v3(&mt.tangent_deltas))));
                }
                DocNode::Map(pairs)
            })
            .collect(),
    )
}

fn flatten_v3(v: &[Vec3]) -> Vec<f32> {
    v.iter().flat_map(|v| v.to_array()).collect()
}

fn encode_bounds(b: Option<Aabb>) -> DocNode {
    match b {
        Some(a) => m!["min" => v3(a.min), "max" => v3(a.max)],
        None => DocNode::Null,
    }
}

// ── Skeleton / skeletal mesh ─────────────────────────────────────────────────

fn skeleton_pairs(sk: &SkeletonAsset) -> Vec<(String, DocNode)> {
    vec![
        (
            "bones".to_string(),
            DocNode::Array(
                sk.bones
                    .iter()
                    .map(|b| {
                        m![
                            "name" => s(&b.name),
                            "parent" => i(b.parent.map_or(-1, |p| p as i64)),
                            "t" => v3(b.local_transform.translation),
                            "r" => v4(b.local_transform.rotation.to_array()),
                            "s" => v3(b.local_transform.scale),
                        ]
                    })
                    .collect(),
            ),
        ),
        (
            "inverse_bind_matrices".to_string(),
            DocNode::Array(sk.inverse_bind_matrices.iter().map(|m| mat4(*m)).collect()),
        ),
    ]
}

fn skeletal_mesh_pairs(sm: &SkeletalMeshAsset) -> Vec<(String, DocNode)> {
    let mut pairs = mesh_asset_pairs(&sm.mesh);
    pairs.push((
        "bones".to_string(),
        DocNode::Array(sm.bones.iter().map(|b| s(b)).collect()),
    ));
    pairs.push(("skeleton".to_string(), opt(sm.skeleton.as_ref().map(|id| s(id)))));
    pairs.push((
        "inverse_bind_matrices".to_string(),
        DocNode::Array(sm.inverse_bind_matrices.iter().map(|m| mat4(*m)).collect()),
    ));
    pairs
}

// ── Material ─────────────────────────────────────────────────────────────────

fn material_asset_pairs(mat: &MaterialAsset) -> Vec<(String, DocNode)> {
    vec![
        ("base_color_factor".to_string(), v4(mat.base_color_factor)),
        (
            "base_color_texture".to_string(),
            encode_binding_opt(&mat.base_color_texture),
        ),
        ("metallic_factor".to_string(), f(mat.metallic_factor)),
        ("roughness_factor".to_string(), f(mat.roughness_factor)),
        (
            "metallic_roughness_texture".to_string(),
            encode_binding_opt(&mat.metallic_roughness_texture),
        ),
        ("specular_color".to_string(), v3(mat.specular_color)),
        (
            "specular_color_texture".to_string(),
            encode_binding_opt(&mat.specular_color_texture),
        ),
        ("specular_weight".to_string(), f(mat.specular_weight)),
        (
            "specular_weight_texture".to_string(),
            encode_binding_opt(&mat.specular_weight_texture),
        ),
        ("ior".to_string(), f(mat.ior)),
        ("normal_texture".to_string(), encode_binding_opt(&mat.normal_texture)),
        ("normal_scale".to_string(), f(mat.normal_scale)),
        (
            "occlusion_texture".to_string(),
            encode_binding_opt(&mat.occlusion_texture),
        ),
        ("occlusion_strength".to_string(), f(mat.occlusion_strength)),
        ("emissive_factor".to_string(), v3(mat.emissive_factor)),
        ("emissive_texture".to_string(), encode_binding_opt(&mat.emissive_texture)),
        ("alpha_mode".to_string(), s(alpha_mode_str(mat.alpha_mode))),
        ("alpha_cutoff".to_string(), f(mat.alpha_cutoff)),
        ("double_sided".to_string(), DocNode::Bool(mat.double_sided)),
    ]
}

fn encode_binding_opt(b: &Option<TextureBinding>) -> DocNode {
    match b {
        Some(b) => encode_texture_binding(b),
        None => DocNode::Null,
    }
}

fn encode_texture_binding(b: &TextureBinding) -> DocNode {
    let mut pairs = vec![
        ("texture".to_string(), s(&b.texture)),
        ("uv_channel".to_string(), i(b.uv_channel as i64)),
    ];
    if let Some(t) = &b.transform {
        pairs.push(("transform".to_string(), encode_texture_transform(t)));
    }
    DocNode::Map(pairs)
}

fn encode_texture_transform(t: &TextureTransform) -> DocNode {
    m![
        "offset" => v2(t.offset.to_array()),
        "rotation" => f(t.rotation),
        "scale" => v2(t.scale.to_array()),
    ]
}

// ── Texture ──────────────────────────────────────────────────────────────────

fn texture_pairs(tex: &TextureAsset) -> Vec<(String, DocNode)> {
    let mut pairs = vec![
        ("source".to_string(), encode_image_source(&tex.image.source)),
        ("sampler".to_string(), encode_sampler(&tex.sampler)),
    ];
    push_opt(&mut pairs, "width", tex.width.map(|w| i(w as i64)));
    push_opt(&mut pairs, "height", tex.height.map(|h| i(h as i64)));
    pairs
}

fn encode_image_source(src: &ImageSource) -> DocNode {
    match src {
        ImageSource::Uri(uri) => m!["uri" => s(uri)],
        ImageSource::Embedded { mime_type, data } => m![
            "mime" => s(mime_type),
            "data" => DocNode::Bytes(data.clone()),
        ],
    }
}

fn encode_sampler(samp: &Sampler) -> DocNode {
    m![
        "mag" => s(filter_mode_str(samp.mag_filter)),
        "min" => s(filter_mode_str(samp.min_filter)),
        "wrap_s" => s(wrap_mode_str(samp.wrap_s)),
        "wrap_t" => s(wrap_mode_str(samp.wrap_t)),
    ]
}

// ── Animation ────────────────────────────────────────────────────────────────

fn animation_pairs(a: &AnimationAsset) -> Vec<(String, DocNode)> {
    let mut pairs = vec![(
        "channels".to_string(),
        DocNode::Array(a.channels.iter().map(encode_anim_channel_asset).collect()),
    )];
    push_opt(&mut pairs, "skeleton", a.skeleton.as_ref().map(|s| s(s)));
    push_opt(&mut pairs, "mesh", a.mesh.as_ref().map(|s| s(s)));
    push_opt(&mut pairs, "duration", a.duration.map(|d| f(d)));
    pairs
}

fn encode_anim_channel_asset(c: &AnimChannelAsset) -> DocNode {
    m![
        "target" => encode_anim_target_asset(&c.target),
        "interpolation" => s(interpolation_str(c.interpolation)),
        "times" => f32a(c.times.clone()),
        "values" => f32a(c.values.clone()),
    ]
}

fn encode_anim_target_asset(t: &AnimTargetAsset) -> DocNode {
    match t {
        AnimTargetAsset::Bone { bone, property } => m![
            "bone" => m![
                "index" => i(*bone as i64),
                "property" => s(bone_property_str(*property)),
            ],
        ],
        AnimTargetAsset::MorphWeight { target_index } => m![
            "morph" => m![ "target" => i(*target_index as i64) ],
        ],
        AnimTargetAsset::Custom(name) => m![ "custom" => s(name) ],
    }
}

// ── Camera / light ───────────────────────────────────────────────────────────

fn light_asset_pairs(l: &LightAsset) -> Vec<(String, DocNode)> {
    let mut pairs = vec![
        ("type".to_string(), s(light_type_str(l))),
        ("color".to_string(), v3(l.color())),
        ("intensity".to_string(), f(l.intensity())),
    ];
    match l {
        LightAsset::Point { range, .. } => push_opt(&mut pairs, "range", range.map(f)),
        LightAsset::Spot {
            range,
            inner_cone_angle,
            outer_cone_angle,
            ..
        } => {
            push_opt(&mut pairs, "range", range.map(f));
            pairs.push(("inner_cone_angle".to_string(), f(*inner_cone_angle)));
            pairs.push(("outer_cone_angle".to_string(), f(*outer_cone_angle)));
        }
        LightAsset::Area { width, height, .. } => {
            pairs.push(("width".to_string(), f(*width)));
            pairs.push(("height".to_string(), f(*height)));
        }
        LightAsset::Directional { .. } => {}
    }
    pairs
}

fn light_type_str(l: &LightAsset) -> &'static str {
    match l {
        LightAsset::Directional { .. } => "directional",
        LightAsset::Point { .. } => "point",
        LightAsset::Spot { .. } => "spot",
        LightAsset::Area { .. } => "area",
    }
}

// ── Scene ────────────────────────────────────────────────────────────────────

fn scene_pairs(scene: &solid_rs::scene::Scene) -> Vec<(String, DocNode)> {
    let mut pairs = vec![
        ("name".to_string(), s(&scene.name)),
        (
            "roots".to_string(),
            u32a(scene.roots.iter().map(|r| r.0).collect()),
        ),
        ("metadata".to_string(), encode_metadata(&scene.metadata)),
    ];
    pairs.push((
        "nodes".to_string(),
        DocNode::Array(scene.nodes.iter().map(encode_node).collect()),
    ));
    pairs.push((
        "meshes".to_string(),
        DocNode::Array(scene.meshes.iter().map(encode_scene_mesh).collect()),
    ));
    pairs.push((
        "materials".to_string(),
        DocNode::Array(scene.materials.iter().map(encode_scene_material).collect()),
    ));
    pairs.push((
        "textures".to_string(),
        DocNode::Array(scene.textures.iter().map(encode_scene_texture).collect()),
    ));
    pairs.push((
        "images".to_string(),
        DocNode::Array(scene.images.iter().map(encode_scene_image).collect()),
    ));
    pairs.push((
        "cameras".to_string(),
        DocNode::Array(scene.cameras.iter().map(encode_scene_camera).collect()),
    ));
    pairs.push((
        "lights".to_string(),
        DocNode::Array(scene.lights.iter().map(encode_scene_light).collect()),
    ));
    pairs.push((
        "skins".to_string(),
        DocNode::Array(scene.skins.iter().map(encode_scene_skin).collect()),
    ));
    pairs.push((
        "animations".to_string(),
        DocNode::Array(scene.animations.iter().map(encode_scene_animation).collect()),
    ));
    DocNode::Map(pairs)
}

fn encode_metadata(meta: &solid_rs::scene::Metadata) -> DocNode {
    let mut pairs = Vec::new();
    if let Some(g) = &meta.generator {
        pairs.push(("generator".to_string(), s(g)));
    }
    if let Some(c) = &meta.copyright {
        pairs.push(("copyright".to_string(), s(c)));
    }
    if let Some(sf) = &meta.source_format {
        pairs.push(("source_format".to_string(), s(sf)));
    }
    if !meta.extra.is_empty() {
        let mut extra: Vec<(String, DocNode)> = meta
            .extra
            .iter()
            .map(|(k, v)| (k.clone(), value_to_node(v)))
            .collect();
        extra.sort_by(|a, b| a.0.cmp(&b.0));
        pairs.push(("extra".to_string(), DocNode::Map(extra)));
    }
    DocNode::Map(pairs)
}

fn encode_node(n: &Node) -> DocNode {
    m![
        "id" => i(n.id.0 as i64),
        "name" => s(&n.name),
        "t" => v3(n.transform.translation),
        "r" => v4(n.transform.rotation.to_array()),
        "s" => v3(n.transform.scale),
        "children" => u32a(n.children.iter().map(|c| c.0).collect()),
        "parent" => i(n.parent.map_or(-1, |p| p.0 as i64)),
        "mesh" => i(n.mesh.map_or(-1, |m| m as i64)),
        "camera" => i(n.camera.map_or(-1, |c| c as i64)),
        "light" => i(n.light.map_or(-1, |l| l as i64)),
        "skin" => i(n.skin.map_or(-1, |k| k as i64)),
    ]
}

fn encode_scene_mesh(m: &Mesh) -> DocNode {
    m![
        "name" => s(&m.name),
        "vertices" => encode_vertices(&m.vertices),
        "primitives" => DocNode::Array(
            m.primitives.iter().map(|p| m![
                "topology" => s(topology_str(p.topology)),
                "indices" => u32a(p.indices.clone()),
                "material" => i(p.material_index.map_or(-1, |x| x as i64)),
            ]).collect()
        ),
        "morphs" => encode_morphs(&m.morph_targets),
        "morph_weights" => f32a(m.morph_weights.clone()),
        "bounds" => encode_bounds(m.bounds),
    ]
}

fn encode_scene_material(m: &Material) -> DocNode {
    m![
        "name" => s(&m.name),
        "base_color_factor" => v4(m.base_color_factor),
        "base_color_texture" => encode_texref_opt(&m.base_color_texture),
        "metallic_factor" => f(m.metallic_factor),
        "roughness_factor" => f(m.roughness_factor),
        "metallic_roughness_texture" => encode_texref_opt(&m.metallic_roughness_texture),
        "specular_color" => v3(m.specular_color),
        "specular_color_texture" => encode_texref_opt(&m.specular_color_texture),
        "specular_weight" => f(m.specular_weight),
        "specular_weight_texture" => encode_texref_opt(&m.specular_weight_texture),
        "ior" => f(m.ior),
        "normal_texture" => encode_texref_opt(&m.normal_texture),
        "normal_scale" => f(m.normal_scale),
        "occlusion_texture" => encode_texref_opt(&m.occlusion_texture),
        "occlusion_strength" => f(m.occlusion_strength),
        "emissive_factor" => v3(m.emissive_factor),
        "emissive_texture" => encode_texref_opt(&m.emissive_texture),
        "alpha_mode" => s(alpha_mode_str(m.alpha_mode)),
        "alpha_cutoff" => f(m.alpha_cutoff),
        "double_sided" => DocNode::Bool(m.double_sided),
    ]
}

fn encode_texref_opt(r: &Option<TextureRef>) -> DocNode {
    match r {
        Some(r) => encode_texture_ref(r),
        None => DocNode::Null,
    }
}

fn encode_texture_ref(r: &TextureRef) -> DocNode {
    let mut pairs = vec![
        ("texture".to_string(), i(r.texture_index as i64)),
        ("uv_channel".to_string(), i(r.uv_channel as i64)),
    ];
    if let Some(t) = &r.transform {
        pairs.push(("transform".to_string(), encode_texture_transform(t)));
    }
    DocNode::Map(pairs)
}

fn encode_scene_texture(t: &Texture) -> DocNode {
    m![
        "name" => s(&t.name),
        "image" => i(t.image_index as i64),
        "sampler" => encode_sampler(&t.sampler),
    ]
}

fn encode_scene_image(img: &Image) -> DocNode {
    m![
        "name" => s(&img.name),
        "source" => encode_image_source(&img.source),
    ]
}

fn encode_scene_camera(c: &solid_rs::scene::Camera) -> DocNode {
    m![
        "name" => s(&c.name),
        "projection" => encode_projection(&c.projection),
    ]
}

fn encode_projection(p: &Projection) -> DocNode {
    match p {
        Projection::Perspective(p) => m![
            "perspective" => m![
                "fov_y" => f(p.fov_y),
                "aspect" => opt(p.aspect_ratio.map(|a| f(a))),
                "near" => f(p.z_near),
                "far" => opt(p.z_far.map(|z| f(z))),
            ],
        ],
        Projection::Orthographic(o) => m![
            "orthographic" => m![
                "x_mag" => f(o.x_mag),
                "y_mag" => f(o.y_mag),
                "near" => f(o.z_near),
                "far" => f(o.z_far),
            ],
        ],
    }
}

fn encode_scene_light(l: &Light) -> DocNode {
    let mut pairs = vec![
        ("name".to_string(), s(l.name())),
        ("type".to_string(), s(light_scene_type_str(l))),
        ("color".to_string(), v3(l.color())),
        ("intensity".to_string(), f(l.intensity())),
    ];
    match l {
        Light::Point(pl) => push_opt(&mut pairs, "range", pl.range.map(f)),
        Light::Spot(sl) => {
            push_opt(&mut pairs, "range", sl.range.map(f));
            pairs.push(("inner_cone_angle".to_string(), f(sl.inner_cone_angle)));
            pairs.push(("outer_cone_angle".to_string(), f(sl.outer_cone_angle)));
        }
        Light::Area(al) => {
            pairs.push(("width".to_string(), f(al.width)));
            pairs.push(("height".to_string(), f(al.height)));
        }
        Light::Directional(_) => {}
    }
    DocNode::Map(pairs)
}

fn light_scene_type_str(l: &Light) -> &'static str {
    match l {
        Light::Directional(_) => "directional",
        Light::Point(_) => "point",
        Light::Spot(_) => "spot",
        Light::Area(_) => "area",
    }
}

fn encode_scene_skin(sk: &Skin) -> DocNode {
    m![
        "name" => s(&sk.name),
        "skeleton_root" => i(sk.skeleton_root.map_or(-1, |n| n.0 as i64)),
        "joints" => u32a(sk.joints.iter().map(|j| j.0).collect()),
        "inverse_bind_matrices" => DocNode::Array(
            sk.inverse_bind_matrices.iter().map(|m| mat4(*m)).collect()
        ),
    ]
}

fn encode_scene_animation(a: &Animation) -> DocNode {
    m![
        "name" => s(&a.name),
        "channels" => DocNode::Array(a.channels.iter().map(encode_scene_channel).collect()),
    ]
}

fn encode_scene_channel(c: &AnimationChannel) -> DocNode {
    m![
        "target" => encode_scene_anim_target(&c.target),
        "interpolation" => s(interpolation_str(c.interpolation)),
        "times" => f32a(c.times.clone()),
        "values" => f32a(c.values.clone()),
    ]
}

fn encode_scene_anim_target(t: &AnimationTarget) -> DocNode {
    match t {
        AnimationTarget::Translation(id) => m!["translation" => i(id.0 as i64)],
        AnimationTarget::Rotation(id) => m!["rotation" => i(id.0 as i64)],
        AnimationTarget::Scale(id) => m!["scale" => i(id.0 as i64)],
        AnimationTarget::MorphWeight {
            node_id,
            target_index,
        } => m![
            "morph" => m![
                "node" => i(node_id.0 as i64),
                "target" => i(*target_index as i64),
            ],
        ],
    }
}

// ── Enum string helpers ──────────────────────────────────────────────────────

pub(crate) fn topology_str(t: Topology) -> &'static str {
    match t {
        Topology::TriangleList => "triangle_list",
        Topology::TriangleStrip => "triangle_strip",
        Topology::LineList => "line_list",
        Topology::LineStrip => "line_strip",
        Topology::PointList => "point_list",
        Topology::QuadList => "quad_list",
        Topology::Polygon => "polygon",
    }
}

pub(crate) fn interpolation_str(i: Interpolation) -> &'static str {
    match i {
        Interpolation::Linear => "linear",
        Interpolation::Step => "step",
        Interpolation::CubicSpline => "cubic_spline",
    }
}

pub(crate) fn alpha_mode_str(m: AlphaMode) -> &'static str {
    match m {
        AlphaMode::Opaque => "opaque",
        AlphaMode::Mask => "mask",
        AlphaMode::Blend => "blend",
    }
}

pub(crate) fn wrap_mode_str(w: WrapMode) -> &'static str {
    match w {
        WrapMode::Repeat => "repeat",
        WrapMode::MirroredRepeat => "mirrored_repeat",
        WrapMode::ClampToEdge => "clamp_to_edge",
    }
}

pub(crate) fn filter_mode_str(fm: FilterMode) -> &'static str {
    match fm {
        FilterMode::Nearest => "nearest",
        FilterMode::Linear => "linear",
        FilterMode::NearestMipmapNearest => "nearest_mipmap_nearest",
        FilterMode::LinearMipmapNearest => "linear_mipmap_nearest",
        FilterMode::NearestMipmapLinear => "nearest_mipmap_linear",
        FilterMode::LinearMipmapLinear => "linear_mipmap_linear",
    }
}

pub(crate) fn bone_property_str(p: BoneProperty) -> &'static str {
    match p {
        BoneProperty::Translation => "translation",
        BoneProperty::Rotation => "rotation",
        BoneProperty::Scale => "scale",
    }
}
