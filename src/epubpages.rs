//! Printed page numbers for EPUB (spec 11): the book's own page list (`nav` with
//! `epub:type="page-list"`) or page-break markers in the content (`epub:type="pagebreak"`,
//! `role="doc-pagebreak"` with the label in `aria-label`). Text is cut at each marker, in reading
//! order. Labels are kept as strings ("14", "xii"); the labels are the book's declared pagination,
//! not positions in the file.

use crate::ingest::{attr, open_tags, percent_decode, resolve_href, strip_html, zip_read};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const MARK: char = '\u{E000}';
const END: char = '\u{E001}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    PageList,
    Markers,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub label: String,
    /// 1-based position in reading order; labels may repeat, ordinals never do.
    pub ordinal: usize,
    pub text: String,
}

#[derive(Debug)]
pub struct Pagination {
    pub source: Source,
    /// Text before the first marker (no page).
    pub front: String,
    pub pages: Vec<Page>,
    pub warnings: Vec<String>,
}

impl Pagination {
    /// Largest Arabic label; Roman and other labels do not count.
    #[cfg(test)]
    pub fn doc_pages(&self) -> usize {
        self.pages
            .iter()
            .filter_map(|p| arabic(&p.label))
            .max()
            .unwrap_or(0)
    }

    /// `(page number, text)` pairs as the indexer takes them: Arabic labels are their own number;
    /// Roman and other labels have no number (0) and stay searchable.
    pub fn to_indexed(&self) -> Vec<(usize, String)> {
        let mut v = vec![(0, self.front.clone())];
        v.extend(
            self.pages
                .iter()
                .map(|p| (arabic(&p.label).unwrap_or(0), p.text.clone())),
        );
        v.retain(|(_, t)| !t.trim().is_empty());
        v
    }
}

fn arabic(label: &str) -> Option<usize> {
    let l = label.trim();
    (!l.is_empty() && l.chars().all(|c| c.is_ascii_digit()))
        .then(|| l.parse().ok())
        .flatten()
        .filter(|n| *n > 0)
}

/// Marks element positions in raw content HTML: before every tag with an `id` in `targets`, and
/// before page-break markers. Marks are private-use characters that survive `strip_html`.
fn mark_html(html: &str, doc: usize, targets: &HashSet<String>, with_breaks: bool) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    let mut rest = html;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let Some(j) = after.find('>') else {
            out.push_str(after);
            return out;
        };
        let tag = &after[..=j];
        let is_open = !(tag.starts_with("</") || tag.starts_with("<!") || tag.starts_with("<?"));
        if is_open {
            if let Some(id) = attr(tag, "id").filter(|id| targets.contains(id)) {
                out.push_str(&format!("{MARK}I{doc}:{id}{END}"));
            }
            if with_breaks {
                let ty = attr(tag, "epub:type").unwrap_or_default();
                let role = attr(tag, "role").unwrap_or_default();
                let is_break = ty.split_whitespace().any(|w| w == "pagebreak")
                    || role.split_whitespace().any(|w| w == "doc-pagebreak");
                if is_break {
                    let label = attr(tag, "aria-label").unwrap_or_default();
                    out.push_str(&format!("{MARK}B{}{END}", label.trim()));
                }
            }
        }
        out.push_str(tag);
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    out
}

enum Ev {
    Text(String),
    Id(usize, String),
    Break(String),
}

fn events(text: &str) -> Vec<Ev> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(MARK) {
        if i > 0 {
            out.push(Ev::Text(rest[..i].to_string()));
        }
        let after = &rest[i + MARK.len_utf8()..];
        let Some(j) = after.find(END) else {
            rest = after;
            continue;
        };
        let body = &after[..j];
        if let Some(r) = body.strip_prefix('I') {
            if let Some((d, id)) = r.split_once(':') {
                out.push(Ev::Id(d.parse().unwrap_or(usize::MAX), id.to_string()));
            }
        } else if let Some(r) = body.strip_prefix('B') {
            out.push(Ev::Break(r.to_string()));
        }
        rest = &after[j + END.len_utf8()..];
    }
    if !rest.is_empty() {
        out.push(Ev::Text(rest.to_string()));
    }
    out
}

struct Entry {
    label: String,
    doc: usize,
    frag: Option<String>,
}

/// Entries of the first `nav epub:type="page-list"`: label = link text, target = file#id.
fn page_list(
    nav_path: &str,
    nav_html: &str,
    spine: &[(String, String)],
    warn: &mut Vec<String>,
) -> Option<Vec<Entry>> {
    let dir = nav_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut from = 0;
    let section = loop {
        let rel = nav_html[from..].find("<nav")?;
        let start = from + rel;
        let end_tag = nav_html[start..].find('>')? + start;
        let tag = &nav_html[start..=end_tag];
        if attr(tag, "epub:type").is_some_and(|t| t.split_whitespace().any(|w| w == "page-list")) {
            let close = nav_html[end_tag..]
                .find("</nav>")
                .map(|c| end_tag + c)
                .unwrap_or(nav_html.len());
            break &nav_html[end_tag + 1..close];
        }
        from = end_tag + 1;
    };
    let mut out = Vec::new();
    let mut rest = section;
    while let Some(i) = rest.find("<a") {
        let after = &rest[i..];
        let Some(j) = after.find('>') else { break };
        let tag = &after[..=j];
        if !after[2..].starts_with(|c: char| c.is_whitespace()) {
            rest = &after[2..];
            continue;
        }
        let inner_end = after.find("</a>").unwrap_or(after.len());
        let label = strip_html(&after[j + 1..inner_end])
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        rest = &after[inner_end.min(after.len())..];
        let Some(href) = attr(tag, "href") else {
            continue;
        };
        let (file_part, frag) = match href.split_once('#') {
            Some((f, g)) => (f, Some(percent_decode(g))),
            None => (href.as_str(), None),
        };
        let target = if file_part.is_empty() {
            nav_path.to_string()
        } else {
            resolve_href(dir, file_part)
        };
        match spine.iter().position(|(p, _)| *p == target) {
            Some(doc) => out.push(Entry { label, doc, frag }),
            None => warn.push(format!(
                "page list link to `{href}` is not a spine document; ignored"
            )),
        }
    }
    Some(out)
}

/// Cuts the book into pages. `spine`: (path, raw XHTML) in reading order, navigation document
/// excluded; `nav`: (path, XHTML) of the navigation document.
pub fn paginate(spine: &[(String, String)], nav: Option<(&str, &str)>) -> Pagination {
    let mut warnings = Vec::new();
    let list = nav
        .and_then(|(p, h)| page_list(p, h, spine, &mut warnings))
        .filter(|l| !l.is_empty());
    let source = if list.is_some() {
        Source::PageList
    } else {
        Source::Markers
    };
    let mut by_target: HashMap<(usize, String), Vec<usize>> = HashMap::new();
    let mut start_of_doc: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut targets: HashSet<String> = HashSet::new();
    if let Some(l) = &list {
        for (k, e) in l.iter().enumerate() {
            match &e.frag {
                Some(f) => {
                    targets.insert(f.clone());
                    by_target.entry((e.doc, f.clone())).or_default().push(k);
                }
                None => start_of_doc.entry(e.doc).or_default().push(k),
            }
        }
    }
    // reading-order events
    let mut evs: Vec<Ev> = Vec::new();
    for (d, (_, html)) in spine.iter().enumerate() {
        let marked = mark_html(html, d, &targets, list.is_none());
        let text = strip_html(&marked);
        let mut here = events(&text);
        if let Some(l) = &list {
            let present: HashSet<&String> = here
                .iter()
                .filter_map(|e| {
                    if let Ev::Id(dd, id) = e {
                        (*dd == d).then_some(id)
                    } else {
                        None
                    }
                })
                .collect();
            let mut missing: Vec<usize> = start_of_doc.get(&d).cloned().unwrap_or_default();
            for ((dd, f), ks) in &by_target {
                if *dd == d && !present.contains(f) {
                    warnings.push(format!(
                        "page list target `{}#{f}` not found; page starts at the file start",
                        spine[d].0
                    ));
                    missing.extend(ks);
                }
            }
            missing.sort_unstable();
            let mut head: Vec<Ev> = missing
                .into_iter()
                .map(|k| Ev::Break(l[k].label.clone()))
                .collect();
            head.append(&mut here);
            here = head;
        }
        evs.extend(here);
        evs.push(Ev::Text("\n\n".into()));
    }
    let mut front = String::new();
    let mut pages: Vec<Page> = Vec::new();
    for ev in evs {
        match ev {
            Ev::Text(t) => match pages.last_mut() {
                Some(p) => p.text.push_str(&t),
                None => front.push_str(&t),
            },
            Ev::Break(label) => {
                if !label.trim().is_empty() {
                    let ordinal = pages.len() + 1;
                    pages.push(Page {
                        label: label.trim().to_string(),
                        ordinal,
                        text: String::new(),
                    });
                }
            }
            Ev::Id(d, id) => {
                if let (Some(l), Some(ks)) = (&list, by_target.get(&(d, id))) {
                    for &k in ks {
                        let ordinal = pages.len() + 1;
                        pages.push(Page {
                            label: l[k].label.clone(),
                            ordinal,
                            text: String::new(),
                        });
                    }
                }
            }
        }
    }
    let tidy = |s: &str| {
        s.lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };
    front = tidy(&front);
    for p in &mut pages {
        p.text = tidy(&p.text);
    }
    let source = if pages.is_empty() {
        Source::None
    } else {
        source
    };
    Pagination {
        source,
        front,
        pages,
        warnings,
    }
}

/// Reads an EPUB 3 and returns `(page, text)` pairs when the book declares printed pages.
/// None: no navigation document (EPUB 2), unreadable package, or no pagination at all.
pub fn read(path: &Path) -> Option<Vec<(usize, String)>> {
    let f = std::fs::File::open(path).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let container = zip_read(&mut z, "META-INF/container.xml")?;
    let opf_path = open_tags(&container, "rootfile")
        .iter()
        .find_map(|t| attr(t, "full-path"))?;
    let opf = zip_read(&mut z, &opf_path)?;
    let dir = opf_path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();
    let mut manifest: HashMap<String, (String, bool)> = HashMap::new();
    let mut nav_href: Option<String> = None;
    for t in open_tags(&opf, "item") {
        if let (Some(id), Some(href)) = (attr(t, "id"), attr(t, "href")) {
            let nav =
                attr(t, "properties").is_some_and(|p| p.split_whitespace().any(|w| w == "nav"));
            if nav && nav_href.is_none() {
                nav_href = Some(resolve_href(&dir, &href));
            }
            manifest.insert(id, (href, nav));
        }
    }
    let nav_path = nav_href?; // EPUB 2: this function does not apply
    let spine_paths: Vec<String> = open_tags(&opf, "itemref")
        .iter()
        .filter_map(|t| attr(t, "idref"))
        .filter_map(|id| manifest.get(&id))
        .filter(|(_, nav)| !nav)
        .map(|(h, _)| resolve_href(&dir, h))
        .collect();
    let spine: Vec<(String, String)> = spine_paths
        .into_iter()
        .filter_map(|p| zip_read(&mut z, &p).map(|h| (p, h)))
        .collect();
    let nav_html = zip_read(&mut z, &nav_path)?;
    let pg = paginate(&spine, Some((&nav_path, &nav_html)));
    for w in &pg.warnings {
        eprintln!("  ({}: {w})", path.display());
    }
    (pg.source != Source::None).then(|| pg.to_indexed())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nav(links: &[(&str, &str)]) -> String {
        let items: String = links
            .iter()
            .map(|(h, l)| format!("<li><a href=\"{h}\">{l}</a></li>"))
            .collect();
        format!(
            "<html><body><nav epub:type=\"toc\"><ol><li><a href=\"c1.xhtml\">Start</a></li></ol></nav><nav epub:type=\"page-list\" hidden=\"\"><ol>{items}</ol></nav></body></html>"
        )
    }

    const C1: &str = "<html><body><p>Alpha. <span id=\"p1\" epub:type=\"pagebreak\"/> Beta. <span id=\"p2\" epub:type=\"pagebreak\"/> Gamma.</p></body></html>";

    fn spine(html: &str) -> Vec<(String, String)> {
        vec![("OPS/c1.xhtml".to_string(), html.to_string())]
    }

    #[test]
    fn page_list_cuts_the_text() {
        let n = nav(&[("c1.xhtml#p1", "1"), ("c1.xhtml#p2", "2")]);
        let p = paginate(&spine(C1), Some(("OPS/nav.xhtml", &n)));
        assert_eq!(p.source, Source::PageList);
        assert!(p.front.contains("Alpha") && !p.front.contains("Beta"));
        let labels: Vec<(&str, usize)> = p
            .pages
            .iter()
            .map(|x| (x.label.as_str(), x.ordinal))
            .collect();
        assert_eq!(labels, [("1", 1), ("2", 2)]);
        assert!(p.pages[0].text.contains("Beta") && !p.pages[0].text.contains("Gamma"));
        assert!(p.pages[1].text.contains("Gamma"));
        assert_eq!(p.doc_pages(), 2);
        assert!(!p.front.contains('\u{E000}') && !p.pages[0].text.contains('\u{E001}'));
    }

    #[test]
    fn markers_with_aria_label_when_no_page_list() {
        let html = "<p>Alpha. <span role=\"doc-pagebreak\" aria-label=\"14\" id=\"x\"/> Beta. <span epub:type=\"pagebreak\" role=\"doc-pagebreak\"/> still beta.</p>";
        let p = paginate(
            &spine(html),
            Some(("OPS/nav.xhtml", "<nav epub:type=\"toc\"/>")),
        );
        assert_eq!(p.source, Source::Markers);
        assert_eq!(p.pages.len(), 1); // the unlabelled marker starts no page
        assert_eq!(p.pages[0].label, "14");
        assert!(p.pages[0].text.contains("Beta") && p.pages[0].text.contains("still beta"));
    }

    #[test]
    fn roman_and_arabic_labels() {
        let n = nav(&[("c1.xhtml#p1", "xii"), ("c1.xhtml#p2", "1")]);
        let p = paginate(&spine(C1), Some(("OPS/nav.xhtml", &n)));
        assert_eq!(p.doc_pages(), 1);
        let idx = p.to_indexed();
        assert_eq!(idx.iter().map(|x| x.0).collect::<Vec<_>>(), [0, 0, 1]);
    }

    #[test]
    fn no_pagination_means_none() {
        let p = paginate(
            &spine("<p>Plain text only.</p>"),
            Some(("OPS/nav.xhtml", "<nav epub:type=\"toc\"/>")),
        );
        assert_eq!(p.source, Source::None);
        assert!(p.pages.is_empty());
    }

    #[test]
    fn page_text_is_split_so_passages_cannot_cross() {
        let n = nav(&[("c1.xhtml#p1", "1"), ("c1.xhtml#p2", "2")]);
        let p = paginate(&spine(C1), Some(("OPS/nav.xhtml", &n)));
        let idx = p.to_indexed();
        assert_eq!(idx.len(), 3);
        assert!(
            idx.iter()
                .all(|(_, t)| !(t.contains("Beta") && t.contains("Gamma")))
        );
    }

    #[test]
    fn links_outside_the_spine_and_missing_ids() {
        let n = nav(&[
            ("gone.xhtml#a", "5"),
            ("c1.xhtml#nope", "6"),
            ("c1.xhtml#p2", "7"),
        ]);
        let p = paginate(&spine(C1), Some(("OPS/nav.xhtml", &n)));
        assert!(p.warnings.iter().any(|w| w.contains("gone.xhtml")));
        assert!(p.warnings.iter().any(|w| w.contains("nope")));
        let labels: Vec<&str> = p.pages.iter().map(|x| x.label.as_str()).collect();
        assert_eq!(labels, ["6", "7"]); // 6 starts at the file start, 7 at its marker
        assert!(p.pages[0].text.contains("Alpha") && p.pages[0].text.contains("Beta"));
        assert!(p.pages[1].text.contains("Gamma"));
    }

    #[test]
    fn repeated_labels_stay_separate_pages() {
        let html = "<p>One <i id=\"a\"/> two <i id=\"b\"/> three</p>";
        let n = nav(&[("c1.xhtml#a", "14"), ("c1.xhtml#b", "14")]);
        let p = paginate(&spine(html), Some(("OPS/nav.xhtml", &n)));
        assert_eq!(p.pages.len(), 2);
        assert_eq!((p.pages[0].ordinal, p.pages[1].ordinal), (1, 2));
        assert_eq!(p.to_indexed().iter().filter(|x| x.0 == 14).count(), 2);
    }

    #[test]
    fn first_page_list_wins_and_fragments_decode() {
        let n = "<nav epub:type=\"page-list\"><ol><li><a href=\"c1.xhtml#a%31\">9</a></li></ol></nav><nav epub:type=\"page-list\"><ol><li><a href=\"c1.xhtml#a1\">99</a></li></ol></nav>";
        let p = paginate(
            &spine("<p>x <i id=\"a1\"/> y</p>"),
            Some(("OPS/nav.xhtml", n)),
        );
        assert_eq!(p.pages.len(), 1);
        assert_eq!(p.pages[0].label, "9");
    }

    fn make_epub(path: &Path, files: &[(&str, &str)]) {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        for (n, c) in files {
            z.start_file(*n, o).unwrap();
            z.write_all(c.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn whole_file_goes_through_the_indexer_path() {
        let dir = std::env::temp_dir().join(format!("tezaur-epub-pages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let container =
            "<container><rootfiles><rootfile full-path=\"OPS/book.opf\"/></rootfiles></container>";
        let opf = "<package><manifest><item id=\"n\" href=\"nav.xhtml\" properties=\"nav\"/><item id=\"c\" href=\"c1.xhtml\"/></manifest><spine><itemref idref=\"c\"/></spine></package>";
        let n = nav(&[("c1.xhtml#p1", "1"), ("c1.xhtml#p2", "2")]);
        let e = dir.join("b.epub");
        make_epub(
            &e,
            &[
                ("META-INF/container.xml", container),
                ("OPS/book.opf", opf),
                ("OPS/nav.xhtml", &n),
                ("OPS/c1.xhtml", C1),
            ],
        );
        let pages = crate::ingest::read_pages(&e).unwrap();
        let by: Vec<(usize, bool)> = pages
            .iter()
            .map(|(p, t)| (*p, t.contains("Beta")))
            .collect();
        assert_eq!(by, vec![(0, false), (1, true), (2, false)]);
        let mut store = crate::index::Store::default();
        crate::ingest::ingest_file(&mut store, &e, &dir, false).unwrap();
        assert_eq!(store.docs[0].pages, 2);
        assert!(
            store
                .chunks
                .iter()
                .any(|c| c.page == 1 && c.text.contains("Beta"))
        );
        // no pagination: unchanged behaviour (one text, page 0)
        let plain = dir.join("p.epub");
        make_epub(
            &plain,
            &[
                ("META-INF/container.xml", container),
                ("OPS/book.opf", opf),
                ("OPS/nav.xhtml", "<nav epub:type=\"toc\"/>"),
                ("OPS/c1.xhtml", "<p>Just text.</p>"),
            ],
        );
        let pp = crate::ingest::read_pages(&plain).unwrap();
        assert_eq!(pp.len(), 1);
        assert_eq!(pp[0].0, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
