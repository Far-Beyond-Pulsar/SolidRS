//! Conversion between `solid-rs` [`Scene`] graphs and [`SolidDocument`]s.
//!
//! The document API ([`crate::save_ascii`] / [`crate::load_ascii`] /
//! [`crate::save_binary`] / [`crate::load_binary`]) is full-fidelity.  The
//! functions here bridge to the `solid-rs` [`Registry`](solid_rs::registry::Registry)
//! path, which works in terms of [`Scene`]s.

use std::collections::HashMap;

use glam::Mat4;

use solid_rs::error::Result;
use solid_rs::extensions::Extensions;
use solid_rs::geometry::Transform;
use solid_rs::scene::{
    Animation, AnimationChannel, AnimationTarget, Node, NodeId, Scene, Skin,
};

use crate::doc::{Prim, PrimData, SolidDocument};
use crate::prims::AnimTargetAsset;

/// Wraps `scene` into a [`SolidDocument`] with a single `scene` prim.
///
/// The scene's `metadata.extra` table becomes the document's top-level
/// `props` table (and vice-versa when loading).
pub fn scene_to_document(scene: &Scene) -> SolidDocument {
    let mut doc = SolidDocument::named(scene.name.clone());
    doc.props = scene.metadata.extra.clone();
    doc.push(Prim::scene("scene", scene.name.clone(), scene.clone()));
    doc
}

/// Converts a [`SolidDocument`] into a [`Scene`].
///
/// If the document contains a `scene` prim, it is returned directly.
/// Otherwise a scene is synthesised from the individual asset prims:
/// meshes, materials, textures, cameras and lights each become top-level
/// nodes; skeletons become skin/joint hierarchies; skeletal meshes are
/// skinned to their referenced skeleton; animations are bound to the
/// generated nodes.
pub fn document_to_scene(doc: &SolidDocument) -> Result<Scene> {
    if let Some(p) = doc.prims.iter().find(|p| matches!(p.data, PrimData::Scene(_))) {
        if let PrimData::Scene(s) = &p.data {
            return Ok(s.clone());
        }
    }
    synthesize_scene(doc)
}

fn synthesize_scene(doc: &SolidDocument) -> Result<Scene> {
    let mut scene = Scene::default();
    scene.name = doc.name.clone();
    scene.metadata.source_format = Some("Solid Native".to_string());

    let mut next_id = 0u32;
    let alloc_id = |next_id: &mut u32| {
        let id = NodeId(*next_id);
        *next_id += 1;
        id
    };

    // 1. Textures + images.
    let mut tex_by_id: HashMap<&str, usize> = HashMap::new();
    for prim in &doc.prims {
        if let PrimData::Texture(t) = &prim.data {
            let image_index = scene.images.len();
            scene.images.push(t.image.clone());
            let texture_index = scene.textures.len();
            scene.textures.push(t.to_solid(image_index));
            tex_by_id.insert(prim.id.as_str(), texture_index);
        }
    }

    // 2. Materials (texture slots resolved by prim ID).
    let mut mat_by_id: HashMap<&str, usize> = HashMap::new();
    for prim in &doc.prims {
        if let PrimData::Material(m) = &prim.data {
            let index = scene.materials.len();
            let mut material = m.to_solid(&|id| tex_by_id.get(id).copied());
            material.name = prim.name.clone();
            scene.materials.push(material);
            mat_by_id.insert(prim.id.as_str(), index);
        }
    }

    // 3. Meshes (with a root node per mesh).
    let mut mesh_by_id: HashMap<&str, (usize, NodeId)> = HashMap::new();
    for prim in &doc.prims {
        if let PrimData::Mesh(m) = &prim.data {
            let mesh_index = scene.meshes.len();
            let mut mesh = m.to_mesh(&|id| mat_by_id.get(id).copied());
            mesh.name = prim.name.clone();
            scene.meshes.push(mesh);

            let node_id = alloc_id(&mut next_id);
            scene.nodes.push(root_node(node_id, &prim.name, Some(mesh_index)));
            scene.roots.push(node_id);
            mesh_by_id.insert(prim.id.as_str(), (mesh_index, node_id));
        }
    }

    // 4. Skeletons → bone nodes + skins.
    //    skeleton id → (skin index, bone node ids, root node id).
    let mut skel_info: HashMap<&str, (usize, Vec<NodeId>, Option<NodeId>)> = HashMap::new();
    for prim in &doc.prims {
        if let PrimData::Skeleton(sk) = &prim.data {
            if sk.bones.is_empty() {
                continue;
            }
            let mut bone_ids = Vec::with_capacity(sk.bones.len());
            for bone in &sk.bones {
                let id = alloc_id(&mut next_id);
                bone_ids.push(id);
                scene.nodes.push(Node {
                    id,
                    name: bone.name.clone(),
                    transform: bone.local_transform.clone(),
                    children: Vec::new(),
                    parent: None,
                    mesh: None,
                    camera: None,
                    light: None,
                    skin: None,
                    extensions: Extensions::new(),
                });
            }
            for (i, bone) in sk.bones.iter().enumerate() {
                if let Some(parent) = bone.parent {
                    if let Some(parent_node) = scene.nodes.get_mut(parent) {
                        parent_node.children.push(bone_ids[i]);
                    }
                    scene.nodes[bone_ids[i].0 as usize].parent = Some(bone_ids[parent]);
                }
            }
            let root = sk
                .bones
                .iter()
                .enumerate()
                .find(|(_, b)| b.parent.is_none())
                .map(|(i, _)| bone_ids[i])
                .or_else(|| bone_ids.first().copied());

            let ibm = if sk.inverse_bind_matrices.is_empty() {
                vec![Mat4::IDENTITY; sk.bones.len()]
            } else {
                sk.inverse_bind_matrices.clone()
            };

            let skin_index = scene.skins.len();
            scene.skins.push(Skin {
                name: prim.name.clone(),
                skeleton_root: root,
                joints: bone_ids.clone(),
                inverse_bind_matrices: ibm,
                extensions: Extensions::new(),
            });
            if let Some(r) = root {
                scene.roots.push(r);
            }
            skel_info.insert(prim.id.as_str(), (skin_index, bone_ids, root));
        }
    }

    // 5. Skeletal meshes: mesh + node skinned to the referenced skeleton.
    let mut skel_mesh_node: HashMap<&str, NodeId> = HashMap::new();
    for prim in &doc.prims {
        if let PrimData::SkeletalMesh(sm) = &prim.data {
            let mesh_index = scene.meshes.len();
            let mut mesh = sm.mesh.to_mesh(&|id| mat_by_id.get(id).copied());
            mesh.name = prim.name.clone();
            scene.meshes.push(mesh);

            let (skin_index, skeleton_root) = sm
                .skeleton
                .as_deref()
                .and_then(|id| skel_info.get(id))
                .map(|(s, _, r)| (Some(*s), r))
                .unwrap_or((None, &None));

            let node_id = alloc_id(&mut next_id);
            let parent = *skeleton_root;
            scene.nodes.push(Node {
                id: node_id,
                name: prim.name.clone(),
                transform: Transform::IDENTITY,
                children: Vec::new(),
                parent,
                mesh: Some(mesh_index),
                camera: None,
                light: None,
                skin: skin_index,
                extensions: Extensions::new(),
            });
            if let Some(p) = parent {
                if let Some(n) = scene.nodes.get_mut(p.0 as usize) {
                    n.children.push(node_id);
                }
            } else {
                scene.roots.push(node_id);
            }
            skel_mesh_node.insert(prim.id.as_str(), node_id);
        }
    }

    // 6. Cameras and lights.
    for prim in &doc.prims {
        match &prim.data {
            PrimData::Camera(cam) => {
                let camera_index = scene.cameras.len();
                let mut camera = cam.to_solid();
                camera.name = prim.name.clone();
                scene.cameras.push(camera);

                let node_id = alloc_id(&mut next_id);
                scene.nodes.push(Node {
                    id: node_id,
                    name: prim.name.clone(),
                    transform: Transform::IDENTITY,
                    children: Vec::new(),
                    parent: None,
                    mesh: None,
                    camera: Some(camera_index),
                    light: None,
                    skin: None,
                    extensions: Extensions::new(),
                });
                scene.roots.push(node_id);
            }
            PrimData::Light(light) => {
                let light_index = scene.lights.len();
                scene.lights.push(light.to_solid(&prim.name));

                let node_id = alloc_id(&mut next_id);
                scene.nodes.push(Node {
                    id: node_id,
                    name: prim.name.clone(),
                    transform: Transform::IDENTITY,
                    children: Vec::new(),
                    parent: None,
                    mesh: None,
                    camera: None,
                    light: Some(light_index),
                    skin: None,
                    extensions: Extensions::new(),
                });
                scene.roots.push(node_id);
            }
            _ => {}
        }
    }

    // 7. Animations.
    let anim_skeleton = |skel_id: &str| skel_info.get(skel_id);
    for prim in &doc.prims {
        if let PrimData::Animation(a) = &prim.data {
            let mut channels = Vec::new();
            for ch in &a.channels {
                let target = match &ch.target {
                    AnimTargetAsset::Bone { bone, property } => {
                        let bone_ids = a
                            .skeleton
                            .as_deref()
                            .and_then(anim_skeleton)
                            .map(|(_, ids, _)| ids)
                            .ok_or_else(|| {
                                solid_rs::SolidError::invalid_ref(format!(
                                    "animation '{}' bone channel needs a skeleton",
                                    prim.id
                                ))
                            })?;
                        let node = bone_ids.get(*bone).copied().ok_or_else(|| {
                            solid_rs::SolidError::invalid_ref(format!(
                                "animation '{}' references missing bone {}",
                                prim.id, bone
                            ))
                        })?;
                        match property {
                            crate::prims::BoneProperty::Translation => {
                                AnimationTarget::Translation(node)
                            }
                            crate::prims::BoneProperty::Rotation => {
                                AnimationTarget::Rotation(node)
                            }
                            crate::prims::BoneProperty::Scale => AnimationTarget::Scale(node),
                        }
                    }
                    AnimTargetAsset::MorphWeight { target_index } => {
                        let node = a
                            .mesh
                            .as_deref()
                            .and_then(|id| {
                                mesh_by_id
                                    .get(id)
                                    .map(|(_, n)| *n)
                                    .or_else(|| skel_mesh_node.get(id).copied())
                            })
                            .ok_or_else(|| {
                                solid_rs::SolidError::invalid_ref(format!(
                                    "animation '{}' morph channel needs a mesh",
                                    prim.id
                                ))
                            })?;
                        AnimationTarget::MorphWeight {
                            node_id: node,
                            target_index: *target_index,
                        }
                    }
                    AnimTargetAsset::Custom(_) => continue,
                };
                channels.push(AnimationChannel {
                    target,
                    interpolation: ch.interpolation,
                    times: ch.times.clone(),
                    values: ch.values.clone(),
                });
            }
            if !channels.is_empty() {
                scene.animations.push(Animation {
                    name: prim.name.clone(),
                    channels,
                    extensions: Extensions::new(),
                });
            }
        }
    }

    // Document props become scene metadata.
    scene.metadata.extra = doc.props.clone();

    Ok(scene)
}

fn root_node(id: NodeId, name: &str, mesh: Option<usize>) -> Node {
    Node {
        id,
        name: name.to_owned(),
        transform: Transform::IDENTITY,
        children: Vec::new(),
        parent: None,
        mesh,
        camera: None,
        light: None,
        skin: None,
        extensions: Extensions::new(),
    }
}
