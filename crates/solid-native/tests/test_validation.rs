//! Document validation: duplicate IDs and dangling cross-prim references must
//! be rejected, both by `SolidDocument::validate` and by the save path.

mod common;
use common::*;

use solid_native::SolidDocument;

#[test]
fn valid_document_validates() {
    assert!(sample_document().validate().is_ok());
    assert!(minimal_document().validate().is_ok());
}

#[test]
fn duplicate_prim_ids_rejected() {
    let mut doc = sample_document();
    let duplicate = doc.find("mat-red").cloned().unwrap();
    doc.push(duplicate);
    assert!(doc.validate().is_err());
}

#[test]
fn dangling_material_reference_rejected() {
    let doc = document_with_dangling_ref();
    assert!(doc.validate().is_err());
}

#[test]
fn dangling_reference_rejected_at_save_time() {
    let doc = document_with_dangling_ref();

    let mut ascii_out = Vec::new();
    assert!(
        solid_native::save_ascii(&doc, &mut ascii_out).is_err(),
        "saving a document with a dangling reference must fail"
    );

    let mut binary_out = Vec::new();
    assert!(
        solid_native::save_binary(&doc, &mut binary_out).is_err(),
        "saving a document with a dangling reference must fail"
    );
}

#[test]
fn duplicate_prim_ids_rejected_at_save_time() {
    let mut doc = minimal_document();
    let duplicate = doc.find("tri").cloned().unwrap();
    doc.push(duplicate);

    let mut out = Vec::new();
    assert!(solid_native::save_ascii(&doc, &mut out).is_err());
}

#[test]
fn material_without_texture_prim_is_valid() {
    // A material referencing an unknown texture is invalid…
    let mut doc = minimal_document();
    doc.push(
        solid_native::Prim::material(
            "mat",
            "Mat",
            solid_native::prims::MaterialAsset::solid_color(glam::Vec4::ONE),
        )
        .with_prop("p".into(), 1_i64.into()),
    );
    assert!(doc.validate().is_ok());

    // …but once it binds a texture that does not exist, validation fails.
    let mut doc2 = doc.clone();
    if let solid_native::PrimData::Material(m) = &mut doc2.prims_mut()[1].data {
        m.base_color_texture = Some(solid_native::prims::TextureBinding::new("no-such-tex"));
    }
    assert!(doc2.validate().is_err());
}
