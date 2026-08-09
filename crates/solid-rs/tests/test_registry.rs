mod common;
use common::*;
use solid_rs::prelude::*;
use std::io::Cursor;

fn batch_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "solidrs_batch_{}_{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ── new / default ─────────────────────────────────────────────────────────────

#[test]
fn registry_new_no_loaders() {
    let r = Registry::new();
    assert_eq!(r.loader_infos().count(), 0);
}

#[test]
fn registry_new_no_savers() {
    let r = Registry::new();
    assert_eq!(r.saver_infos().count(), 0);
}

#[test]
fn registry_default_same_as_new() {
    assert_eq!(Registry::default().loader_infos().count(), 0);
}

// ── register ─────────────────────────────────────────────────────────────────

#[test]
fn registry_register_loader_adds_one() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert_eq!(r.loader_infos().count(), 1);
}

#[test]
fn registry_register_saver_adds_one() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert_eq!(r.saver_infos().count(), 1);
}

#[test]
fn registry_register_chaining() {
    let mut r = Registry::new();
    r.register_loader(MockLoader)
        .register_saver(MockSaver)
        .register_loader(XyzLoader)
        .register_saver(XyzSaver);
    assert_eq!(r.loader_infos().count(), 2);
    assert_eq!(r.saver_infos().count(), 2);
}

// ── loader_by_id ──────────────────────────────────────────────────────────────

#[test]
fn loader_by_id_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_by_id("mock").is_some());
}

#[test]
fn loader_by_id_not_found() {
    let r = Registry::new();
    assert!(r.loader_by_id("obj").is_none());
}

#[test]
fn loader_by_id_case_insensitive() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_by_id("MOCK").is_some());
    assert!(r.loader_by_id("Mock").is_some());
}

// ── loader_for_extension ──────────────────────────────────────────────────────

#[test]
fn loader_for_extension_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_extension("mock").is_some());
}

#[test]
fn loader_for_extension_not_found() {
    let r = Registry::new();
    assert!(r.loader_for_extension("fbx").is_none());
}

#[test]
fn loader_for_extension_case_insensitive() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_extension("MOCK").is_some());
}

// ── saver_by_id ───────────────────────────────────────────────────────────────

#[test]
fn saver_by_id_found() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert!(r.saver_by_id("mock").is_some());
}

#[test]
fn saver_by_id_not_found() {
    let r = Registry::new();
    assert!(r.saver_by_id("gltf").is_none());
}

// ── can_load / can_save ───────────────────────────────────────────────────────

#[test]
fn can_load_extension_true() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.can_load_extension("mock"));
}

#[test]
fn can_load_extension_false() {
    let r = Registry::new();
    assert!(!r.can_load_extension("mock"));
}

#[test]
fn can_save_extension_true() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert!(r.can_save_extension("mock"));
}

#[test]
fn can_save_extension_false() {
    let r = Registry::new();
    assert!(!r.can_save_extension("mock"));
}

// ── load_file errors ──────────────────────────────────────────────────────────

#[test]
fn load_file_no_extension_returns_error() {
    let r = Registry::new();
    let result = r.load_file("model_no_ext");
    assert!(result.is_err());
}

#[test]
fn load_file_unregistered_extension_returns_error() {
    let r = Registry::new();
    let result = r.load_file("model.fbx");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SolidError::UnsupportedFormat(_)
    ));
}

// ── save_file errors ──────────────────────────────────────────────────────────

#[test]
fn save_file_unregistered_extension_returns_error() {
    let r = Registry::new();
    let s = Scene::new();
    let result = r.save_file(&s, "out.fbx");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SolidError::UnsupportedFormat(_)
    ));
}

#[test]
fn save_file_no_extension_returns_error() {
    let r = Registry::new();
    let result = r.save_file(&Scene::new(), "no_extension");
    assert!(result.is_err());
}

// ── load_from ─────────────────────────────────────────────────────────────────

#[test]
fn load_from_known_format_succeeds() {
    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    let data = b"0 0 0\n1 0 0\n0 1 0\n";
    let scene = r
        .load_from(Cursor::new(data), "xyz", &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertex_count(), 3);
}

#[test]
fn load_from_unknown_format_returns_error() {
    let r = Registry::new();
    let e = r.load_from(Cursor::new(b""), "unknown", &LoadOptions::default());
    assert!(matches!(e.unwrap_err(), SolidError::UnsupportedFormat(_)));
}

// ── loader_infos / saver_infos ────────────────────────────────────────────────

#[test]
fn loader_infos_lists_all_registered() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    r.register_loader(XyzLoader);
    let ids: Vec<&str> = r.loader_infos().map(|i| i.id).collect();
    assert!(ids.contains(&"mock"));
    assert!(ids.contains(&"xyz"));
}

#[test]
fn saver_infos_empty_when_none() {
    let r = Registry::new();
    assert_eq!(r.saver_infos().count(), 0);
}

// ── loader_for_mime ───────────────────────────────────────────────────────────

#[test]
fn loader_for_mime_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_mime("model/x-mock").is_some());
}

#[test]
fn loader_for_mime_not_found() {
    let r = Registry::new();
    assert!(r.loader_for_mime("model/obj").is_none());
}

// ── format info on retrieved loader ──────────────────────────────────────────

#[test]
fn loader_format_info_correct_after_lookup() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    let info = r.loader_by_id("mock").unwrap().format_info();
    assert_eq!(info.name, "Mock Format");
    assert!(info.can_load);
}

// ── multiple formats ──────────────────────────────────────────────────────────

#[test]
fn registry_routes_xyz_correctly() {
    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    r.register_loader(MockLoader);
    let l = r.loader_for_extension("xyz").unwrap();
    assert_eq!(l.format_info().id, "xyz");
}

// ── batch load_files / save_files ─────────────────────────────────────────────

fn write_xyz(path: &std::path::Path, lines: &[&str]) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}

#[test]
fn load_files_preserves_order_serially() {
    let dir = batch_dir("serial");
    let a = dir.join("a.xyz");
    let b = dir.join("b.xyz");
    write_xyz(&a, &["0 0 0", "1 0 0", "0 1 0"]);
    write_xyz(&b, &["9 9 9"]);

    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    let paths = [&a, &b];
    let opts = LoadOptions { num_threads: Some(1), ..Default::default() };
    let scenes = r.load_files(&paths, &opts).unwrap();
    assert_eq!(scenes.len(), 2);
    assert_eq!(scenes[0].meshes[0].vertex_count(), 3);
    assert_eq!(scenes[1].meshes[0].vertex_count(), 1);
}

#[test]
fn load_files_parallel_matches_serial() {
    let dir = batch_dir("par");
    let mut paths = Vec::new();
    for i in 0..8 {
        let p = dir.join(format!("m{i}.xyz"));
        write_xyz(&p, &[&format!("{i} 0 0")]);
        paths.push(p);
    }

    let mut r = Registry::new();
    r.register_loader(XyzLoader);

    let serial = r
        .load_files(&paths, &LoadOptions { num_threads: Some(1), ..Default::default() })
        .unwrap();
    let parallel = r
        .load_files(&paths, &LoadOptions { num_threads: Some(4), ..Default::default() })
        .unwrap();

    assert_eq!(serial.len(), parallel.len());
    for (s, p) in serial.iter().zip(&parallel) {
        assert_eq!(s.meshes[0].vertices[0].position, p.meshes[0].vertices[0].position);
    }
    // Order preserved: first file is m0, last is m7.
    assert_eq!(parallel[0].meshes[0].vertices[0].position.x, 0.0);
    assert_eq!(parallel[7].meshes[0].vertices[0].position.x, 7.0);
}

#[test]
fn save_files_round_trips_all_paths() {
    let dir = batch_dir("save");
    let s1 = make_triangle_scene();
    let s2 = make_triangle_scene();
    let p1 = dir.join("one.xyz");
    let p2 = dir.join("two.xyz");

    let mut r = Registry::new();
    r.register_saver(XyzSaver);
    r.register_loader(XyzLoader);

    let scenes = [&s1, &s2];
    let paths = [&p1, &p2];
    let opts = SaveOptions { num_threads: Some(4), ..Default::default() };
    r.save_files(&scenes, &paths, &opts).unwrap();
    assert!(p1.exists());
    assert!(p2.exists());

    // Reload both to confirm they round-tripped.
    let back = r
        .load_files(&[&p1, &p2], &LoadOptions { num_threads: Some(4), ..Default::default() })
        .unwrap();
    assert_eq!(back.len(), 2);
    for s in &back {
        assert_eq!(s.meshes[0].vertex_count(), 3);
    }
}

#[test]
fn save_files_mismatched_lengths_errors() {
    let mut r = Registry::new();
    r.register_saver(XyzSaver);
    let s = make_triangle_scene();
    let e = r
        .save_files(&[&s], &["one.xyz"], &SaveOptions::default());
    assert!(e.is_ok()); // 1 vs 1
    let e2 = r.save_files(&[&s, &s], &["one.xyz"], &SaveOptions::default());
    assert!(e2.is_err());
}

#[test]
fn load_files_reports_failing_path() {
    let dir = batch_dir("err");
    let good = dir.join("good.xyz");
    let bad = dir.join("bad.xyz");
    write_xyz(&good, &["0 0 0"]);
    write_xyz(&bad, &["not a number"]);

    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    let opts = LoadOptions { num_threads: Some(4), ..Default::default() };
    let e = r.load_files(&[&good, &bad], &opts).unwrap_err();
    match e {
        SolidError::Batch { path, source } => {
            assert!(path.ends_with("bad.xyz"), "path was {path:?}");
            assert!(matches!(*source, SolidError::Parse(_)));
        }
        other => panic!("expected Batch error, got {other:?}"),
    }
}

#[cfg(feature = "configurator")]
#[test]
fn load_files_configured_honours_global_threads() {
    use solid_rs::configurator::{keys, OptionValue, OptionValues};

    let dir = batch_dir("conf");
    let paths: Vec<_> = (0..4)
        .map(|i| {
            let p = dir.join(format!("c{i}.xyz"));
            write_xyz(&p, &[&format!("{i} 1 1")]);
            p
        })
        .collect();

    let mut r = Registry::new();
    r.register_loader(XyzLoader);

    let mut values = OptionValues::new();
    values.set(keys::THREADS, OptionValue::Int(4));
    let scenes = r.load_files_configured(&paths, &values).unwrap();
    assert_eq!(scenes.len(), 4);
    assert_eq!(scenes[0].meshes[0].vertices[0].position.x, 0.0);
    assert_eq!(scenes[3].meshes[0].vertices[0].position.x, 3.0);
}

#[cfg(feature = "configurator")]
#[test]
fn options_schema_includes_globals() {
    use solid_rs::configurator::keys;

    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    let schema = r.options_schema_for_extension("xyz").unwrap();
    let ks: Vec<&str> = schema.fields.iter().map(|f| f.key.as_str()).collect();
    assert!(ks.contains(&keys::THREADS));
    assert!(ks.contains(&keys::PARALLEL));
    assert!(ks.contains(&"triangulate"));
}
