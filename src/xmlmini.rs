//! A small, safe XML reader for protocol answers (OAI-PMH). Elements, attributes, text, CDATA.
//! Only the five predefined entities and numeric character references are expanded; DOCTYPE
//! declarations are skipped and never interpreted, so there is no external or recursive entity
//! expansion. Nesting depth and input size are bounded. Namespace prefixes are dropped: names
//! are compared by their local part.

pub const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 128;

#[derive(Debug, Default, Clone)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// Text directly inside this element (pieces joined), not trimmed.
    pub text: String,
}

impl Node {
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
    pub fn all(&self, name: &str) -> std::vec::IntoIter<&Node> {
        self.children
            .iter()
            .filter(|c| c.name == name)
            .collect::<Vec<_>>()
            .into_iter()
    }
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn text(&self) -> &str {
        self.text.trim()
    }
    /// Trimmed text of the first child with this name.
    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name).map(|c| c.text())
    }
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(j) = rest.find(';').filter(|j| *j <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..j];
        let rep = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match rep {
            Some(c) => {
                out.push(c);
                rest = &rest[j + 1..];
            }
            None => {
                out.push('&'); // unknown entity: left as written, never expanded
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

struct P<'a> {
    s: &'a str,
    i: usize,
}

impl P<'_> {
    fn rest(&self) -> &str {
        &self.s[self.i..]
    }
    fn skip_ws(&mut self) {
        while self.rest().starts_with(|c: char| c.is_whitespace()) {
            self.i += self.rest().chars().next().map_or(1, char::len_utf8);
        }
    }
    fn skip_to(&mut self, end: &str) -> Result<(), String> {
        match self.rest().find(end) {
            Some(k) => {
                self.i += k + end.len();
                Ok(())
            }
            None => Err(format!("unterminated construct (missing `{end}`)")),
        }
    }
    /// Skips `<?..?>`, `<!--..-->`, `<!DOCTYPE..>` (with internal subset) before/after content.
    fn skip_misc(&mut self) -> Result<(), String> {
        loop {
            self.skip_ws();
            if self.rest().starts_with("<?") {
                self.skip_to("?>")?;
            } else if self.rest().starts_with("<!--") {
                self.skip_to("-->")?;
            } else if self.rest().starts_with("<!") {
                let mut depth = 0i32;
                let mut closed = false;
                while let Some(c) = self.rest().chars().next() {
                    self.i += c.len_utf8();
                    match c {
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        '>' if depth <= 0 => {
                            closed = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if !closed {
                    return Err("unterminated declaration".into());
                }
            } else {
                return Ok(());
            }
        }
    }
    fn name(&mut self) -> String {
        let r = self.rest();
        let n = r
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/' || c == '=')
            .unwrap_or(r.len());
        let name = r[..n].to_string();
        self.i += n;
        name
    }
    fn element(&mut self, depth: usize) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err("XML nested too deeply".into());
        }
        if !self.rest().starts_with('<') {
            return Err("expected an element".into());
        }
        self.i += 1;
        let raw = self.name();
        if raw.is_empty() {
            return Err("empty element name".into());
        }
        let mut node = Node {
            name: local(&raw).to_string(),
            ..Default::default()
        };
        loop {
            self.skip_ws();
            let r = self.rest();
            if r.starts_with("/>") {
                self.i += 2;
                return Ok(node);
            }
            if r.starts_with('>') {
                self.i += 1;
                break;
            }
            if r.is_empty() {
                return Err("unexpected end inside a tag".into());
            }
            let an = self.name();
            self.skip_ws();
            if !self.rest().starts_with('=') || an.is_empty() {
                return Err(format!("malformed attribute in <{raw}>"));
            }
            self.i += 1;
            self.skip_ws();
            let q = self
                .rest()
                .chars()
                .next()
                .filter(|c| *c == '"' || *c == '\'');
            let Some(q) = q else {
                return Err("attribute value without quotes".into());
            };
            self.i += 1;
            let end = self.rest().find(q).ok_or("unterminated attribute value")?;
            let val = unescape(&self.rest()[..end]);
            self.i += end + 1;
            node.attrs.push((local(&an).to_string(), val));
        }
        loop {
            let r = self.rest();
            if r.is_empty() {
                return Err(format!("element <{raw}> is not closed"));
            }
            if r.starts_with("</") {
                self.i += 2;
                let close = self.name();
                self.skip_ws();
                if !self.rest().starts_with('>') {
                    return Err("malformed closing tag".into());
                }
                self.i += 1;
                if local(&close) != node.name {
                    return Err(format!("</{close}> does not close <{raw}>"));
                }
                return Ok(node);
            } else if r.starts_with("<!--") {
                self.skip_to("-->")?;
            } else if r.starts_with("<![CDATA[") {
                self.i += 9;
                let end = self.rest().find("]]>").ok_or("unterminated CDATA")?;
                node.text.push_str(&self.rest()[..end]);
                self.i += end + 3;
            } else if r.starts_with("<?") {
                self.skip_to("?>")?;
            } else if r.starts_with('<') {
                let c = self.element(depth + 1)?;
                node.children.push(c);
            } else {
                let end = r.find('<').unwrap_or(r.len());
                node.text.push_str(&unescape(&r[..end]));
                self.i += end;
            }
        }
    }
}

/// Parses a document and returns its root element.
pub fn parse(s: &str) -> Result<Node, String> {
    if s.len() > MAX_BYTES {
        return Err("XML document is larger than 16 MB".into());
    }
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut p = P { s, i: 0 };
    p.skip_misc()?;
    let root = p.element(0)?;
    p.skip_misc()?;
    if !p.rest().trim().is_empty() {
        return Err("content after the root element".into());
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_elements_and_namespaces() {
        let r = parse(
            "<?xml version=\"1.0\"?><!-- c --><a:r xmlns:a=\"u\" id='7'><a:x>1 &amp; 2</a:x><x><![CDATA[<raw>]]></x><e/></a:r>",
        )
        .unwrap();
        assert_eq!(r.name, "r");
        assert_eq!(r.attr("id"), Some("7"));
        assert_eq!(r.child_text("x"), Some("1 & 2"));
        assert_eq!(r.all("x").count(), 2);
        assert_eq!(r.children[1].text, "<raw>");
        assert!(r.child("e").is_some());
    }

    #[test]
    fn entities_are_never_expanded_beyond_the_predefined() {
        let r = parse(
            "<!DOCTYPE x [<!ENTITY a \"aaaa\"><!ENTITY b \"&a;&a;\">]><x>&b; &#65;&#x42; &lt;</x>",
        )
        .unwrap();
        assert_eq!(r.text(), "&b; AB <");
    }

    #[test]
    fn rejects_broken_documents() {
        for bad in [
            "",
            "<a>",
            "<a></b>",
            "<a><b></a>",
            "<a x=1/>",
            "<a/><b/>",
            "text",
            "<a>&#xFFFFFFFF;</a><!--",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        let deep = "<a>".repeat(500) + &"</a>".repeat(500);
        assert!(parse(&deep).is_err());
        assert!(parse(&" ".repeat(MAX_BYTES + 1)).is_err());
    }
}
