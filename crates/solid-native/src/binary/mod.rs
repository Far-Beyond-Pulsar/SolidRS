//! The binary (`.sldb`) encoding.
//!
//! Layout:
//!
//! ```text
//! magic    "SLDB"                     (4 bytes)
//! version  u8 = 1                     (1 byte)
//! reserved [u8; 3] = [0; 3]           (3 bytes)
//! node     <a single DocNode tree>
//! ```
//!
//! Nodes use a one-byte tag followed by a payload:
//!
//! | Tag | Node            | Payload                                |
//! |-----|-----------------|----------------------------------------|
//! | 0   | Null            | —                                      |
//! | 1   | Bool(false)     | —                                      |
//! | 2   | Bool(true)      | —                                      |
//! | 3   | Int             | zigzag-varint                          |
//! | 4   | Float           | f64 LE                                 |
//! | 5   | String          | varint len + UTF-8                     |
//! | 6   | Bytes           | varint len + bytes                     |
//! | 7   | Vec2            | 2 × f32 LE                             |
//! | 8   | Vec3            | 3 × f32 LE                             |
//! | 9   | Vec4            | 4 × f32 LE                             |
//! | 10  | F32Array        | varint count + count × f32 LE          |
//! | 11  | U32Array        | varint count + count × u32 LE          |
//! | 12  | I32Array        | varint count + count × i32 LE          |
//! | 13  | Array           | varint count + count × node            |
//! | 14  | Map             | varint count + count × (key, node)     |
//!
//! All integers are little-endian; lengths and counts use unsigned LEB128
//! varints.

use std::io::{Read, Write};

use solid_rs::SolidError;

use crate::tree::DocNode;

const TAG_NULL: u8 = 0;
const TAG_BOOL_FALSE: u8 = 1;
const TAG_BOOL_TRUE: u8 = 2;
const TAG_INT: u8 = 3;
const TAG_FLOAT: u8 = 4;
const TAG_STRING: u8 = 5;
const TAG_BYTES: u8 = 6;
const TAG_VEC2: u8 = 7;
const TAG_VEC3: u8 = 8;
const TAG_VEC4: u8 = 9;
const TAG_F32_ARRAY: u8 = 10;
const TAG_U32_ARRAY: u8 = 11;
const TAG_I32_ARRAY: u8 = 12;
const TAG_ARRAY: u8 = 13;
const TAG_MAP: u8 = 14;

/// Writes `node` (a root map) to `w` in `.sldb` format.
pub(crate) fn write<W: Write + ?Sized>(node: &DocNode, w: &mut W) -> crate::Result<()> {
    w.write_all(b"SLDB")?;
    w.write_all(&[1, 0, 0, 0])?;
    write_node(node, w)
}

/// Reads a `.sldb` byte stream into a [`DocNode`] tree.
pub(crate) fn read(bytes: &[u8]) -> crate::Result<DocNode> {
    let mut r = bytes;
    if r.len() < 8 {
        return Err(SolidError::parse("sldb file is too short"));
    }
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != b"SLDB" {
        return Err(SolidError::parse("bad sldb magic"));
    }
    let mut version = [0u8; 1];
    r.read_exact(&mut version)?;
    if version[0] != 1 {
        return Err(SolidError::parse(format!(
            "unsupported sldb version {}",
            version[0]
        )));
    }
    r.read_exact(&mut [0u8; 3])?;
    let node = read_node(&mut r)?;
    if !r.is_empty() {
        return Err(SolidError::parse("trailing bytes after sldb document"));
    }
    Ok(node)
}

fn write_node<W: Write + ?Sized>(n: &DocNode, w: &mut W) -> crate::Result<()> {
    match n {
        DocNode::Null => w.write_all(&[TAG_NULL])?,
        DocNode::Bool(b) => w.write_all(&[if *b { TAG_BOOL_TRUE } else { TAG_BOOL_FALSE }])?,
        DocNode::Int(i) => {
            w.write_all(&[TAG_INT])?;
            w_var(w, zigzag(*i))?;
        }
        DocNode::Float(f) => {
            w.write_all(&[TAG_FLOAT])?;
            w.write_all(&f.to_le_bytes())?;
        }
        DocNode::String(s) => {
            w.write_all(&[TAG_STRING])?;
            write_bytes(w, s.as_bytes())?;
        }
        DocNode::Bytes(b) => {
            w.write_all(&[TAG_BYTES])?;
            write_bytes(w, b)?;
        }
        DocNode::Vec2(v) => {
            w.write_all(&[TAG_VEC2])?;
            write_f32s(w, v)?;
        }
        DocNode::Vec3(v) => {
            w.write_all(&[TAG_VEC3])?;
            write_f32s(w, v)?;
        }
        DocNode::Vec4(v) => {
            w.write_all(&[TAG_VEC4])?;
            write_f32s(w, v)?;
        }
        DocNode::F32Array(a) => {
            w.write_all(&[TAG_F32_ARRAY])?;
            write_count(w, a.len())?;
            for x in a {
                w.write_all(&x.to_le_bytes())?;
            }
        }
        DocNode::U32Array(a) => {
            w.write_all(&[TAG_U32_ARRAY])?;
            write_count(w, a.len())?;
            for x in a {
                w.write_all(&x.to_le_bytes())?;
            }
        }
        DocNode::I32Array(a) => {
            w.write_all(&[TAG_I32_ARRAY])?;
            write_count(w, a.len())?;
            for x in a {
                w.write_all(&x.to_le_bytes())?;
            }
        }
        DocNode::Array(items) => {
            w.write_all(&[TAG_ARRAY])?;
            write_count(w, items.len())?;
            for item in items {
                write_node(item, w)?;
            }
        }
        DocNode::Map(pairs) => {
            w.write_all(&[TAG_MAP])?;
            write_count(w, pairs.len())?;
            for (k, v) in pairs {
                write_bytes(w, k.as_bytes())?;
                write_node(v, w)?;
            }
        }
    }
    Ok(())
}

fn read_node<R: Read>(r: &mut R) -> crate::Result<DocNode> {
    let mut tag = [0u8; 1];
    r.read_exact(&mut tag)?;
    match tag[0] {
        TAG_NULL => Ok(DocNode::Null),
        TAG_BOOL_FALSE => Ok(DocNode::Bool(false)),
        TAG_BOOL_TRUE => Ok(DocNode::Bool(true)),
        TAG_INT => Ok(DocNode::Int(unzigzag(read_var(r)?))),
        TAG_FLOAT => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b)?;
            Ok(DocNode::Float(f64::from_le_bytes(b)))
        }
        TAG_STRING => Ok(DocNode::String(read_string(r)?)),
        TAG_BYTES => Ok(DocNode::Bytes(read_bytes_buf(r)?)),
        TAG_VEC2 => {
            let v = read_f32s(r, 2)?;
            Ok(DocNode::Vec2([v[0], v[1]]))
        }
        TAG_VEC3 => {
            let v = read_f32s(r, 3)?;
            Ok(DocNode::Vec3([v[0], v[1], v[2]]))
        }
        TAG_VEC4 => {
            let v = read_f32s(r, 4)?;
            Ok(DocNode::Vec4(v))
        }
        TAG_F32_ARRAY => {
            let n = read_count(r)?;
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                let mut b = [0u8; 4];
                r.read_exact(&mut b)?;
                a.push(f32::from_le_bytes(b));
            }
            Ok(DocNode::F32Array(a))
        }
        TAG_U32_ARRAY => {
            let n = read_count(r)?;
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                let mut b = [0u8; 4];
                r.read_exact(&mut b)?;
                a.push(u32::from_le_bytes(b));
            }
            Ok(DocNode::U32Array(a))
        }
        TAG_I32_ARRAY => {
            let n = read_count(r)?;
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                let mut b = [0u8; 4];
                r.read_exact(&mut b)?;
                a.push(i32::from_le_bytes(b));
            }
            Ok(DocNode::I32Array(a))
        }
        TAG_ARRAY => {
            let n = read_count(r)?;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(read_node(r)?);
            }
            Ok(DocNode::Array(items))
        }
        TAG_MAP => {
            let n = read_count(r)?;
            let mut pairs = Vec::with_capacity(n);
            for _ in 0..n {
                let k = read_string(r)?;
                let v = read_node(r)?;
                pairs.push((k, v));
            }
            Ok(DocNode::Map(pairs))
        }
        other => Err(SolidError::parse(format!("unknown sldb node tag {other}"))),
    }
}

fn write_f32s<W: Write + ?Sized>(w: &mut W, v: &[f32]) -> crate::Result<()> {
    for x in v {
        w.write_all(&x.to_le_bytes())?;
    }
    Ok(())
}

fn read_f32s<R: Read>(r: &mut R, n: usize) -> crate::Result<[f32; 4]> {
    let mut out = [0.0f32; 4];
    for slot in out.iter_mut().take(n) {
        let mut b = [0u8; 4];
        r.read_exact(&mut b)?;
        *slot = f32::from_le_bytes(b);
    }
    Ok(out)
}

fn write_bytes<W: Write + ?Sized>(w: &mut W, b: &[u8]) -> crate::Result<()> {
    write_count(w, b.len())?;
    w.write_all(b)?;
    Ok(())
}

fn read_bytes_buf<R: Read>(r: &mut R) -> crate::Result<Vec<u8>> {
    let n = read_count(r)?;
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn read_string<R: Read>(r: &mut R) -> crate::Result<String> {
    let b = read_bytes_buf(r)?;
    String::from_utf8(b).map_err(|e| SolidError::parse(format!("invalid UTF-8: {e}")))
}

/// Writes an unsigned LEB128 varint.
fn write_count<W: Write + ?Sized>(w: &mut W, n: usize) -> crate::Result<()> {
    w_var(w, n as u64)
}

/// Reads an unsigned LEB128 varint.
fn read_count<R: Read>(r: &mut R) -> crate::Result<usize> {
    Ok(read_var(r)? as usize)
}

fn w_var<W: Write + ?Sized>(w: &mut W, mut v: u64) -> crate::Result<()> {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        w.write_all(&[byte])?;
        if v == 0 {
            break;
        }
    }
    Ok(())
}

fn read_var<R: Read>(r: &mut R) -> crate::Result<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let mut byte = [0u8; 1];
        r.read_exact(&mut byte)?;
        let b = byte[0];
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(SolidError::parse("varint too long"));
        }
    }
    Ok(result)
}

fn zigzag(i: i64) -> u64 {
    ((i << 1) ^ (i >> 63)) as u64
}

fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}
