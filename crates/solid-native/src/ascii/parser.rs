//! Parses the ASCII (`.slda`) format into a [`DocNode`] tree.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use solid_rs::SolidError;

use crate::tree::{DocNode, SCHEMA_VERSION};

/// Parses a complete `.slda` byte stream (UTF-8) into a [`DocNode`] map.
pub(crate) fn parse(bytes: &[u8]) -> crate::Result<DocNode> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| SolidError::parse(format!("slda is not valid UTF-8: {e}")))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser { s: text, pos: 0 };

    p.skip_ws()?;
    let header = p.parse_ident()?;
    if header != "SLDA" {
        return Err(SolidError::parse(format!(
            "expected 'SLDA' header, found '{header}'"
        )));
    }
    p.skip_ws()?;
    let version = p.parse_int_literal()?;
    if version != SCHEMA_VERSION {
        return Err(SolidError::parse(format!(
            "unsupported schema version {version} (expected {SCHEMA_VERSION})"
        )));
    }
    p.parse_root_map()
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn eof(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Skips whitespace and `#`-comments.
    fn skip_ws(&mut self) -> crate::Result<()> {
        loop {
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.bump();
                } else {
                    break;
                }
            }
            if self.peek() == Some('#') {
                while let Some(c) = self.peek() {
                    self.bump();
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn expect(&mut self, c: char) -> crate::Result<()> {
        self.skip_ws()?;
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(SolidError::parse(format!(
                "expected '{c}' at byte {}",
                self.pos
            )))
        }
    }

    /// Parses a top-level (brace-less) map.
    fn parse_root_map(&mut self) -> crate::Result<DocNode> {
        let mut pairs = Vec::new();
        loop {
            self.skip_ws()?;
            if self.eof() {
                break;
            }
            let key = self.parse_string()?;
            self.skip_ws()?;
            let value = self.parse_value()?;
            pairs.push((key, value));
        }
        Ok(DocNode::Map(pairs))
    }

    fn parse_value(&mut self) -> crate::Result<DocNode> {
        self.skip_ws()?;
        let c = self
            .peek()
            .ok_or_else(|| SolidError::parse("unexpected end of input"))?;

        match c {
            '{' => self.parse_map(),
            '[' => self.parse_array(),
            '"' => Ok(DocNode::String(self.parse_string()?)),
            '-' | '0'..='9' | '.' => self.parse_number(),
            c if c.is_ascii_alphabetic() || c == '_' => {
                let ident = self.parse_ident()?;
                match ident {
                    "null" => Ok(DocNode::Null),
                    "true" => Ok(DocNode::Bool(true)),
                    "false" => Ok(DocNode::Bool(false)),
                    "nan" => Ok(DocNode::Float(f64::NAN)),
                    "inf" => Ok(DocNode::Float(f64::INFINITY)),
                    "b64" => {
                        self.skip_ws()?;
                        let b64 = self.parse_string()?;
                        let data = STANDARD
                            .decode(b64.as_bytes())
                            .map_err(|e| SolidError::parse(format!("invalid base64: {e}")))?;
                        Ok(DocNode::Bytes(data))
                    }
                    "v2" | "v3" | "v4" => self.parse_vec(ident),
                    other => Err(SolidError::parse(format!("unknown token '{other}'"))),
                }
            }
            other => Err(SolidError::parse(format!(
                "unexpected character '{other}' at byte {}",
                self.pos
            ))),
        }
    }

    fn parse_map(&mut self) -> crate::Result<DocNode> {
        self.expect('{')?;
        let mut pairs = Vec::new();
        loop {
            self.skip_ws()?;
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err(SolidError::parse("unterminated map"));
            }
            let key = self.parse_string()?;
            self.skip_ws()?;
            let value = self.parse_value()?;
            pairs.push((key, value));
        }
        Ok(DocNode::Map(pairs))
    }

    fn parse_array(&mut self) -> crate::Result<DocNode> {
        self.expect('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws()?;
            if self.peek() == Some(']') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err(SolidError::parse("unterminated array"));
            }
            items.push(self.parse_value()?);
        }
        Ok(DocNode::Array(items))
    }

    fn parse_vec(&mut self, kind: &str) -> crate::Result<DocNode> {
        let n = match kind {
            "v2" => 2,
            "v3" => 3,
            "v4" => 4,
            _ => unreachable!(),
        };
        self.expect('(')?;
        let mut parts = Vec::with_capacity(n);
        for _ in 0..n {
            self.skip_ws()?;
            parts.push(self.parse_float_literal()?);
        }
        self.expect(')')?;
        let arr: [f32; 4] = match n {
            2 => [parts[0], parts[1], 0.0, 0.0],
            3 => [parts[0], parts[1], parts[2], 0.0],
            4 => [parts[0], parts[1], parts[2], parts[3]],
            _ => unreachable!(),
        };
        Ok(match n {
            2 => DocNode::Vec2([arr[0], arr[1]]),
            3 => DocNode::Vec3([arr[0], arr[1], arr[2]]),
            _ => DocNode::Vec4(arr),
        })
    }

    fn parse_string(&mut self) -> crate::Result<String> {
        self.skip_ws()?;
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let c = self
                .bump()
                .ok_or_else(|| SolidError::parse("unterminated string"))?;
            match c {
                '"' => break,
                '\\' => {
                    let esc = self
                        .bump()
                        .ok_or_else(|| SolidError::parse("unterminated escape"))?;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            self.expect('{')?;
                            let mut hex = String::new();
                            loop {
                                let h = self.bump().ok_or_else(|| {
                                    SolidError::parse("unterminated unicode escape")
                                })?;
                                if h == '}' {
                                    break;
                                }
                                hex.push(h);
                            }
                            let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                                SolidError::parse(format!("invalid unicode escape '\\u{{{hex}}}'"))
                            })?;
                            out.push(
                                char::from_u32(code).ok_or_else(|| {
                                    SolidError::parse(format!("invalid unicode code point {code}"))
                                })?,
                            );
                        }
                        other => {
                            return Err(SolidError::parse(format!("invalid escape '\\{other}'")))
                        }
                    }
                }
                c => out.push(c),
            }
        }
        Ok(out)
    }

    fn parse_ident(&mut self) -> crate::Result<&'a str> {
        self.skip_ws()?;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                self.bump();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(SolidError::parse(format!(
                "expected an identifier at byte {start}"
            )));
        }
        Ok(&self.s[start..self.pos])
    }

    fn parse_number(&mut self) -> crate::Result<DocNode> {
        self.skip_ws()?;
        // A '-' followed by an alpha is handled as a negative identifier
        // (only `-inf` is produced by the writer).
        if self.peek() == Some('-') {
            let mut rest = self.s[self.pos + 1..].chars();
            if let Some(n) = rest.next() {
                if n.is_ascii_alphabetic() {
                    let ident = self.parse_ident()?;
                    if ident == "-inf" {
                        return Ok(DocNode::Float(f64::NEG_INFINITY));
                    }
                    return Err(SolidError::parse(format!("unknown token '{ident}'")));
                }
            }
        }
        let start = self.pos;
        let mut is_float = false;
        if self.peek() == Some('-') || self.peek() == Some('.') {
            if self.peek() == Some('.') {
                is_float = true;
            }
            self.bump();
        }
        while let Some(c) = self.peek() {
            match c {
                '0'..='9' => {
                    self.bump();
                }
                '.' => {
                    is_float = true;
                    self.bump();
                }
                'e' | 'E' => {
                    is_float = true;
                    self.bump();
                    if matches!(self.peek(), Some('+') | Some('-')) {
                        self.bump();
                    }
                }
                _ => break,
            }
        }
        let lit = &self.s[start..self.pos];
        if lit.is_empty() {
            return Err(SolidError::parse(format!("expected a number at byte {start}")));
        }
        if is_float {
            let v: f64 = lit
                .parse()
                .map_err(|_| SolidError::parse(format!("invalid float '{lit}'")))?;
            Ok(DocNode::Float(v))
        } else {
            let v: i64 = lit
                .parse()
                .map_err(|_| SolidError::parse(format!("invalid int '{lit}'")))?;
            Ok(DocNode::Int(v))
        }
    }

    fn parse_int_literal(&mut self) -> crate::Result<i64> {
        match self.parse_number()? {
            DocNode::Int(i) => Ok(i),
            _ => Err(SolidError::parse("expected an integer")),
        }
    }

    fn parse_float_literal(&mut self) -> crate::Result<f32> {
        match self.parse_number()? {
            DocNode::Int(i) => Ok(i as f32),
            DocNode::Float(f) => Ok(f as f32),
            _ => Err(SolidError::parse("expected a number")),
        }
    }
}
