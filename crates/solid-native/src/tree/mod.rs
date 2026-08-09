//! Format-neutral document tree ([`DocNode`]) shared by the ASCII and binary
//! encoders.
//!
//! The document schema (see [`encode`] / [`decode`]) is written once against
//! [`DocNode`]; the `.slda` and `.sldb` codecs only have to serialise this
//! tree, guaranteeing both encodings carry identical data.

use solid_rs::value::Value;

pub(crate) mod decode;
pub(crate) mod encode;

/// The current on-disk document schema version.
pub(crate) const SCHEMA_VERSION: i64 = 1;

/// A format-neutral, self-describing tree node.
///
/// Numeric arrays ([`F32Array`](DocNode::F32Array),
/// [`U32Array`](DocNode::U32Array), [`I32Array`](DocNode::I32Array)) exist so
/// large geometry buffers can be serialised without per-element boxing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DocNode {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    F32Array(Vec<f32>),
    U32Array(Vec<u32>),
    I32Array(Vec<i32>),
    Array(Vec<DocNode>),
    Map(Vec<(String, DocNode)>),
}

/// Builds a [`DocNode::Map`] from `"key" => value` pairs.
macro_rules! m {
    ($($k:expr => $v:expr),* $(,)?) => {
        crate::tree::DocNode::Map(vec![$(($k.to_string(), $v)),*])
    };
}
pub(crate) use m;

impl DocNode {
    /// Returns `true` for [`DocNode::Null`].
    pub(crate) fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

// ── Value (solid-rs) ⇄ DocNode ───────────────────────────────────────────────

/// Converts a `solid-rs` [`Value`] (used for props tables) into a [`DocNode`].
pub(crate) fn value_to_node(v: &Value) -> DocNode {
    match v {
        Value::Null => DocNode::Null,
        Value::Bool(b) => DocNode::Bool(*b),
        Value::Int(i) => DocNode::Int(*i),
        Value::Float(f) => DocNode::Float(*f),
        Value::String(s) => DocNode::String(s.clone()),
        Value::Vec2(v) => DocNode::Vec2(*v),
        Value::Vec3(v) => DocNode::Vec3(*v),
        Value::Vec4(v) => DocNode::Vec4(*v),
        Value::Bytes(b) => DocNode::Bytes(b.clone()),
        Value::Array(a) => DocNode::Array(a.iter().map(value_to_node).collect()),
        Value::Map(map) => {
            let mut pairs: Vec<(String, DocNode)> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_node(v)))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            DocNode::Map(pairs)
        }
    }
}

/// Converts a [`DocNode`] back into a `solid-rs` [`Value`].
///
/// Every [`DocNode`] variant maps onto a [`Value`] variant, so this is
/// infallible.
pub(crate) fn node_to_value(n: &DocNode) -> Value {
    match n {
        DocNode::Null => Value::Null,
        DocNode::Bool(b) => Value::Bool(*b),
        DocNode::Int(i) => Value::Int(*i),
        DocNode::Float(f) => Value::Float(*f),
        DocNode::String(s) => Value::String(s.clone()),
        DocNode::Bytes(b) => Value::Bytes(b.clone()),
        DocNode::Vec2(v) => Value::Vec2(*v),
        DocNode::Vec3(v) => Value::Vec3(*v),
        DocNode::Vec4(v) => Value::Vec4(*v),
        DocNode::F32Array(a) => Value::Array(a.iter().map(|f| Value::Float(*f as f64)).collect()),
        DocNode::U32Array(a) => {
            Value::Array(a.iter().map(|i| Value::Int(*i as i64)).collect())
        }
        DocNode::I32Array(a) => Value::Array(a.iter().map(|i| Value::Int(*i as i64)).collect()),
        DocNode::Array(a) => Value::Array(a.iter().map(node_to_value).collect()),
        DocNode::Map(pairs) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in pairs {
                map.insert(k.clone(), node_to_value(v));
            }
            Value::Map(map)
        }
    }
}

// ── Decoding accessors ───────────────────────────────────────────────────────

/// Interprets a [`DocNode`] as a map, returning its key/value pairs.
pub(crate) fn as_map(n: &DocNode) -> crate::Result<&[(String, DocNode)]> {
    match n {
        DocNode::Map(pairs) => Ok(pairs),
        _ => Err(solid_rs::SolidError::parse(format!("expected a map, got {}", kind_name(n)))),
    }
}

/// Interprets a [`DocNode`] as an array.
pub(crate) fn as_array(n: &DocNode) -> crate::Result<&[DocNode]> {
    match n {
        DocNode::Array(items) => Ok(items),
        _ => Err(solid_rs::SolidError::parse(format!("expected an array, got {}", kind_name(n)))),
    }
}

/// Fetches a key from a map node.
pub(crate) fn map_get<'a>(n: &'a DocNode, key: &str) -> crate::Result<&'a DocNode> {
    match n {
        DocNode::Map(pairs) => pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .ok_or_else(|| solid_rs::SolidError::parse(format!("missing field '{key}'"))),
        _ => Err(solid_rs::SolidError::parse("expected a map")),
    }
}

/// Fetches an optional key from a map node, treating [`DocNode::Null`] as
/// absent.
pub(crate) fn map_get_opt<'a>(n: &'a DocNode, key: &str) -> crate::Result<Option<&'a DocNode>> {
    match n {
        DocNode::Map(pairs) => Ok(pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .filter(|v| !v.is_null())),
        _ => Err(solid_rs::SolidError::parse("expected a map")),
    }
}

/// Returns a human-readable name for a node's variant (for error messages).
pub(crate) fn kind_name(n: &DocNode) -> &'static str {
    match n {
        DocNode::Null => "null",
        DocNode::Bool(_) => "bool",
        DocNode::Int(_) => "int",
        DocNode::Float(_) => "float",
        DocNode::String(_) => "string",
        DocNode::Bytes(_) => "bytes",
        DocNode::Vec2(_) => "vec2",
        DocNode::Vec3(_) => "vec3",
        DocNode::Vec4(_) => "vec4",
        DocNode::F32Array(_) => "float array",
        DocNode::U32Array(_) => "u32 array",
        DocNode::I32Array(_) => "i32 array",
        DocNode::Array(_) => "array",
        DocNode::Map(_) => "map",
    }
}

/// Reads a required string field.
pub(crate) fn field_str<'a>(n: &'a DocNode, key: &str) -> crate::Result<&'a str> {
    as_str(map_get(n, key)?)
}

/// Reads an optional string field.
pub(crate) fn field_opt_str(n: &DocNode, key: &str) -> crate::Result<Option<String>> {
    match map_get_opt(n, key)? {
        None => Ok(None),
        Some(v) => Ok(Some(as_str(v)?.to_owned())),
    }
}

/// Reads an `i64` field.
pub(crate) fn field_i64(n: &DocNode, key: &str) -> crate::Result<i64> {
    as_i64(map_get(n, key)?)
}

/// Reads an `i64` field with a default.
pub(crate) fn field_i64_or(n: &DocNode, key: &str, default: i64) -> crate::Result<i64> {
    match map_get_opt(n, key)? {
        None => Ok(default),
        Some(v) => as_i64(v),
    }
}

/// Reads a `f32` field.
pub(crate) fn field_f32(n: &DocNode, key: &str) -> crate::Result<f32> {
    as_f32(map_get(n, key)?)
}

/// Reads a `f32` field with a default.
pub(crate) fn field_f32_or(n: &DocNode, key: &str, default: f32) -> crate::Result<f32> {
    match map_get_opt(n, key)? {
        None => Ok(default),
        Some(v) => as_f32(v),
    }
}

/// Reads an optional `f32` field.
pub(crate) fn field_opt_f32(n: &DocNode, key: &str) -> crate::Result<Option<f32>> {
    match map_get_opt(n, key)? {
        None => Ok(None),
        Some(v) => Ok(Some(as_f32(v)?)),
    }
}

/// Reads a `bool` field with a default.
pub(crate) fn field_bool_or(n: &DocNode, key: &str, default: bool) -> crate::Result<bool> {
    match map_get_opt(n, key)? {
        None => Ok(default),
        Some(v) => as_bool(v),
    }
}

pub(crate) fn as_str(n: &DocNode) -> crate::Result<&str> {
    match n {
        DocNode::String(s) => Ok(s),
        _ => Err(solid_rs::SolidError::parse(format!("expected a string, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_i64(n: &DocNode) -> crate::Result<i64> {
    match n {
        DocNode::Int(i) => Ok(*i),
        _ => Err(solid_rs::SolidError::parse(format!("expected an int, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_f64(n: &DocNode) -> crate::Result<f64> {
    match n {
        DocNode::Float(f) => Ok(*f),
        DocNode::Int(i) => Ok(*i as f64),
        _ => Err(solid_rs::SolidError::parse(format!("expected a number, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_f32(n: &DocNode) -> crate::Result<f32> {
    match n {
        DocNode::Float(f) => Ok(*f as f32),
        DocNode::Int(i) => Ok(*i as f32),
        _ => Err(solid_rs::SolidError::parse(format!("expected a number, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_bool(n: &DocNode) -> crate::Result<bool> {
    match n {
        DocNode::Bool(b) => Ok(*b),
        _ => Err(solid_rs::SolidError::parse(format!("expected a bool, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_vec2(n: &DocNode) -> crate::Result<[f32; 2]> {
    match n {
        DocNode::Vec2(v) => Ok(*v),
        _ => Err(solid_rs::SolidError::parse(format!("expected a vec2, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_vec3(n: &DocNode) -> crate::Result<[f32; 3]> {
    match n {
        DocNode::Vec3(v) => Ok(*v),
        _ => Err(solid_rs::SolidError::parse(format!("expected a vec3, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_vec4(n: &DocNode) -> crate::Result<[f32; 4]> {
    match n {
        DocNode::Vec4(v) => Ok(*v),
        _ => Err(solid_rs::SolidError::parse(format!("expected a vec4, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_f32_array(n: &DocNode) -> crate::Result<Vec<f32>> {
    match n {
        DocNode::F32Array(v) => Ok(v.clone()),
        DocNode::Array(items) => items.iter().map(as_f32).collect(),
        _ => Err(solid_rs::SolidError::parse(format!("expected a float array, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_u32_array(n: &DocNode) -> crate::Result<Vec<u32>> {
    match n {
        DocNode::U32Array(v) => Ok(v.clone()),
        DocNode::Array(items) => items
            .iter()
            .map(|x| as_i64(x).map(|i| i as u32))
            .collect(),
        _ => Err(solid_rs::SolidError::parse(format!("expected a u32 array, got {}", kind_name(n)))),
    }
}

pub(crate) fn as_i32_array(n: &DocNode) -> crate::Result<Vec<i32>> {
    match n {
        DocNode::I32Array(v) => Ok(v.clone()),
        DocNode::Array(items) => items
            .iter()
            .map(|x| as_i64(x).map(|i| i as i32))
            .collect(),
        _ => Err(solid_rs::SolidError::parse(format!("expected an i32 array, got {}", kind_name(n)))),
    }
}

/// Reads a 4×4 matrix from a 16-element float array (column-major).
pub(crate) fn as_mat4(n: &DocNode) -> crate::Result<glam::Mat4> {
    let v = as_f32_array(n)?;
    if v.len() != 16 {
        return Err(solid_rs::SolidError::parse("matrix must have 16 elements"));
    }
    let arr: [f32; 16] = v
        .try_into()
        .map_err(|_| solid_rs::SolidError::parse("matrix must have 16 elements"))?;
    Ok(glam::Mat4::from_cols_array(&arr))
}
