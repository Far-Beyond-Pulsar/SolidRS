//! Registry integration: `Loader`/`Saver` registration, format detection and
//! scene round-trips through `.slda` / `.sldb` files.

mod common;
use common::*;

use std::io::Cursor;

use solid_native::{SldaSaver, SldbSaver};
use solid_rs::builder::SceneBuilder;
use solid_rs::geometry::Vertex;
use solid_rs::registry::Registry;
use solid_rs::scene::Mesh;
use solid_rs::traits::{LoadOptions, Loader, Saver, SaveOptions};
use solid_rs::value::Value;

fn registry() -> Registry {
    Registry::new()
        .register_loader(solid_native::SldaLoader)
        .register_loader(solid_native::SldbLoader)
        .register_saver(SldaSaver)
        .register_saver(SldbSaver)
}

fn sample_scene() -> solid_rs::scene::Scene {
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(glam::vec3(0.0, 1.0, 0.0)),
        Vertex::new(glam::vec3(-1.0, -1.0, 0.0)),
        Vertex::new(glam::vec3(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![solid_rs::geometry::Primitive::triangles(
        vec![0, 1, 2],
        None,
    )];

    let mut builder = SceneBuilder::named("Registry Scene");
    let mesh_idx = builder.push_mesh(mesh);
    let root = builder.add_root_node("Root");
    builder.attach_mesh(root, mesh_idx);
    let mut scene = builder.build();
    scene
        .metadata
        .extra
        .insert("author".into(), Value::String("tests".into()));
    scene
}

fn temp_path(ext: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("solid-native-tests");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir.join(format!("roundtrip-{}.{ext}", std::process::id()))
}

#[test]
fn loader_and_saver_registration() {
    let reg = registry();
    assert!(reg.loader_by_id("slda").is_some());
    assert!(reg.loader_by_id("sldb").is_some());
    assert!(reg.saver_by_id("slda").is_some());
    assert!(reg.saver_by_id("sldb").is_some());

    assert!(reg.loader_for_extension("slda").is_some());
    assert!(reg.saver_for_extension("sldb").is_some());
    assert!(reg.loader_for_mime("text/x-slda").is_some());
}

#[test]
fn format_detection() {
    let mut a = Vec::new();
    let mut b = Vec::new();
    solid_native::save_ascii(&minimal_document(), &mut a).unwrap();
    solid_native::save_binary(&minimal_document(), &mut b).unwrap();

    let slda = solid_native::SldaLoader;
    let sldb = solid_native::SldbLoader;
    assert_eq!(slda.detect(&mut Cursor::new(&a[..])), 1.0);
    assert_eq!(sldb.detect(&mut Cursor::new(&b[..])), 1.0);
    assert_eq!(slda.detect(&mut Cursor::new(&b[..])), 0.0);
    assert_eq!(sldb.detect(&mut Cursor::new(&a[..])), 0.0);
    assert_eq!(slda.detect(&mut Cursor::new(b"garbage")), 0.0);
}

#[test]
fn scene_roundtrip_through_registry() {
    let reg = registry();
    let scene = sample_scene();

    for ext in ["slda", "sldb"] {
        let path = temp_path(ext);
        reg.save_file(&scene, &path).expect("registry save");
        let loaded = reg.load_file(&path).expect("registry load");
        std::fs::remove_file(&path).ok();

        assert_eq!(loaded.name, "Registry Scene");
        assert_eq!(loaded.meshes.len(), 1);
        assert_eq!(loaded.meshes[0].vertices.len(), 3);
        assert_eq!(loaded.meshes[0].vertices[0].position, scene.meshes[0].vertices[0].position);
        assert_eq!(
            loaded.metadata.extra["author"],
            Value::String("tests".into())
        );
    }
}

#[test]
fn load_from_by_format_id() {
    let reg = registry();

    let mut bytes = Vec::new();
    SldaSaver
        .save(&sample_scene(), &mut bytes, &SaveOptions::default())
        .unwrap();
    let mut cursor = Cursor::new(bytes);
    let loaded = reg
        .load_from(&mut cursor, "slda", &LoadOptions::default())
        .expect("load_from by format id");
    assert_eq!(loaded.name, "Registry Scene");
    assert_eq!(loaded.meshes.len(), 1);
}

#[test]
fn loader_rejects_wrong_extension() {
    let reg = Registry::new().register_loader(solid_native::SldaLoader);
    assert!(reg.loader_for_extension("sldb").is_none());
    assert!(reg.loader_by_id("sldb").is_none());
}
