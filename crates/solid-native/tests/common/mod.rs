//! Shared helpers for the solid-native integration tests.

#![allow(dead_code)]

use glam::{Mat4, Vec2, Vec3, Vec4};
use solid_native::prims::{
    AnimChannelAsset, AnimTargetAsset, AnimationAsset, Bone, BoneProperty, CameraAsset,
    LightAsset, MaterialAsset, MeshAsset, PrimitiveAsset, SkeletalMeshAsset, SkeletonAsset,
    TextureAsset, TextureBinding,
};
use solid_native::{Prim, PrimData, SolidDocument};
use solid_rs::geometry::{SkinWeights, Vertex};
use solid_rs::scene::Interpolation;
use solid_rs::value::Value;

/// Builds a document exercising every prim kind with valid cross-references:
///
/// `tex-main` (texture) ← `mat-red` (material) ← `tri` (mesh)
/// `skel-root` (skeleton) ← `char` (skeletal mesh) + `anim` (animation)
/// plus a camera and a light.
pub fn sample_document() -> SolidDocument {
    let mut doc = SolidDocument::named("Sample Scene");
    doc.props.insert("engine".into(), Value::String("SolidRS".into()));
    doc.props.insert("version".into(), Value::Int(7));
    doc.props.insert("scale".into(), Value::Float(1.5));
    doc.props.insert("origin".into(), Value::Vec3([1.0, 2.0, 3.0]));
    doc.props.insert(
        "tags".into(),
        Value::Array(vec![
            Value::String("test".into()),
            Value::Bool(true),
            Value::Int(-3),
        ]),
    );

    // 1. Texture prim (embedded bytes).
    let tex = TextureAsset::embedded(
        "albedo",
        "image/png",
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
    );
    doc.push(Prim::texture("tex-main", "Albedo", tex));

    // 2. Material prim referencing the texture.
    let mut mat = MaterialAsset::solid_color(Vec4::new(0.8, 0.2, 0.2, 1.0));
    mat.metallic_factor = 0.1;
    mat.roughness_factor = 0.7;
    mat.base_color_texture = Some(TextureBinding::new("tex-main"));
    mat.double_sided = true;
    doc.push(
        Prim::material("mat-red", "Red Painted", mat)
            .with_prop("shader", Value::String("pbr".into())),
    );

    // 3. Mesh prim.
    let mut mesh = MeshAsset::new();
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.5, 1.0))
            .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 0.0)),
    ];
    mesh.primitives = vec![PrimitiveAsset::triangles(
        vec![0, 1, 2],
        Some("mat-red".into()),
    )];
    mesh.morph_targets.push(solid_rs::scene::MorphTarget {
        name: "smile".into(),
        position_deltas: vec![
            Vec3::new(0.0, 0.1, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ],
        normal_deltas: Vec::new(),
        tangent_deltas: Vec::new(),
    });
    mesh.morph_weights = vec![0.5];
    mesh.compute_bounds();
    doc.push(Prim::mesh("tri", "Triangle", mesh));

    // 4. Skeleton prim.
    let mut skel = SkeletonAsset::new();
    skel.push_bone(Bone::new("Hips"));
    skel.push_bone(Bone::child_of("Spine", 0));
    skel.inverse_bind_matrices = vec![
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, 0.1, 0.0)),
    ];
    doc.push(Prim::skeleton("skel-root", "Humanoid", skel));

    // 5. Skeletal mesh prim skinned to the skeleton.
    let mut skm = SkeletalMeshAsset::new();
    skm.bones = vec!["Hips".into(), "Spine".into()];
    skm.skeleton = Some("skel-root".into());
    skm.mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_skin_weights(SkinWeights {
                joints: [0, 1, 0, 0],
                weights: [0.7, 0.3, 0.0, 0.0],
            }),
        Vertex::new(Vec3::new(0.0, 0.5, 0.0))
            .with_skin_weights(SkinWeights {
                joints: [0, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            }),
    ];
    skm.mesh.primitives = vec![PrimitiveAsset::lines(
        vec![0, 1],
        Some("mat-red".into()),
    )];
    skm.inverse_bind_matrices = vec![Mat4::IDENTITY, Mat4::IDENTITY];
    doc.push(Prim::skeletal_mesh("char", "Character", skm));

    // 6. Animation prim bound to the skeleton and the mesh.
    let mut anim = AnimationAsset::new();
    anim.skeleton = Some("skel-root".into());
    anim.mesh = Some("tri".into());
    anim.push_channel(AnimChannelAsset {
        target: AnimTargetAsset::Bone {
            bone: 1,
            property: BoneProperty::Translation,
        },
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0, 2.0],
        values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    });
    anim.push_channel(AnimChannelAsset {
        target: AnimTargetAsset::MorphWeight { target_index: 0 },
        interpolation: Interpolation::Step,
        times: vec![0.0, 1.0],
        values: vec![0.0, 1.0],
    });
    doc.push(Prim::animation("anim", "Idle", anim));

    // 7. Camera prim.
    doc.push(Prim::camera(
        "cam-main",
        "Main Camera",
        CameraAsset::perspective(),
    ));

    // 8. Light prim.
    let mut light = LightAsset::spot();
    if let LightAsset::Spot { color, intensity, .. } = &mut light {
        *color = Vec3::new(1.0, 0.9, 0.8);
        *intensity = 1200.0;
    }
    doc.push(Prim::light("light-key", "Key Light", light));

    doc
}

/// A sample document whose mesh carries no material reference (so it stays
/// valid even without a material prim), used by the registry tests.
pub fn minimal_document() -> SolidDocument {
    let mut mesh = MeshAsset::new();
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![PrimitiveAsset::triangles(vec![0, 1, 2], None)];
    mesh.compute_bounds();

    let mut doc = SolidDocument::named("Triangle");
    doc.push(Prim::mesh("tri", "Triangle", mesh));
    doc
}

/// The sample document with the `mat-red` material removed, leaving the mesh,
/// skeletal mesh and animation with dangling references.
pub fn document_with_dangling_ref() -> SolidDocument {
    let mut doc = sample_document();
    doc.prims.retain(|p| p.id != "mat-red");
    doc
}

/// Canonical string form of a [`Value`] that is independent of map ordering.
pub fn canonical_value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("bool:{b}"),
        Value::Int(i) => format!("int:{i}"),
        Value::Float(f) => format!("float:{f}"),
        Value::String(s) => format!("str:{s:?}"),
        Value::Vec2(a) => format!("vec2:{a:?}"),
        Value::Vec3(a) => format!("vec3:{a:?}"),
        Value::Vec4(a) => format!("vec4:{a:?}"),
        Value::Bytes(b) => format!("bytes:{b:?}"),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical_value).collect();
            format!("array:[{}]", inner.join(","))
        }
        Value::Map(map) => {
            let mut entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k:?}:{}", canonical_value(v)))
                .collect();
            entries.sort();
            format!("map:{{{}}}", entries.join(","))
        }
    }
}

/// Canonical, order-independent string form of a document.  Used to compare
/// documents field-by-field without relying on `HashMap` iteration order.
pub fn canonical_document(doc: &SolidDocument) -> String {
    let mut props: Vec<String> = doc
        .props
        .iter()
        .map(|(k, v)| format!("{k:?}:{}", canonical_value(v)))
        .collect();
    props.sort();

    let prims: Vec<String> = doc.prims.iter().map(canonical_prim).collect();
    format!(
        "doc {:?}\nprops: {{{}}}\nprims: [{}]",
        doc.name,
        props.join(","),
        prims.join("\n")
    )
}

fn canonical_prim(p: &Prim) -> String {
    let mut props: Vec<String> = p
        .props
        .iter()
        .map(|(k, v)| format!("{k:?}:{}", canonical_value(v)))
        .collect();
    props.sort();
    format!(
        "prim {:?}/{:?}/{:?} props:{{{}}} data:{}",
        p.id,
        p.name,
        p.kind().as_str(),
        props.join(","),
        canonical_data(&p.data)
    )
}

fn canonical_data(d: &PrimData) -> String {
    match d {
        PrimData::Mesh(m) => format!("mesh {}", canonical_mesh(m)),
        PrimData::Skeleton(s) => format!(
            "skeleton bones:{:?} ibm:{:?}",
            s.bones
                .iter()
                .map(|b| format!("{}->{:?}", b.name, b.parent))
                .collect::<Vec<_>>(),
            s.inverse_bind_matrices
        ),
        PrimData::SkeletalMesh(sm) => format!(
            "skeletal_mesh bones:{:?} skeleton:{:?} ibm:{:?} mesh:{}",
            sm.bones,
            sm.skeleton,
            sm.inverse_bind_matrices,
            canonical_mesh(&sm.mesh)
        ),
        PrimData::Material(m) => format!(
            "material base:{:?} metallic:{:?} rough:{:?} base_tex:{:?} double_sided:{:?}",
            m.base_color_factor,
            m.metallic_factor,
            m.roughness_factor,
            m.base_color_texture.as_ref().map(|t| t.texture.clone()),
            m.double_sided
        ),
        PrimData::Texture(t) => {
            let data = match &t.image.source {
                solid_rs::scene::ImageSource::Embedded { mime_type, data } => {
                    format!("{mime_type}:{data:?}")
                }
                solid_rs::scene::ImageSource::Uri(uri) => format!("uri:{uri}"),
            };
            format!("texture {:?} {data}", t.name)
        }
        PrimData::Animation(a) => format!(
            "animation skeleton:{:?} mesh:{:?} duration:{:?} channels:{}",
            a.skeleton,
            a.mesh,
            a.duration,
            a.channels
                .iter()
                .map(|c| format!(
                    "{{{:?} {:?} times:{:?} values:{:?}}}",
                    c.target, c.interpolation, c.times, c.values
                ))
                .collect::<Vec<_>>()
                .join(";")
        ),
        PrimData::Camera(c) => format!("camera projection:{:?}", c.projection),
        PrimData::Light(l) => format!("light {l:?}"),
        PrimData::Scene(s) => format!(
            "scene {:?} meshes:{} vertices:{} nodes:{}",
            s.name,
            s.meshes.len(),
            s.meshes.iter().map(|m| m.vertices.len()).sum::<usize>(),
            s.nodes.len()
        ),
    }
}

fn canonical_mesh(m: &MeshAsset) -> String {
    format!(
        "vertices:{:?} primitives:{:?} morphs:{:?} weights:{:?} bounds:{:?}",
        m.vertices, m.primitives, m.morph_targets, m.morph_weights, m.bounds
    )
}

/// Writes `doc` to ASCII, loads it back, and returns the loaded document.
pub fn ascii_round_trip(doc: &SolidDocument) -> SolidDocument {
    let mut bytes = Vec::new();
    solid_native::save_ascii(doc, &mut bytes).expect("ascii save should succeed");
    let mut slice = bytes.as_slice();
    solid_native::load_ascii(&mut slice).expect("ascii load should succeed")
}

/// Writes `doc` to binary, loads it back, and returns the loaded document.
pub fn binary_round_trip(doc: &SolidDocument) -> SolidDocument {
    let mut bytes = Vec::new();
    solid_native::save_binary(doc, &mut bytes).expect("binary save should succeed");
    let mut slice = bytes.as_slice();
    solid_native::load_binary(&mut slice).expect("binary load should succeed")
}
