//! Writes a [`DocNode`] tree in the ASCII (`.slda`) format.

use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use solid_rs::error::Result;

use crate::tree::{DocNode, SCHEMA_VERSION};

/// Writes `node` (a root map) to `w` in `.slda` format.
pub(crate) fn write<W: Write + ?Sized>(node: &DocNode, w: &mut W) -> Result<()> {
    let pairs = match node {
        DocNode::Map(pairs) => pairs,
        _ => return Err(solid_rs::SolidError::parse("document root must be a map")),
    };
    write!(w, "SLDA {SCHEMA_VERSION}\n")?;
    for (k, v) in pairs {
        write_string(w, k)?;
        write!(w, " ")?;
        write_value(w, v, 1)?;
        write!(w, "\n")?;
    }
    Ok(())
}

fn write_value<W: Write + ?Sized>(w: &mut W, n: &DocNode, depth: usize) -> Result<()> {
    match n {
        DocNode::Null => write!(w, "null")?,
        DocNode::Bool(b) => write!(w, "{b}")?,
        DocNode::Int(i) => write!(w, "{i}")?,
        DocNode::Float(f) => write!(w, "{}", fmt_float(*f))?,
        DocNode::String(s) => write_string(w, s)?,
        DocNode::Bytes(b) => {
            write!(w, "b64\"{}\"", STANDARD.encode(b))?;
        }
        DocNode::Vec2(v) => write!(w, "v2({} {})", fmt_float(v[0] as f64), fmt_float(v[1] as f64))?,
        DocNode::Vec3(v) => write!(
            w,
            "v3({} {} {})",
            fmt_float(v[0] as f64),
            fmt_float(v[1] as f64),
            fmt_float(v[2] as f64)
        )?,
        DocNode::Vec4(v) => write!(
            w,
            "v4({} {} {} {})",
            fmt_float(v[0] as f64),
            fmt_float(v[1] as f64),
            fmt_float(v[2] as f64),
            fmt_float(v[3] as f64)
        )?,
        DocNode::F32Array(a) => write_scalar_array(w, a.iter().map(|x| DocNode::Float(*x as f64)))?,
        DocNode::U32Array(a) => {
            write_scalar_array(w, a.iter().map(|x| DocNode::Int(*x as i64)))?
        }
        DocNode::I32Array(a) => {
            write_scalar_array(w, a.iter().map(|x| DocNode::Int(*x as i64)))?
        }
        DocNode::Array(items) => write_array(w, items, depth)?,
        DocNode::Map(pairs) => write_map(w, pairs, depth)?,
    }
    Ok(())
}

fn write_scalar_array<W: Write + ?Sized, I: Iterator<Item = DocNode>>(w: &mut W, items: I) -> Result<()> {
    write!(w, "[")?;
    let mut first = true;
    for item in items {
        if !first {
            write!(w, " ")?;
        }
        first = false;
        write_value(w, &item, 1)?;
    }
    write!(w, "]")?;
    Ok(())
}

fn write_array<W: Write + ?Sized>(w: &mut W, items: &[DocNode], depth: usize) -> Result<()> {
    if items.iter().all(|n| is_scalar(n)) {
        return write_scalar_array(w, items.iter().cloned());
    }
    if items.is_empty() {
        write!(w, "[ ]")?;
        return Ok(());
    }
    write!(w, "[\n")?;
    for item in items {
        indent(w, depth)?;
        write_value(w, item, depth + 1)?;
        write!(w, "\n")?;
    }
    indent(w, depth - 1)?;
    write!(w, "]")?;
    Ok(())
}

fn write_map<W: Write + ?Sized>(w: &mut W, pairs: &[(String, DocNode)], depth: usize) -> Result<()> {
    if pairs.is_empty() {
        write!(w, "{{}}")?;
        return Ok(());
    }
    write!(w, "{{\n")?;
    for (k, v) in pairs {
        indent(w, depth)?;
        write_string(w, k)?;
        write!(w, " ")?;
        write_value(w, v, depth + 1)?;
        write!(w, "\n")?;
    }
    indent(w, depth - 1)?;
    write!(w, "}}")?;
    Ok(())
}

fn is_scalar(n: &DocNode) -> bool {
    matches!(
        n,
        DocNode::Null
            | DocNode::Bool(_)
            | DocNode::Int(_)
            | DocNode::Float(_)
            | DocNode::String(_)
            | DocNode::Bytes(_)
    )
}

fn indent<W: Write + ?Sized>(w: &mut W, depth: usize) -> Result<()> {
    for _ in 0..depth {
        write!(w, "    ")?;
    }
    Ok(())
}

fn write_string<W: Write + ?Sized>(w: &mut W, s: &str) -> Result<()> {
    write!(w, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(w, "\\\"")?,
            '\\' => write!(w, "\\\\")?,
            '\n' => write!(w, "\\n")?,
            '\r' => write!(w, "\\r")?,
            '\t' => write!(w, "\\t")?,
            c if (c as u32) < 0x20 => write!(w, "\\u{{{:X}}}", c as u32)?,
            c => write!(w, "{c}")?,
        }
    }
    write!(w, "\"")?;
    Ok(())
}

/// Formats a float with a guaranteed decimal point / exponent (so it cannot be
/// mistaken for an integer) and shortest round-trip precision.
fn fmt_float(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x == f64::INFINITY {
        return "inf".to_string();
    }
    if x == f64::NEG_INFINITY {
        return "-inf".to_string();
    }
    // Values that originated as f32 round-trip better printed as f32.
    let f32_candidate = x as f32;
    if f32_candidate.is_finite() && f32_candidate as f64 == x {
        let mut s = format!("{f32_candidate}");
        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
            s.push_str(".0");
        }
        return s;
    }
    let mut s = format!("{x}");
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        s.push_str(".0");
    }
    s
}
