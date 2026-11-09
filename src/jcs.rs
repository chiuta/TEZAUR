//! JSON Canonicalization (RFC 8785) for receipts: a strict JSON reader that keeps key order and rejects
//! duplicate keys and lone surrogates, and a writer that emits the one canonical byte form.

use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    #[allow(dead_code)]
    pub fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(v) => v.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }
    /// Remove a property from an object, returning it.
    pub fn take(&mut self, k: &str) -> Option<J> {
        if let J::Obj(v) = self
            && let Some(i) = v.iter().position(|(n, _)| n == k)
        {
            return Some(v.remove(i).1);
        }
        None
    }
}

pub fn from_value(v: &serde_json::Value) -> Result<J> {
    use serde_json::Value as V;
    Ok(match v {
        V::Null => J::Null,
        V::Bool(b) => J::Bool(*b),
        V::Number(n) => J::Num(
            n.as_f64()
                .ok_or_else(|| anyhow::anyhow!("number out of range"))?,
        ),
        V::String(s) => J::Str(s.clone()),
        V::Array(a) => J::Arr(a.iter().map(from_value).collect::<Result<_>>()?),
        V::Object(o) => J::Obj(
            o.iter()
                .map(|(k, v)| Ok((k.clone(), from_value(v)?)))
                .collect::<Result<_>>()?,
        ),
    })
}

// ------------------------------------------------------------------ reader

pub fn parse(text: &str) -> Result<J> {
    let mut p = P {
        b: text.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        bail!("trailing characters after the JSON value");
    }
    Ok(v)
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<()> {
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            bail!("expected '{}' at byte {}", c as char, self.i)
        }
    }
    fn lit(&mut self, s: &str, v: J) -> Result<J> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            bail!("invalid literal at byte {}", self.i)
        }
    }
    fn value(&mut self, depth: usize) -> Result<J> {
        if depth > 64 {
            bail!("nesting too deep");
        }
        match self.b.get(self.i) {
            Some(b'n') => self.lit("null", J::Null),
            Some(b't') => self.lit("true", J::Bool(true)),
            Some(b'f') => self.lit("false", J::Bool(false)),
            Some(b'"') => Ok(J::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(J::Arr(v));
                }
                loop {
                    self.ws();
                    v.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(J::Arr(v));
                        }
                        _ => bail!("expected ',' or ']' at byte {}", self.i),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut v: Vec<(String, J)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(J::Obj(v));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    if v.iter().any(|(n, _)| *n == k) {
                        bail!("duplicate property name {k:?}");
                    }
                    self.ws();
                    self.eat(b':')?;
                    self.ws();
                    v.push((k, self.value(depth + 1)?));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(J::Obj(v));
                        }
                        _ => bail!("expected ',' or '}}' at byte {}", self.i),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let s = self.i;
                self.i += 1;
                while self.i < self.b.len()
                    && matches!(
                        self.b[self.i],
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'
                    )
                {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.b[s..self.i])?;
                let n: f64 = t.parse().map_err(|_| anyhow::anyhow!("bad number {t:?}"))?;
                if !n.is_finite() {
                    bail!("number out of range: {t}");
                }
                Ok(J::Num(n))
            }
            _ => bail!("unexpected input at byte {}", self.i),
        }
    }
    fn hex4(&mut self) -> Result<u32> {
        let s = self
            .b
            .get(self.i..self.i + 4)
            .ok_or_else(|| anyhow::anyhow!("short \\u escape"))?;
        let t = std::str::from_utf8(s)?;
        self.i += 4;
        u32::from_str_radix(t, 16).map_err(|_| anyhow::anyhow!("bad \\u escape"))
    }
    fn string(&mut self) -> Result<String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let c = *self
                .b
                .get(self.i)
                .ok_or_else(|| anyhow::anyhow!("unterminated string"))?;
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self
                        .b
                        .get(self.i)
                        .ok_or_else(|| anyhow::anyhow!("bad escape"))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let u = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&u) {
                                if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                    bail!("lone surrogate in string");
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    bail!("lone surrogate in string");
                                }
                                0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&u) {
                                bail!("lone surrogate in string");
                            } else {
                                u
                            };
                            out.push(
                                char::from_u32(cp)
                                    .ok_or_else(|| anyhow::anyhow!("bad code point"))?,
                            );
                        }
                        _ => bail!("bad escape \\{}", e as char),
                    }
                }
                0..=0x1f => bail!("control character in string"),
                _ => {
                    // copy one UTF-8 scalar (input is a &str, so it is valid)
                    let rest = std::str::from_utf8(&self.b[self.i..])?;
                    let ch = rest.chars().next().unwrap();
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }
}

// ------------------------------------------------------------------ writer

/// Canonical bytes. `ints_only` is the receipt-v2 rule: numbers must be integers below 2^53 in magnitude.
pub fn canonical(v: &J, ints_only: bool) -> Result<Vec<u8>> {
    let mut s = String::new();
    write(v, ints_only, &mut s)?;
    Ok(s.into_bytes())
}

fn write(v: &J, ints_only: bool, out: &mut String) -> Result<()> {
    match v {
        J::Null => out.push_str("null"),
        J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        J::Num(n) => {
            if ints_only && (n.fract() != 0.0 || n.abs() >= 9007199254740992.0) {
                bail!("receipt numbers must be integers below 2^53; put {n} in a string");
            }
            out.push_str(&number(*n)?);
        }
        J::Str(s) => string(s, out),
        J::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, ints_only, out)?;
            }
            out.push(']');
        }
        J::Obj(o) => {
            let mut keys: Vec<(&String, &J)> = o.iter().map(|(k, v)| (k, v)).collect();
            keys.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            for w in keys.windows(2) {
                if w[0].0 == w[1].0 {
                    bail!("duplicate property name {:?}", w[0].0);
                }
            }
            out.push('{');
            for (i, (k, x)) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(k, out);
                out.push(':');
                write(x, ints_only, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// ECMAScript Number-to-string for a finite double (shortest round-trip digits).
pub fn number(n: f64) -> Result<String> {
    if !n.is_finite() {
        bail!("NaN and Infinity cannot be represented");
    }
    if n == 0.0 {
        return Ok("0".into());
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let e = format!("{:e}", n.abs()); // shortest digits, e.g. "1.2345e-7"
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let nn = exp + 1;
    let body = if k <= nn && nn <= 21 {
        format!("{digits}{}", "0".repeat((nn - k) as usize))
    } else if 0 < nn && nn <= 21 {
        format!("{}.{}", &digits[..nn as usize], &digits[nn as usize..])
    } else if -6 < nn && nn <= 0 {
        format!("0.{}{digits}", "0".repeat((-nn) as usize))
    } else {
        let m = if k == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!(
            "{m}e{}{}",
            if nn - 1 < 0 { "-" } else { "+" },
            (nn - 1).abs()
        )
    };
    Ok(format!("{sign}{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon(s: &str) -> String {
        String::from_utf8(canonical(&parse(s).unwrap(), false).unwrap()).unwrap()
    }

    #[test]
    fn utf16_key_order_differs_from_utf8() {
        let src = r#"{"\u20ac":"Euro Sign","\r":"Carriage Return","\ufb33":"Hebrew Dalet With Dagesh","1":"One","\ud83d\ude00":"Emoji","\u0080":"Control","\u00f6":"Latin o diaeresis"}"#;
        let v = parse(src).unwrap();
        let c = String::from_utf8(canonical(&v, false).unwrap()).unwrap();
        let order = [
            "\"\\r\":",
            "\"1\":",
            "\"\u{80}\":",
            "\"\u{f6}\":",
            "\"\u{20ac}\":",
            "\"\u{1F600}\":",
            "\"\u{fb33}\":",
        ];
        let mut at = 0;
        for k in order {
            let p = c[at..]
                .find(k)
                .unwrap_or_else(|| panic!("{k} out of order in {c}"));
            at += p + k.len();
        }
        // UTF-8 byte order would put U+FB33 before the emoji, so naive output hashes differently
        let J::Obj(o) = &v else { unreachable!() };
        let mut by_bytes: Vec<&str> = o.iter().map(|(k, _)| k.as_str()).collect();
        by_bytes.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(&by_bytes[5..], ["\u{fb33}", "\u{1F600}"]);
        let naive: String = by_bytes.concat();
        let canon_keys: String = order
            .iter()
            .map(|k| k.trim_matches(|c| c == '"' || c == ':'))
            .collect::<String>()
            .replace("\\r", "\r");
        assert_ne!(naive, canon_keys);
    }

    #[test]
    fn rfc_example() {
        let src = "{\n \"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],\n \"string\": \"\\u20ac$\\u000F\\u000aA'\\u0042\\u0022\\u005c\\\\\\\"\\/\",\n \"literals\": [null, true, false]\n}";
        assert_eq!(
            canon(src),
            "{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"€$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}"
        );
    }

    #[test]
    fn number_forms() {
        let f = |bits: u64| f64::from_bits(bits);
        assert_eq!(number(f(0x4340000000000000)).unwrap(), "9007199254740992");
        assert_eq!(number(f(0x444b1ae4d6e2ef50)).unwrap(), "1e+21");
        assert_eq!(number(f(0x3eb0c6f7a0b5ed8d)).unwrap(), "0.000001");
        assert_eq!(number(f(0x8000000000000000)).unwrap(), "0");
        assert!(number(f(0x7ff0000000000000)).is_err());
        assert!(number(f64::NAN).is_err());
        assert_eq!(number(-1.5).unwrap(), "-1.5");
        assert_eq!(
            number(123456789012345680000.0).unwrap(),
            "123456789012345680000"
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse(r#"{"a":1,"a":2}"#).is_err());
        assert!(parse(r#""\ud800""#).is_err());
        assert!(parse(r#""\ud83d\ude00""#).is_ok());
        assert!(parse("1e999").is_err());
        assert!(canonical(&parse("1.5").unwrap(), true).is_err());
        assert!(canonical(&parse("9007199254740993").unwrap(), true).is_err());
    }
}
