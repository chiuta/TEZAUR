//! `tezaur export`: Markdown (as written by draft/ask -o) to LaTeX, Word (.docx) or HTML.
//! Pandoc-style citations `[@smith2019; @doe2020]` become `\cite{…}` in LaTeX and "(smith2019; doe2020)" elsewhere.

use anyhow::Result;
use std::io::Write;

#[derive(Debug, PartialEq, Clone)]
pub enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Cite(Vec<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Block {
    Heading(usize, Vec<Span>),
    Para(Vec<Span>),
    Item(bool, Vec<Span>), // ordered?, text
    Quote(Vec<Span>),
}

pub fn spans(s: &str) -> Vec<Span> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let flush = |buf: &mut String, out: &mut Vec<Span>| {
        if !buf.is_empty() {
            out.push(Span::Text(std::mem::take(buf)));
        }
    };
    let find = |from: usize, pat: &[char]| -> Option<usize> { (from..c.len().saturating_sub(pat.len() - 1)).find(|&j| c[j..j + pat.len()] == *pat) };
    let mut i = 0;
    while i < c.len() {
        if c[i] == '*' && c.get(i + 1) == Some(&'*') {
            if let Some(j) = find(i + 2, &['*', '*']) {
                if j > i + 2 {
                    flush(&mut buf, &mut out);
                    out.push(Span::Bold(c[i + 2..j].iter().collect()));
                    i = j + 2;
                    continue;
                }
            }
        }
        if c[i] == '*' && c.get(i + 1).is_some_and(|n| !n.is_whitespace() && *n != '*') {
            if let Some(j) = find(i + 1, &['*']) {
                if j > i + 1 && !c[j - 1].is_whitespace() {
                    flush(&mut buf, &mut out);
                    out.push(Span::Italic(c[i + 1..j].iter().collect()));
                    i = j + 1;
                    continue;
                }
            }
        }
        if c[i] == '`' {
            if let Some(j) = find(i + 1, &['`']) {
                flush(&mut buf, &mut out);
                out.push(Span::Code(c[i + 1..j].iter().collect()));
                i = j + 1;
                continue;
            }
        }
        if c[i] == '[' && c.get(i + 1) == Some(&'@') {
            if let Some(j) = find(i + 1, &[']']) {
                let inner: String = c[i + 1..j].iter().collect();
                let keys: Vec<String> = inner.split(';').map(|k| k.trim().trim_start_matches('@').trim().to_string()).filter(|k| !k.is_empty()).collect();
                if !keys.is_empty() {
                    flush(&mut buf, &mut out);
                    out.push(Span::Cite(keys));
                    i = j + 1;
                    continue;
                }
            }
        }
        buf.push(c[i]);
        i += 1;
    }
    flush(&mut buf, &mut out);
    out
}

pub fn blocks(md: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Para(spans(&para.join(" "))));
            para.clear();
        }
    };
    for raw in md.lines() {
        let l = raw.trim_end();
        let t = l.trim_start();
        if t.is_empty() {
            flush(&mut para, &mut out);
        } else if t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && t.chars().filter(|c| !c.is_whitespace()).count() >= 3 {
            flush(&mut para, &mut out); // horizontal rule: dropped
        } else if let Some(h) = t.strip_prefix('#') {
            flush(&mut para, &mut out);
            let level = 1 + h.chars().take_while(|c| *c == '#').count();
            out.push(Block::Heading(level.min(4), spans(h.trim_start_matches('#').trim())));
        } else if let Some(r) = t.strip_prefix("> ") {
            flush(&mut para, &mut out);
            out.push(Block::Quote(spans(r)));
        } else if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
            flush(&mut para, &mut out);
            out.push(Block::Item(false, spans(r)));
        } else if let Some(r) = {
            let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
            (d > 0 && t[d..].starts_with(". ")).then(|| &t[d + 2..])
        } {
            flush(&mut para, &mut out);
            out.push(Block::Item(true, spans(r)));
        } else {
            para.push(t.to_string());
        }
    }
    flush(&mut para, &mut out);
    out
}

fn tex_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\textbackslash{}"),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                o.push('\\');
                o.push(c)
            }
            '~' => o.push_str("\\textasciitilde{}"),
            '^' => o.push_str("\\textasciicircum{}"),
            c => o.push(c),
        }
    }
    o
}

fn tex_spans(v: &[Span], cited: &mut bool) -> String {
    v.iter()
        .map(|s| match s {
            Span::Text(t) => tex_escape(t),
            Span::Bold(t) => format!("\\textbf{{{}}}", tex_escape(t)),
            Span::Italic(t) => format!("\\emph{{{}}}", tex_escape(t)),
            Span::Code(t) => format!("\\texttt{{{}}}", tex_escape(t)),
            Span::Cite(k) => {
                *cited = true;
{
                // "smith2019, p. 3" -> key + locator; a locator is kept only for a single key
                let split: Vec<(String, String)> = k.iter().map(|x| match x.find(|c: char| c == ',' || c.is_whitespace()) {
                    Some(i) => (x[..i].to_string(), x[i..].trim_start_matches(|c: char| c == ',' || c.is_whitespace()).to_string()),
                    None => (x.clone(), String::new()),
                }).collect();
                let keys: Vec<&str> = split.iter().map(|(a, _)| a.as_str()).collect();
                if split.len() == 1 && !split[0].1.is_empty() {
                    format!("\\cite[{}]{{{}}}", split[0].1, keys[0])
                } else {
                    format!("\\cite{{{}}}", keys.join(","))
                }
            }
            }
        })
        .collect()
}

/// `bib` = BibTeX file stem to reference when the text cites keys.
pub fn to_tex(md: &str, title: Option<&str>, bib: Option<&str>) -> String {
    let mut cited = false;
    let mut body = String::new();
    let mut list: Option<bool> = None;
    let close = |list: &mut Option<bool>, body: &mut String| {
        if let Some(o) = list.take() {
            body.push_str(if o { "\\end{enumerate}\n\n" } else { "\\end{itemize}\n\n" });
        }
    };
    for b in blocks(md) {
        match b {
            Block::Item(o, s) => {
                if list != Some(o) {
                    close(&mut list, &mut body);
                    body.push_str(if o { "\\begin{enumerate}\n" } else { "\\begin{itemize}\n" });
                    list = Some(o);
                }
                body.push_str(&format!("  \\item {}\n", tex_spans(&s, &mut cited)));
            }
            other => {
                close(&mut list, &mut body);
                match other {
                    Block::Heading(l, s) => {
                        let cmd = ["section", "subsection", "subsubsection", "paragraph"][l.saturating_sub(1).min(3)];
                        body.push_str(&format!("\\{cmd}{{{}}}\n\n", tex_spans(&s, &mut cited)));
                    }
                    Block::Para(s) => body.push_str(&format!("{}\n\n", tex_spans(&s, &mut cited))),
                    Block::Quote(s) => body.push_str(&format!("\\begin{{quote}}\n{}\n\\end{{quote}}\n\n", tex_spans(&s, &mut cited))),
                    Block::Item(..) => unreachable!(),
                }
            }
        }
    }
    close(&mut list, &mut body);
    let mut t = String::from("\\documentclass[12pt]{article}\n\\usepackage[utf8]{inputenc}\n\\usepackage[T1]{fontenc}\n\\usepackage[hidelinks]{hyperref}\n");
    if let Some(ti) = title {
        t.push_str(&format!("\\title{{{}}}\n\\date{{}}\n", tex_escape(ti)));
    }
    t.push_str("\\begin{document}\n");
    if title.is_some() {
        t.push_str("\\maketitle\n");
    }
    t.push_str(&body);
    if let (true, Some(b)) = (cited, bib) {
        t.push_str(&format!("\\bibliographystyle{{plain}}\n\\bibliography{{{b}}}\n"));
    }
    t.push_str("\\end{document}\n");
    t
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn html_spans(v: &[Span]) -> String {
    v.iter()
        .map(|s| match s {
            Span::Text(t) => esc(t),
            Span::Bold(t) => format!("<b>{}</b>", esc(t)),
            Span::Italic(t) => format!("<i>{}</i>", esc(t)),
            Span::Code(t) => format!("<code>{}</code>", esc(t)),
            Span::Cite(k) => format!("<cite>({})</cite>", esc(&k.join("; "))),
        })
        .collect()
}

pub fn to_html(md: &str, title: Option<&str>) -> String {
    let mut body = String::new();
    let mut list: Option<bool> = None;
    for b in blocks(md) {
        if let Block::Item(o, s) = &b {
            if list != Some(*o) {
                if let Some(p) = list {
                    body.push_str(if p { "</ol>\n" } else { "</ul>\n" });
                }
                body.push_str(if *o { "<ol>\n" } else { "<ul>\n" });
                list = Some(*o);
            }
            body.push_str(&format!("<li>{}</li>\n", html_spans(s)));
            continue;
        }
        if let Some(p) = list.take() {
            body.push_str(if p { "</ol>\n" } else { "</ul>\n" });
        }
        match b {
            Block::Heading(l, s) => body.push_str(&format!("<h{l}>{}</h{l}>\n", html_spans(&s))),
            Block::Para(s) => body.push_str(&format!("<p>{}</p>\n", html_spans(&s))),
            Block::Quote(s) => body.push_str(&format!("<blockquote>{}</blockquote>\n", html_spans(&s))),
            Block::Item(..) => {}
        }
    }
    if let Some(p) = list {
        body.push_str(if p { "</ol>\n" } else { "</ul>\n" });
    }
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{t}</title>\n<style>body{{max-width:46em;margin:2em auto;padding:0 1em;font:17px/1.6 Georgia,serif}}blockquote{{color:#555;border-left:3px solid #ccc;margin-left:0;padding-left:1em}}</style></head>\n<body>\n{h}{body}</body></html>\n",
        t = esc(title.unwrap_or("TEZAUR")),
        h = title.map(|t| format!("<h1>{}</h1>\n", esc(t))).unwrap_or_default()
    )
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn docx_runs(v: &[Span]) -> String {
    let run = |t: &str, rpr: &str| format!("<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>", if rpr.is_empty() { String::new() } else { format!("<w:rPr>{rpr}</w:rPr>") }, xml(t));
    v.iter()
        .map(|s| match s {
            Span::Text(t) => run(t, ""),
            Span::Bold(t) => run(t, "<w:b/>"),
            Span::Italic(t) => run(t, "<w:i/>"),
            Span::Code(t) => run(t, "<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>"),
            Span::Cite(k) => run(&format!("({})", k.join("; ")), ""),
        })
        .collect()
}

pub fn to_docx(md: &str, title: Option<&str>) -> Result<Vec<u8>> {
    let mut body = String::new();
    if let Some(t) = title {
        body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr>{}</w:p>", docx_runs(&[Span::Text(t.to_string())])));
    }
    let mut n = 0usize;
    for b in blocks(md) {
        if !matches!(b, Block::Item(..)) {
            n = 0;
        }
        match b {
            Block::Heading(l, s) => body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"Heading{}\"/></w:pPr>{}</w:p>", l.min(3), docx_runs(&s))),
            Block::Para(s) => body.push_str(&format!("<w:p>{}</w:p>", docx_runs(&s))),
            Block::Quote(s) => body.push_str(&format!("<w:p><w:pPr><w:ind w:left=\"720\"/></w:pPr>{}</w:p>", docx_runs(&s))),
            Block::Item(ordered, mut s) => {
                n = if ordered { n + 1 } else { 0 };
                let mark = if ordered { format!("{n}. ") } else { "• ".to_string() };
                s.insert(0, Span::Text(mark));
                body.push_str(&format!("<w:p><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr>{}</w:p>", docx_runs(&s)));
            }
        }
    }
    let ns = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"";
    let document = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document {ns}><w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>");
    let style = |id: &str, name: &str, size: u32, bold: bool| {
        format!("<w:style w:type=\"paragraph\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/><w:pPr><w:spacing w:before=\"240\" w:after=\"120\"/></w:pPr><w:rPr>{}<w:sz w:val=\"{size}\"/></w:rPr></w:style>", if bold { "<w:b/>" } else { "" })
    };
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles {ns}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Times New Roman\" w:hAnsi=\"Times New Roman\"/><w:sz w:val=\"24\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"120\" w:line=\"288\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>{}{}{}{}</w:styles>",
        style("Title", "Title", 40, true),
        style("Heading1", "heading 1", 32, true),
        style("Heading2", "heading 2", 28, true),
        style("Heading3", "heading 3", 24, true)
    );
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut add = |name: &str, data: &str| -> Result<()> {
            z.start_file(name, o)?;
            z.write_all(data.as_bytes())?;
            Ok(())
        };
        add("[Content_Types].xml", "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>")?;
        add("_rels/.rels", "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>")?;
        add("word/_rels/document.xml.rels", "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>")?;
        add("word/document.xml", &document)?;
        add("word/styles.xml", &styles)?;
        z.finish()?;
    }
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "# Chapter *one*\n\nThe burden lies with the **authority** [@smith2019; @doe2020]. Costs & 5% apply_here.\n\n- first\n- second `code`\n\n1. a\n2. b\n\n> quoted\n\n---\n\n## References\n[1] Smith (2019), p. 3";

    #[test]
    fn parses_inline_and_blocks() {
        let s = spans("a **b** *c* `d` [@x; @y] e*f");
        assert_eq!(s, vec![
            Span::Text("a ".into()), Span::Bold("b".into()), Span::Text(" ".into()), Span::Italic("c".into()), Span::Text(" ".into()),
            Span::Code("d".into()), Span::Text(" ".into()), Span::Cite(vec!["x".into(), "y".into()]), Span::Text(" e*f".into())
        ]);
        let b = blocks(MD);
        assert!(matches!(b[0], Block::Heading(1, _)));
        assert_eq!(b.iter().filter(|x| matches!(x, Block::Item(false, _))).count(), 2);
        assert_eq!(b.iter().filter(|x| matches!(x, Block::Item(true, _))).count(), 2);
        assert!(b.iter().any(|x| matches!(x, Block::Quote(_))));
    }

    #[test]
    fn latex_escapes_and_cites() {
        let t = to_tex(MD, Some("My thesis"), Some("refs"));
        assert!(t.contains("\\section{Chapter \\emph{one}}"));
        assert!(t.contains("\\cite{smith2019,doe2020}"));
        assert!(to_tex("x [@a, p. 3] y", None, None).contains("\\cite[p. 3]{a}"));
        assert!(t.contains("Costs \\& 5\\% apply\\_here."));
        assert!(t.contains("\\begin{enumerate}") && t.contains("\\begin{itemize}"));
        assert!(t.contains("\\bibliography{refs}") && t.contains("\\maketitle"));
        assert!(!to_tex("no cites", None, Some("refs")).contains("\\bibliography"));
    }

    #[test]
    fn html_and_docx_are_well_formed() {
        let h = to_html(MD, Some("T & T"));
        assert!(h.contains("<h1>Chapter <i>one</i></h1>") && h.contains("<title>T &amp; T</title>") && h.contains("<ol>"));
        let d = to_docx(MD, Some("Title")).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(d)).unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut s).unwrap();
        assert!(s.contains("Heading1") && s.contains("(smith2019; doe2020)") && s.contains("• </w:t>") && s.contains("1. </w:t>") && s.contains("2. </w:t>"));
        assert!(s.contains("Costs &amp; 5%"));
    }
}
