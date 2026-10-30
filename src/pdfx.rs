//! Layout-aware PDF structure: tables become Markdown tables (and CSV), display equations become LaTeX-style text.
//!
//! This is a heuristic reader built on poppler's `pdftotext -layout`; it does not "see" the page. It recovers what
//! the text layer and its spacing reveal: aligned columns, centred equation lines with limits above/below, equation numbers.
//! It does NOT recover stacked fractions, matrices, multi-line cells or tables drawn as images. For those, a vision/OCR math
//! model (Nougat, MinerU, Marker) is the right tool and is not integrated. Set `TEZAUR_PDF=basic` to turn all of this off.

use std::path::Path;

pub fn enabled() -> bool {
    std::env::var("TEZAUR_PDF")
        .map(|v| v != "basic")
        .unwrap_or(true)
}

fn pdftotext(args: &[String]) -> Option<String> {
    let o = crate::ingest::run_timeout(std::process::Command::new("pdftotext").args(args), 180)?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

fn attr(tag: &str, name: &str) -> Option<f32> {
    let k = format!(" {name}=\"");
    let i = tag.find(&k)? + k.len();
    tag[i..].split('"').next()?.parse().ok()
}

/// (page width, page height, words as (x0, y0, x1, y1)) for every page, from `pdftotext -bbox-layout`. Words, not lines: poppler happily
/// merges the two columns of a page into one "line" when they share a baseline.
type Word = (f32, f32, f32, f32);
type Page = (f32, f32, Vec<Word>);

fn page_lines(path: &Path) -> Option<Vec<Page>> {
    let x = pdftotext(&[
        "-bbox-layout".into(),
        path.to_string_lossy().into(),
        "-".into(),
    ])?;
    let mut pages = Vec::new();
    let mut cur: Option<Page> = None;
    for tag in x.split('<').skip(1) {
        if tag.starts_with("page ") {
            cur = Some((attr(tag, "width")?, attr(tag, "height")?, Vec::new()));
        } else if tag.starts_with("word ") {
            if let (Some(c), Some(a), Some(b), Some(d), Some(e)) = (
                cur.as_mut(),
                attr(tag, "xMin"),
                attr(tag, "yMin"),
                attr(tag, "xMax"),
                attr(tag, "yMax"),
            ) {
                c.2.push((a, b, d, e));
            }
        } else if tag.starts_with("/page") {
            pages.extend(cur.take());
        }
    }
    Some(pages)
}

/// Gutter x of a two-column page: a vertical band that almost no word crosses (titles and wide equations may), with real prose on both sides.
fn gutter(w: f32, words: &[(f32, f32, f32, f32)]) -> Option<f32> {
    if words.len() < 120 {
        return None;
    }
    let mut best: Option<(usize, f32, f32)> = None;
    let mut x = w * 0.35;
    while x <= w * 0.65 {
        let cross = words
            .iter()
            .filter(|l| l.0 < x - 0.5 && l.2 > x + 0.5)
            .count();
        let key = (cross, (x - w / 2.0).abs());
        if best
            .map(|(c, d, _)| key.0 < c || (key.0 == c && key.1 < d))
            .unwrap_or(true)
        {
            best = Some((key.0, key.1, x));
        }
        x += 1.0;
    }
    let (cross, _, g0) = best?;
    // centre the gutter in the free run (same crossing count) around the best point
    let cnt = |x: f32| {
        words
            .iter()
            .filter(|l| l.0 < x - 0.5 && l.2 > x + 0.5)
            .count()
    };
    let (mut a, mut b) = (g0, g0);
    while a - 1.0 >= w * 0.3 && cnt(a - 1.0) <= cross {
        a -= 1.0;
    }
    while b + 1.0 <= w * 0.7 && cnt(b + 1.0) <= cross {
        b += 1.0;
    }
    let g = (a + b) / 2.0;
    let cross = cnt(g);
    let near = 0usize;
    // prose density on each side: rows (by baseline) with at least 5 words
    let rows = |left: bool| -> usize {
        let mut m: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for wd in words
            .iter()
            .filter(|l| if left { l.2 <= g } else { l.0 >= g })
        {
            *m.entry((wd.1 / 3.0) as i32).or_default() += 1;
        }
        m.values().filter(|n| **n >= 5).count()
    };
    (cross * 100 <= words.len() * 3
        && near * 100 <= words.len() * 2
        && rows(true) >= 8
        && rows(false) >= 8)
        .then_some(g)
}

/// Per-page layout text from `pdftotext -layout`, or None when poppler is missing or fails. Two-column pages are cut at the gutter and
/// read left column then right column, band by band (lines that cross the gutter, such as titles, are their own bands), so tables and
/// equations inside one column are not interleaved with the other column's text.
pub fn layout_pages(path: &Path) -> Option<Vec<String>> {
    let p = path.to_string_lossy().to_string();
    let mut v: Vec<String> = pdftotext(&["-layout".into(), p.clone(), "-".into()])?
        .split('\x0c')
        .map(str::to_string)
        .collect();
    if v.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
        v.pop();
    }
    if let Some(pages) = page_lines(path) {
        for (i, (w, h, lines)) in pages.iter().enumerate() {
            if i >= v.len() {
                break;
            }
            let Some(g) = gutter(*w, lines) else { continue };
            let mut barriers: Vec<(f32, f32)> = lines
                .iter()
                .filter(|l| l.0 < g - 0.5 && l.2 > g + 0.5)
                .map(|l| (l.1, l.3))
                .collect();
            barriers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            // merge touching barriers
            let mut merged: Vec<(f32, f32)> = Vec::new();
            for b in barriers {
                match merged.last_mut() {
                    Some(m) if b.0 <= m.1 + 4.0 => m.1 = m.1.max(b.1),
                    _ => merged.push(b),
                }
            }
            let crop = |x: f32, y: f32, cw: f32, ch: f32| -> String {
                let a: Vec<String> = [
                    "-layout",
                    "-f",
                    &(i + 1).to_string(),
                    "-l",
                    &(i + 1).to_string(),
                    "-x",
                    &(x.max(0.0) as i32).to_string(),
                    "-y",
                    &(y.max(0.0) as i32).to_string(),
                    "-W",
                    &(cw.ceil() as i32).to_string(),
                    "-H",
                    &(ch.ceil() as i32).to_string(),
                    &p,
                    "-",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect();
                pdftotext(&a).unwrap_or_default().replace('\x0c', "")
            };
            let mut out = String::new();
            let mut y = 0.0f32;
            let mut bands: Vec<(f32, f32, bool)> = Vec::new();
            for (b0, b1) in &merged {
                if *b0 - 1.0 > y {
                    bands.push((y, *b0 - 1.0, false));
                }
                bands.push((*b0 - 1.0, *b1 + 1.0, true));
                y = *b1 + 1.0;
            }
            bands.push((y, *h, false));
            for (y0, y1, full) in bands {
                if y1 - y0 < 2.0 {
                    continue;
                }
                if full {
                    out.push_str(&crop(0.0, y0, *w, y1 - y0));
                } else {
                    out.push_str(&crop(0.0, y0, g, y1 - y0));
                    out.push('\n');
                    out.push_str(&crop(g, y0, *w - g, y1 - y0));
                }
                out.push('\n');
            }
            v[i] = out;
        }
    }
    Some(v)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub caption: Option<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub latex: String,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Text(String),
    Table(Table),
    Equation(Equation),
}

impl Table {
    pub fn to_markdown(&self) -> String {
        let n = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let line = |r: &Vec<String>| {
            let mut c: Vec<String> = r.iter().map(|x| x.replace('|', "\\|")).collect();
            c.resize(n, String::new());
            format!("| {} |", c.join(" | "))
        };
        let mut out = String::new();
        if let Some(c) = &self.caption {
            out.push_str(c);
            out.push('\n');
        }
        for (i, r) in self.rows.iter().enumerate() {
            out.push_str(&line(r));
            out.push('\n');
            if i == 0 {
                out.push_str(&format!("|{}|\n", vec![" --- "; n].join("|")));
            }
        }
        out
    }

    pub fn to_csv(&self) -> String {
        let q = |s: &str| {
            if s.contains([',', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        self.rows
            .iter()
            .map(|r| r.iter().map(|c| q(c)).collect::<Vec<_>>().join(","))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }
}

// ------------------------------------------------------------------ helpers

fn indent(l: &str) -> usize {
    l.chars().take_while(|c| *c == ' ').count()
}

/// (start, end, text) of each cell: pieces separated by two or more spaces. Positions are in characters.
fn cells(line: &str) -> Vec<(usize, usize, String)> {
    let ch: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let (mut i, n) = (0, ch.len());
    while i < n {
        while i < n && ch[i] == ' ' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let s = i;
        let mut last = i;
        while i < n {
            if ch[i] == ' ' {
                if i + 1 < n && ch[i + 1] == ' ' {
                    break;
                }
            } else {
                last = i;
            }
            i += 1;
        }
        out.push((s, last + 1, ch[s..=last].iter().collect::<String>()));
    }
    out
}

fn is_caption(l: &str) -> bool {
    let t = l.trim().to_lowercase();
    ["table ", "tabel ", "tabelul ", "tabela ", "tab. "]
        .iter()
        .any(|p| {
            t.strip_prefix(p)
                .map(|r| {
                    r.chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
}

fn words(s: &str) -> usize {
    s.split_whitespace().count()
}

// ------------------------------------------------------------------- tables

/// A table starting at `i` (without caption): rows are lines with at least two cells; short single-cell lines between rows are kept.
fn find_table(lines: &[&str], i: usize) -> Option<(usize, Vec<Vec<String>>)> {
    if i >= lines.len() || cells(lines[i]).len() < 2 {
        return None;
    }
    let mut end = i;
    let mut tab_lines = 0;
    let mut j = i;
    while j < lines.len() {
        let c = cells(lines[j]);
        if c.len() >= 2 {
            tab_lines += 1;
            end = j + 1;
            j += 1;
        } else if !lines[j].trim().is_empty()
            && lines[j].trim().chars().count() <= 40
            && j + 1 < lines.len()
            && cells(lines[j + 1]).len() >= 2
            && end > i
        {
            j += 1; // a spanning heading or a one-cell row between two real rows
        } else {
            break;
        }
    }
    if tab_lines < 2 {
        return None;
    }
    let block: Vec<&str> = lines[i..end].to_vec();
    // column boundaries: positions blank in every multi-cell row, runs of at least 2
    let width = block.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut occ = vec![false; width + 1];
    // columns come from the body rows (a header may be wider than its data); fall back to every row
    let multi: Vec<usize> = (0..block.len())
        .filter(|&k| cells(block[k]).len() >= 2)
        .collect();
    let skip_first = multi.len() >= 3;
    for (k, l) in block.iter().enumerate() {
        if skip_first && k == multi[0] {
            continue;
        }
        let c = cells(l);
        if c.len() >= 2 {
            for (s, e, _) in c {
                for o in occ.iter_mut().take(e).skip(s) {
                    *o = true;
                }
            }
        }
    }
    let mut cols: Vec<(usize, usize)> = Vec::new();
    let mut p = 0;
    while p < width {
        if occ[p] {
            let s = p;
            let mut e = p;
            let mut gap = 0;
            while e < width {
                if occ[e] {
                    gap = 0;
                } else {
                    gap += 1;
                    if gap >= 2 {
                        break;
                    }
                }
                e += 1;
            }
            cols.push((s, e - gap.min(e - s)));
            p = e + 1;
        } else {
            p += 1;
        }
    }
    if cols.len() < 2 {
        return None;
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for l in &block {
        let mut row = vec![String::new(); cols.len()];
        for (s, e, t) in cells(l) {
            let k = cols
                .iter()
                .position(|&(cs, ce)| s < ce && e > cs)
                .unwrap_or_else(|| {
                    cols.iter()
                        .position(|&(_, ce)| s < ce)
                        .unwrap_or(cols.len() - 1)
                });
            if !row[k].is_empty() {
                row[k].push(' ');
            }
            row[k].push_str(t.trim());
        }
        if row.iter().any(|c| !c.is_empty()) {
            rows.push(row);
        }
    }
    // a header whose words ran together ("Method Acc.   F1" over three columns): give the trailing words to the empty columns after it
    if let Some(h) = rows.first_mut() {
        let mut k = 0;
        while k < h.len() {
            let empties = h[k + 1..].iter().take_while(|c| c.is_empty()).count();
            let ws: Vec<String> = h[k].split_whitespace().map(String::from).collect();
            if empties > 0
                && ws.len() > empties
                && k + empties < h.len()
                && h[k + 1..].iter().skip(empties).any(|c| !c.is_empty())
                || (empties > 0 && ws.len() > empties && k + empties + 1 == h.len())
            {
                let keep = ws.len() - empties;
                h[k] = ws[..keep].join(" ");
                for (e, w) in ws[keep..].iter().enumerate() {
                    h[k + 1 + e] = w.clone();
                }
                k += empties;
            }
            k += 1;
        }
    }
    // reject running text set in columns: tables hold short cells
    let all: Vec<&String> = rows.iter().flatten().filter(|c| !c.is_empty()).collect();
    let long = all.iter().filter(|c| words(c) > 7).count();
    if all.is_empty() || long * 4 > all.len() {
        return None;
    }
    Some((end, rows))
}

// ---------------------------------------------------------------- equations

const OPS: &str = "=≤≥≈≠±∑∫∏√∂∇∞∈∉⊂⊆∪∩→⇒⇔∀∃×÷≡∝⟨⟩";

fn latex_of(c: char) -> Option<&'static str> {
    Some(match c {
        'α' => "\\alpha ",
        'β' => "\\beta ",
        'γ' => "\\gamma ",
        'δ' => "\\delta ",
        'ε' | 'ϵ' => "\\epsilon ",
        'ζ' => "\\zeta ",
        'η' => "\\eta ",
        'θ' => "\\theta ",
        'ι' => "\\iota ",
        'κ' => "\\kappa ",
        'λ' => "\\lambda ",
        'μ' | 'µ' => "\\mu ",
        'ν' => "\\nu ",
        'ξ' => "\\xi ",
        'π' => "\\pi ",
        'ρ' => "\\rho ",
        'σ' => "\\sigma ",
        'τ' => "\\tau ",
        'υ' => "\\upsilon ",
        'φ' | 'ϕ' => "\\phi ",
        'χ' => "\\chi ",
        'ψ' => "\\psi ",
        'ω' => "\\omega ",
        'Γ' => "\\Gamma ",
        'Δ' => "\\Delta ",
        'Θ' => "\\Theta ",
        'Λ' => "\\Lambda ",
        'Ξ' => "\\Xi ",
        'Π' => "\\Pi ",
        'Σ' => "\\Sigma ",
        'Φ' => "\\Phi ",
        'Ψ' => "\\Psi ",
        'Ω' => "\\Omega ",
        '≤' => "\\leq ",
        '≥' => "\\geq ",
        '≈' => "\\approx ",
        '≠' => "\\neq ",
        '±' => "\\pm ",
        '∑' => "\\sum ",
        '∫' => "\\int ",
        '∏' => "\\prod ",
        '√' => "\\sqrt ",
        '∂' => "\\partial ",
        '∇' => "\\nabla ",
        '∞' => "\\infty ",
        '∈' => "\\in ",
        '∉' => "\\notin ",
        '⊂' => "\\subset ",
        '⊆' => "\\subseteq ",
        '∪' => "\\cup ",
        '∩' => "\\cap ",
        '→' => "\\to ",
        '⇒' => "\\Rightarrow ",
        '⇔' => "\\Leftrightarrow ",
        '∀' => "\\forall ",
        '∃' => "\\exists ",
        '×' => "\\times ",
        '÷' => "\\div ",
        '·' | '⋅' => "\\cdot ",
        '≡' => "\\equiv ",
        '∝' => "\\propto ",
        '−' | '–' => "-",
        '⟨' => "\\langle ",
        '⟩' => "\\rangle ",
        '∅' => "\\emptyset ",
        _ => return None,
    })
}

fn sup_sub(c: char) -> Option<(bool, char)> {
    const SUP: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹ⁿ⁺⁻";
    const SUPN: &str = "0123456789n+-";
    const SUB: &str = "₀₁₂₃₄₅₆₇₈₉₊₋";
    const SUBN: &str = "0123456789+-";
    if let Some(i) = SUP.chars().position(|x| x == c) {
        return Some((true, SUPN.chars().nth(i)?));
    }
    SUB.chars()
        .position(|x| x == c)
        .and_then(|i| SUBN.chars().nth(i))
        .map(|d| (false, d))
}

/// Unicode math → LaTeX commands; super/subscript digits → ^{…} / _{…}.
pub fn to_latex(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if let Some((sup, d)) = sup_sub(c) {
            out.push_str(&format!("{}{{{d}}}", if sup { '^' } else { '_' }));
        } else if let Some(l) = latex_of(c) {
            out.push_str(l);
        } else {
            out.push(c);
        }
    }
    let mut t = String::new();
    let mut prev_space = false;
    for c in out.chars() {
        if c == ' ' {
            if !prev_space {
                t.push(' ');
            }
            prev_space = true;
        } else {
            t.push(c);
            prev_space = false;
        }
    }
    t.trim().to_string()
}

fn limit_like(l: &str) -> bool {
    let t = l.trim();
    !t.is_empty() && t.chars().count() <= 14 && indent(l) >= 6
}

/// Trailing equation number: "(3)" or "(2.1)" after a wide gap. Returns (body, tag).
fn split_tag(l: &str) -> (String, Option<String>) {
    let t = l.trim_end();
    if let Some(open) = t.rfind('(') {
        let inner = &t[open + 1..];
        if let Some(num) = inner.strip_suffix(')') {
            let gap = t[..open].chars().rev().take_while(|c| *c == ' ').count();
            if gap >= 3 && !num.is_empty() && num.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return (t[..open].to_string(), Some(num.to_string()));
            }
        }
    }
    (t.to_string(), None)
}

/// A display-equation line: strong math signal, few real words, and centred or numbered.
fn is_main_eq(l: &str) -> bool {
    let t = l.trim();
    let n = t.chars().count();
    if !(3..=160).contains(&n) {
        return false;
    }
    let (body, tag) = split_tag(l);
    let strong = body.chars().any(|c| OPS.contains(c));
    let long_words = body
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.chars().count() >= 5)
        .count();
    strong && long_words <= 2 && (indent(l) >= 8 || tag.is_some())
}

fn find_equation(lines: &[&str], i: usize) -> Option<(usize, Equation)> {
    // up to two short centred lines above (limits, big operator), then the main line
    let mut m = i;
    let op_line = |l: &str| {
        matches!(
            l.split_whitespace().next(),
            Some("X") | Some("Z") | Some("Y")
        )
    };
    while m < lines.len()
        && m < i + 3
        && limit_like(lines[m])
        && (op_line(lines[m]) || !is_main_eq(lines[m]))
    {
        m += 1;
    }
    if m >= lines.len() || !is_main_eq(lines[m]) {
        return None;
    }
    let above: Vec<&str> = lines[i..m].iter().map(|l| l.trim()).collect();
    let (b0, mut tag) = split_tag(lines[m]);
    let mut body = b0.trim_start().to_string();
    let mut end = m + 1;
    let mut below: Option<String> = None;
    // a big operator drawn as a letter in a math font: X = sum, Z = integral, Y = product
    let has_op = above.iter().any(|a| {
        matches!(
            a.split_whitespace().next(),
            Some("X") | Some("Z") | Some("Y")
        )
    });
    if end < lines.len() && limit_like(lines[end]) && (has_op || !is_main_eq(lines[end])) {
        below = Some(lines[end].trim().to_string());
        end += 1;
    }
    let mut op: Option<(&str, String)> = None;
    let mut rest: Vec<String> = Vec::new();
    for (k, a) in above.iter().enumerate() {
        let toks: Vec<&str> = a.split_whitespace().collect();
        if op.is_none() && !toks.is_empty() && matches!(toks[0], "X" | "Z" | "Y") {
            let name = match toks[0] {
                "X" => "\\sum",
                "Z" => "\\int",
                _ => "\\prod",
            };
            let mut upper = toks[1..].join(" ");
            if upper.is_empty() && k > 0 {
                upper = above[k - 1].to_string();
                rest.retain(|r| r != &upper);
            }
            op = Some((name, upper));
        } else {
            rest.push(a.to_string());
        }
    }
    let mut extra: Vec<String> = Vec::new();
    if let Some((name, upper)) = &op {
        let lower = below.take().unwrap_or_default();
        let mut s = name.to_string();
        if !lower.is_empty() {
            s.push_str(&format!("_{{{}}}", to_latex(&lower)));
        }
        if !upper.is_empty() {
            s.push_str(&format!("^{{{}}}", to_latex(upper)));
        }
        // the operator sits in the wide gap of the main line
        let ch: Vec<char> = body.chars().collect();
        let mut pos = None;
        let mut run = 0;
        for (idx, c) in ch.iter().enumerate() {
            if *c == ' ' {
                run += 1;
                if run >= 3 && pos.is_none() && idx > 1 {
                    pos = Some(idx + 1 - run);
                }
            } else {
                run = 0;
            }
        }
        match pos {
            Some(p) => {
                let (a, b): (String, String) = (ch[..p].iter().collect(), ch[p..].iter().collect());
                body = format!("{a} {s} {b}");
            }
            None => body = format!("{body} {s}"),
        }
        rest.retain(|r| !upper.contains(r.as_str()));
    } else {
        if let Some(b) = below.take() {
            extra.push(b);
        }
    }
    extra.extend(rest);
    let mut latex = to_latex(&body);
    // continuation lines of an aligned block
    while end < lines.len() && is_main_eq(lines[end]) {
        let (b2, t2) = split_tag(lines[end]);
        latex.push_str(&format!(" \\\\ {}", to_latex(&b2)));
        if t2.is_some() {
            tag = t2;
        }
        end += 1;
    }
    if !extra.is_empty() {
        latex.push_str(&format!(
            " % also on the page near it: {}",
            extra
                .iter()
                .map(|e| to_latex(e))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Some((end, Equation { latex, tag }))
}

// -------------------------------------------------------------------- pages

/// Splits one page of `pdftotext -layout` output into text, tables and display equations, in reading order.
pub fn parse_page(layout: &str) -> Vec<Item> {
    let lines: Vec<&str> = layout.lines().collect();
    let mut items: Vec<Item> = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, items: &mut Vec<Item>| {
        if !text.trim().is_empty() {
            items.push(Item::Text(std::mem::take(text)));
        } else {
            text.clear();
        }
    };
    let mut i = 0;
    while i < lines.len() {
        // caption above a table
        if is_caption(lines[i])
            && cells(lines[i]).len() == 1
            && let Some((end, rows)) = find_table(&lines, i + 1)
        {
            flush(&mut text, &mut items);
            items.push(Item::Table(Table {
                caption: Some(lines[i].trim().to_string()),
                rows,
            }));
            i = end;
            continue;
        }
        if let Some((end, rows)) = find_table(&lines, i)
            && (rows.len() >= 3
                || (rows.len() == 2
                    && rows
                        .iter()
                        .flatten()
                        .any(|c| c.chars().any(|x| x.is_ascii_digit()))))
        {
            flush(&mut text, &mut items);
            let mut cap = None;
            let mut e = end;
            if e < lines.len() && is_caption(lines[e]) {
                cap = Some(lines[e].trim().to_string());
                e += 1;
            }
            items.push(Item::Table(Table { caption: cap, rows }));
            i = e;
            continue;
        }
        if let Some((end, eq)) = find_equation(&lines, i) {
            flush(&mut text, &mut items);
            items.push(Item::Equation(eq));
            i = end;
            continue;
        }
        let t = lines[i].trim();
        if !t.is_empty() || !text.ends_with("\n\n") {
            text.push_str(
                &t.split("  ")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            text.push('\n');
        }
        i += 1;
    }
    flush(&mut text, &mut items);
    items
}

pub fn has_structure(items: &[Item]) -> bool {
    items.iter().any(|i| !matches!(i, Item::Text(_)))
}

/// Text form stored in the index: tables as Markdown (caption line first), equations as `[equation] $$ … $$ (eq. N)`.
pub fn render(items: &[Item]) -> String {
    let mut s = String::new();
    for it in items {
        match it {
            Item::Text(t) => {
                s.push_str(t.trim_end());
                s.push_str("\n\n");
            }
            Item::Table(t) => {
                s.push_str(&t.to_markdown());
                s.push('\n');
            }
            Item::Equation(e) => {
                s.push_str(&format!(
                    "[equation] $$ {} $${}\n\n",
                    e.latex,
                    e.tag
                        .as_ref()
                        .map(|t| format!(" (eq. {t})"))
                        .unwrap_or_default()
                ));
            }
        }
    }
    s
}

// ------------------------------------------------------------ chunk support

fn is_table_line(l: &str) -> bool {
    l.trim_start().starts_with('|')
}

/// Splits a page's text into passages, keeping Markdown tables whole (and repeating the header when a long table must be split)
/// so that a row is never separated from its column names. Prose goes through `chunk_words` as usual.
pub fn chunk_page(
    text: &str,
    size: usize,
    overlap: usize,
    prose: &dyn Fn(&str, usize, usize) -> Vec<String>,
) -> Vec<String> {
    if !text.lines().any(is_table_line) {
        return prose(text, size, overlap);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < lines.len() {
        let caption_next =
            is_caption(lines[i]) && i + 1 < lines.len() && is_table_line(lines[i + 1]);
        if caption_next || is_table_line(lines[i]) {
            out.extend(prose(&buf, size, overlap));
            buf.clear();
            let cap = if caption_next {
                i += 1;
                Some(lines[i - 1].trim().to_string())
            } else {
                None
            };
            let mut j = i;
            while j < lines.len() && is_table_line(lines[j]) {
                j += 1;
            }
            let tab = &lines[i..j];
            let head: Vec<&str> = tab.iter().take(2).cloned().collect();
            let body = &tab[head.len()..];
            let prefix =
                |c: &Option<String>| c.as_ref().map(|c| format!("{c}\n")).unwrap_or_default();
            let mut cur: Vec<&str> = Vec::new();
            let mut w = words(&head.join(" "));
            for r in body {
                if !cur.is_empty() && w + words(r) > size * 3 / 2 {
                    out.push(format!(
                        "{}{}\n{}",
                        prefix(&cap),
                        head.join("\n"),
                        cur.join("\n")
                    ));
                    cur.clear();
                    w = words(&head.join(" "));
                }
                cur.push(r);
                w += words(r);
            }
            out.push(format!(
                "{}{}{}",
                prefix(&cap),
                head.join("\n"),
                if cur.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", cur.join("\n"))
                }
            ));
            i = j;
        } else {
            buf.push_str(lines[i]);
            buf.push('\n');
            i += 1;
        }
    }
    out.extend(prose(&buf, size, overlap));
    out.into_iter().filter(|c| !c.trim().is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "\
The mean processing time differed between member states, as summarised in Table 1.

                       Table 1: Mean processing time by country (days).
                      Country      Applications       Mean days        Approved (%)
                      Romania                1240            62.5              81.2
                      France                 3310            48.0              90.4
                      Germany                5120            71.3              77.9

    The expected waiting time follows
                                                n
                                                X
                                      E[T ] =         pi ti ≤ α + βσ                          (1)
                                                i=1

where α and β are constants. The total cost is
                                     Z ∞
                               C=         λe−λt dt = 1           (λ > 0)                      (2)
                                         0

and finally µ ≈ 3.14 was estimated. This is ordinary prose that continues after the equations.
";

    #[test]
    fn finds_table_with_caption_and_columns() {
        let items = parse_page(PAGE);
        let t = items
            .iter()
            .find_map(|i| {
                if let Item::Table(t) = i {
                    Some(t)
                } else {
                    None
                }
            })
            .expect("table");
        assert_eq!(
            t.caption.as_deref(),
            Some("Table 1: Mean processing time by country (days).")
        );
        assert_eq!(t.rows.len(), 4);
        assert_eq!(
            t.rows[0],
            vec!["Country", "Applications", "Mean days", "Approved (%)"]
        );
        assert_eq!(t.rows[3], vec!["Germany", "5120", "71.3", "77.9"]);
        let md = t.to_markdown();
        assert!(md.contains("| Romania | 1240 | 62.5 | 81.2 |") && md.contains("| --- |"));
        assert!(
            t.to_csv()
                .starts_with("Country,Applications,Mean days,Approved (%)\nRomania,1240")
        );
    }

    #[test]
    fn reconstructs_sum_and_integral_with_tags() {
        let eqs: Vec<Equation> = parse_page(PAGE)
            .into_iter()
            .filter_map(|i| {
                if let Item::Equation(e) = i {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(eqs.len(), 2, "{eqs:?}");
        assert_eq!(eqs[0].tag.as_deref(), Some("1"));
        assert!(
            eqs[0].latex.contains("\\sum_{i=1}^{n}")
                && eqs[0].latex.contains("\\leq \\alpha")
                && eqs[0].latex.starts_with("E[T ] ="),
            "{}",
            eqs[0].latex
        );
        assert_eq!(eqs[1].tag.as_deref(), Some("2"));
        assert!(
            eqs[1].latex.contains("\\int_{0}^{\\infty}") && eqs[1].latex.contains("\\lambda"),
            "{}",
            eqs[1].latex
        );
    }

    #[test]
    fn prose_and_inline_math_stay_text() {
        let items = parse_page(PAGE);
        let text: String = items
            .iter()
            .filter_map(|i| {
                if let Item::Text(t) = i {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            text.contains("where α and β are constants") && text.contains("µ ≈ 3.14 was estimated"),
            "{text}"
        );
    }

    #[test]
    fn two_column_prose_is_not_a_table() {
        let p = "\
  The committee considered the evidence presented     In the second stage the authorities reviewed the file
  by the applicants during the long hearing today     and compared the qualification with the national one
  and found that several of the claims were not     before issuing a decision within the legal time limit
  supported by the documents that were attached     which is four months from the complete application
";
        assert!(!has_structure(&parse_page(p)));
    }

    #[test]
    fn short_numbered_lines_are_not_equations() {
        let p = "Chapter 3\n\n        3.2 Methods and results\n\n   x = 5 is mentioned in the text of the paragraph here\n";
        assert!(!has_structure(&parse_page(p)));
    }

    #[test]
    fn chunker_keeps_tables_whole_and_repeats_header() {
        let md = render(&parse_page(PAGE));
        let prose = |t: &str, s: usize, o: usize| crate::ingest::chunk_words(t, s, o);
        let ch = chunk_page(&md, 200, 40, &prose);
        let tab = ch.iter().find(|c| c.contains("Romania")).unwrap();
        assert!(
            tab.starts_with("Table 1:")
                && tab.contains("| Country |")
                && tab.contains("| Germany | 5120 |")
        );
        assert!(ch.iter().any(|c| c.contains("[equation] $$")));
        // a long table splits by rows with the header repeated
        let mut big = String::from("Table 9: Big.\n| a | b |\n| --- | --- |\n");
        for i in 0..400 {
            big.push_str(&format!("| row{i} | value{i} |\n"));
        }
        let parts = chunk_page(&big, 100, 10, &prose);
        assert!(
            parts.len() > 2
                && parts
                    .iter()
                    .all(|p| p.contains("| a | b |") && p.starts_with("Table 9"))
        );
    }

    #[test]
    fn latex_conversion() {
        assert_eq!(to_latex("x² ≤ α + y₁"), "x^{2} \\leq \\alpha + y_{1}");
    }
}
