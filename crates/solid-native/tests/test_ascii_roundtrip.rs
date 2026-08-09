//! ASCII (`.slda`) codec integration tests.

mod common;
use common::*;

use solid_native::SolidDocument;
use solid_rs::value::Value;

#[test]
fn ascii_magic_header() {
    let mut bytes = Vec::new();
    solid_native::save_ascii(&minimal_document(), &mut bytes).unwrap();
    assert_eq!(&bytes[..5], b"SLDA ");
}

#[test]
fn ascii_full_document_roundtrip() {
    let doc = sample_document();
    let loaded = ascii_round_trip(&doc);

    assert_eq!(
        canonical_document(&doc),
        canonical_document(&loaded),
        "ASCII round-trip must preserve every field"
    );
    assert_eq!(loaded.len(), 8);
}

#[test]
fn ascii_encoding_is_idempotent() {
    let doc = sample_document();

    let mut bytes = Vec::new();
    solid_native::save_ascii(&doc, &mut bytes).unwrap();

    let mut slice = bytes.as_slice();
    let loaded = solid_native::load_ascii(&mut slice).unwrap();

    let mut rebytes = Vec::new();
    solid_native::save_ascii(&loaded, &mut rebytes).unwrap();
    assert_eq!(
        bytes, rebytes,
        "saving a loaded document must produce byte-identical output"
    );
}

#[test]
fn ascii_document_props_survive() {
    let doc = sample_document();
    let loaded = ascii_round_trip(&doc);

    assert_eq!(loaded.name, "Sample Scene");
    assert_eq!(loaded.props["engine"], Value::String("SolidRS".into()));
    assert_eq!(loaded.props["version"], Value::Int(7));
    assert_eq!(loaded.props["scale"], Value::Float(1.5));
    assert_eq!(loaded.props["origin"], Value::Vec3([1.0, 2.0, 3.0]));
    assert_eq!(
        loaded.props["tags"],
        Value::Array(vec![
            Value::String("test".into()),
            Value::Bool(true),
            Value::Int(-3),
        ])
    );
}

#[test]
fn ascii_prim_props_and_kind_survive() {
    let loaded = ascii_round_trip(&sample_document());

    let mat = loaded.find("mat-red").expect("material prim");
    assert_eq!(mat.name, "Red Painted");
    assert_eq!(mat.kind().as_str(), "material");
    assert_eq!(mat.prop("shader"), Some(&Value::String("pbr".into())));

    let mesh = loaded.find("tri").expect("mesh prim");
    assert_eq!(mesh.kind().as_str(), "mesh");
    assert_eq!(mesh.prop("nope"), None);

    let light = loaded.find("light-key").expect("light prim");
    assert_eq!(light.kind().as_str(), "light");
    assert_eq!(loaded.len(), 8);
}

#[test]
fn ascii_rejects_garbage() {
    let mut input: &[u8] = b"this is definitely not a solid document";
    let err = solid_native::load_ascii(&mut input);
    assert!(err.is_err(), "garbage input must fail to load");
}

#[test]
fn ascii_empty_document_roundtrip() {
    let doc = SolidDocument::named("Empty");
    let loaded = ascii_round_trip(&doc);
    assert_eq!(loaded.name, "Empty");
    assert!(loaded.is_empty());
}
