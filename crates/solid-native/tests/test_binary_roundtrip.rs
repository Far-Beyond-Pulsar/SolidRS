//! Binary (`.sldb`) codec integration tests.

mod common;
use common::*;

use solid_native::SolidDocument;

#[test]
fn binary_magic_header() {
    let mut bytes = Vec::new();
    solid_native::save_binary(&minimal_document(), &mut bytes).unwrap();
    assert_eq!(&bytes[..4], b"SLDB");
}

#[test]
fn binary_full_document_roundtrip() {
    let doc = sample_document();
    let loaded = binary_round_trip(&doc);

    assert_eq!(
        canonical_document(&doc),
        canonical_document(&loaded),
        "binary round-trip must preserve every field"
    );
    assert_eq!(loaded.len(), 8);
}

#[test]
fn binary_encoding_is_idempotent() {
    let doc = sample_document();

    let mut bytes = Vec::new();
    solid_native::save_binary(&doc, &mut bytes).unwrap();

    let mut slice = bytes.as_slice();
    let loaded = solid_native::load_binary(&mut slice).unwrap();

    let mut rebytes = Vec::new();
    solid_native::save_binary(&loaded, &mut rebytes).unwrap();
    assert_eq!(
        bytes, rebytes,
        "saving a loaded document must produce byte-identical output"
    );
}

#[test]
fn binary_is_compact_relative_to_ascii() {
    let doc = sample_document();
    let mut a = Vec::new();
    let mut b = Vec::new();
    solid_native::save_ascii(&doc, &mut a).unwrap();
    solid_native::save_binary(&doc, &mut b).unwrap();
    assert!(
        b.len() < a.len(),
        "binary should be more compact than ASCII ({} vs {})",
        b.len(),
        a.len()
    );
}

#[test]
fn cross_encoding_consistency() {
    // Both encodings must carry identical data: decoding each and re-encoding
    // with either codec yields the same bytes.
    let doc = sample_document();

    let mut a_bytes = Vec::new();
    let mut b_bytes = Vec::new();
    solid_native::save_ascii(&doc, &mut a_bytes).unwrap();
    solid_native::save_binary(&doc, &mut b_bytes).unwrap();

    let a_loaded = ascii_round_trip(&doc);
    let b_loaded = binary_round_trip(&doc);
    assert_eq!(
        canonical_document(&a_loaded),
        canonical_document(&b_loaded),
        "ASCII and binary decodings must agree"
    );

    // Re-encoding the binary-loaded document in ASCII matches the original
    // ASCII bytes, and vice-versa.
    let mut reb_from_b = Vec::new();
    solid_native::save_ascii(&b_loaded, &mut reb_from_b).unwrap();
    assert_eq!(a_bytes, reb_from_b);

    let mut reb_from_a = Vec::new();
    solid_native::save_binary(&a_loaded, &mut reb_from_a).unwrap();
    assert_eq!(b_bytes, reb_from_a);
}

#[test]
fn binary_rejects_garbage() {
    let mut input: &[u8] = b"definitely not a binary document";
    let err = solid_native::load_binary(&mut input);
    assert!(err.is_err(), "garbage input must fail to load");
}

#[test]
fn binary_empty_document_roundtrip() {
    let doc = SolidDocument::named("Empty");
    let loaded = binary_round_trip(&doc);
    assert_eq!(loaded.name, "Empty");
    assert!(loaded.is_empty());
}
