//! Minimal BibTeX reader (Zotero / Better BibTeX exports). Used only to make citations read
//! "Author (Year)" and to build a proper reference list. It never touches the index format:
//! the `.bib` files are read at run time, so editing metadata needs no re-indexing.

use crate::kinds::{self, Format, Genre};
use crate::names::{self, NameList};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub key: String,
    pub author: String,
    /// Author (or editor) field as written, braces and LaTeX intact, for name parsing.
    pub raw_author: String,
    pub year: String,
    pub title: String,
    pub container: String,
    pub doi: String,
    pub file: String,
    pub note: String,
    pub url: String,
    /// File the entry was read from (for reports).
    pub source: String,
    /// Other keys of the same work after unification (spec 07).
    pub aliases: Vec<String>,
    pub editor: String,
    pub publisher: String,
    pub volume: String,
    pub number: String,
    pub pages: String,
    pub address: String,
    pub edition: String,
    pub month: String,
    pub isbn: String,
    pub issn: String,
    pub howpublished: String,
    /// Entry type as read, with the format it came from (kept so exports can reproduce it).
    pub origin: Option<(Format, String)>,
    pub genre: Option<Genre>,
}

pub(crate) fn clean(s: &str) -> String {
    let ch: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < ch.len() {
        match ch[i] {
            '{' | '}' => i += 1,
            '\\' if i + 1 < ch.len() => {
                let n = ch[i + 1];
                if "'`\"^~=.".contains(n) {
                    i += 2; // accent mark: keep the letter, drop the mark
                } else if "&%_#$".contains(n) {
                    out.push(n);
                    i += 2;
                } else {
                    i += 1; // \command: skip its name, keep its braced argument
                    while i < ch.len() && ch[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn to_entry(r: &crate::bibtex::RawEntry) -> Entry {
    let mut e = Entry {
        key: r.key.clone(),
        origin: Some((Format::Bibtex, r.typ.clone())),
        genre: kinds::to_csl(Format::Bibtex, &r.typ).1,
        ..Default::default()
    };
    let g = |n: &str| r.get(n).map(|v| v.trim()).filter(|v| !v.is_empty());
    let c = |n: &str| g(n).map(clean).unwrap_or_default();
    if let Some(a) = g("author") {
        e.author = clean(a);
        e.raw_author = a.to_string();
    }
    if let Some(ed) = g("editor") {
        e.editor = clean(ed);
        if e.author.is_empty() {
            e.author = e.editor.clone();
            e.raw_author = ed.to_string();
        }
    }
    e.year = if let Some(y) = g("year") {
        clean(y)
    } else {
        c("date").chars().take(4).collect()
    };
    e.title = c("title");
    e.container = ["journal", "journaltitle", "booktitle"]
        .iter()
        .find_map(|n| g(n).map(clean))
        .or_else(|| {
            ["publisher", "school", "institution", "howpublished"]
                .iter()
                .find_map(|n| g(n).map(clean))
        })
        .unwrap_or_default();
    e.publisher = ["publisher", "school", "institution", "organization"]
        .iter()
        .find_map(|n| g(n).map(clean))
        .unwrap_or_default();
    e.doi = c("doi");
    e.note = c("note");
    e.url = c("url");
    e.file = g("file").unwrap_or("").to_string(); // raw: Windows paths must not be LaTeX-cleaned
    (e.volume, e.number, e.pages) = (c("volume"), c("number"), c("pages"));
    (e.address, e.edition, e.month) = (c("address"), c("edition"), c("month"));
    (e.isbn, e.issn, e.howpublished) = (c("isbn"), c("issn"), c("howpublished"));
    if let Some(t) = g("type")
        && let Some(gn) = kinds::genre_from_text(t)
    {
        e.genre = Some(gn);
    }
    e
}

/// Parses a `.bib` text; also returns reader warnings (undefined abbreviation, dangling
/// crossref, duplicate key, ...).
pub fn parse_with_warnings(text: &str) -> (Vec<Entry>, Vec<String>) {
    let f = crate::bibtex::read(text);
    (f.entries.iter().map(to_entry).collect(), f.warnings)
}

pub fn parse(text: &str) -> Vec<Entry> {
    parse_with_warnings(text).0
}

/// Words that mark an unbraced, comma-less name as an institution ("Council of Europe").
const ORG: [&str; 12] = [
    "of",
    "for",
    "the",
    "council",
    "university",
    "institute",
    "commission",
    "union",
    "association",
    "ministry",
    "organization",
    "organisation",
];

impl Entry {
    /// Citation surname of the first author (used to build generated keys).
    pub(crate) fn people_for_key(&self) -> Option<String> {
        self.people().people.first().map(|p| p.family())
    }

    /// Parsed authors (spec 04). A name without comma containing an institution word is kept
    /// whole as one literal name.
    fn people(&self) -> NameList {
        let src = if self.raw_author.is_empty() {
            &self.author
        } else {
            &self.raw_author
        };
        let mut l = names::parse_names(src);
        for p in &mut l.people {
            if p.inverted_form {
                continue;
            }
            let toks: Vec<&String> = p.first.iter().chain(&p.von).chain(&p.last).collect();
            let org = toks
                .iter()
                .any(|t| ORG.contains(&t.to_lowercase().as_str()));
            if org && toks.len() > 1 {
                let whole = toks
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                *p = names::Person {
                    last: vec![format!("{{{whole}}}")],
                    ..Default::default()
                };
            }
        }
        l
    }

    /// "Smith (2019)", "Smith & Jones (2019)", "Smith et al. (2019)".
    pub fn cite(&self) -> String {
        let l = self.people();
        let a = &l.people;
        let who = match a.len() {
            0 => String::new(),
            _ if l.et_al => format!("{} et al.", a[0].short()),
            1 => a[0].short(),
            2 => format!("{} & {}", a[0].short(), a[1].short()),
            _ => format!("{} et al.", a[0].short()),
        };
        match (who.is_empty(), self.year.is_empty()) {
            (true, _) if !self.title.is_empty() => self.title.clone(),
            (true, _) => self.key.clone(),
            (false, true) => who,
            (false, false) => format!("{who} ({})", self.year),
        }
    }

    /// Full reference line for the bibliography.
    pub fn reference(&self) -> String {
        let mut s = String::new();
        let l = self.people();
        if !l.people.is_empty() {
            let mut v: Vec<String> = l.people.iter().map(|p| p.inverted()).collect();
            if l.et_al {
                v.push("et al.".into());
            }
            s.push_str(&v.join("; "));
            s.push_str(". ");
        }
        if !self.year.is_empty() {
            s.push_str(&format!("({}). ", self.year));
        }
        if !self.title.is_empty() {
            s.push_str(&self.title);
            s.push_str(". ");
        }
        if !self.container.is_empty() {
            s.push_str(&self.container);
            s.push_str(". ");
        }
        if !self.doi.is_empty() {
            s.push_str(&format!("https://doi.org/{}", self.doi));
        }
        s.trim().to_string()
    }
}

impl Entry {
    /// CSL type of this entry: from the kept origin, else a container means a journal article.
    fn csl_type(&self) -> &'static str {
        match &self.origin {
            Some((f, t)) => kinds::to_csl(*f, t).0,
            None if self.container.is_empty() => "document",
            None => "article-journal",
        }
    }

    fn out_type(&self, target: Format) -> String {
        let origin = self.origin.as_ref().map(|(f, t)| (*f, t.as_str()));
        kinds::emit(target, self.csl_type(), self.genre, origin)
    }

    /// RIS record (Zotero, Mendeley, EndNote import it). Type from the crosswalk (spec 01).
    pub fn to_ris(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("TY  - {}\n", self.out_type(Format::Ris)));
        s.push_str(&format!("ID  - {}\n", self.key));
        for p in &self.people().people {
            s.push_str(&format!("AU  - {}\n", p.inverted()));
        }
        if !self.title.is_empty() {
            s.push_str(&format!("TI  - {}\n", self.title));
        }
        if !self.container.is_empty() {
            s.push_str(&format!("T2  - {}\n", self.container));
        }
        if !self.year.is_empty() {
            s.push_str(&format!("PY  - {}\n", self.year));
        }
        if !self.doi.is_empty() {
            s.push_str(&format!("DO  - {}\n", self.doi));
        }
        s.push_str("ER  - \n");
        s
    }

    /// CSL JSON item (pandoc --citeproc, Zotero, Word/LibreOffice plugins).
    pub fn to_csl(&self) -> serde_json::Value {
        let authors: Vec<serde_json::Value> =
            self.people().people.iter().map(|p| p.to_csl()).collect();
        let mut v = serde_json::json!({
            "id": self.key,
            "type": self.out_type(Format::Csl),
            "title": self.title,
            "author": authors,
        });
        if !self.container.is_empty() {
            v["container-title"] = self.container.clone().into();
        }
        if let Ok(y) = self.year.parse::<i64>() {
            v["issued"] = serde_json::json!({"date-parts": [[y]]});
        }
        if !self.doi.is_empty() {
            v["DOI"] = self.doi.clone().into();
        }
        for (k, x) in [
            ("volume", &self.volume),
            ("issue", &self.number),
            ("page", &self.pages),
            ("publisher", &self.publisher),
            ("publisher-place", &self.address),
            ("edition", &self.edition),
            ("ISBN", &self.isbn),
            ("ISSN", &self.issn),
        ] {
            if !x.is_empty() {
                v[k] = x.clone().into();
            }
        }
        if !self.url.is_empty() {
            v["URL"] = self.url.clone().into();
        }
        if !self.note.is_empty() {
            v["note"] = self.note.clone().into();
        }
        v
    }

    /// BibTeX entry with the fields the reader keeps. The container goes to the field its type
    /// uses: journal, booktitle, school, institution or publisher.
    pub fn to_bibtex(&self) -> String {
        let kind = self.out_type(Format::Bibtex);
        let mut s = format!("@{kind}{{{},\n", self.key);
        let mut f = |k: &str, v: &str| {
            if !v.is_empty() {
                s.push_str(&format!("  {k} = {{{v}}},\n"));
            }
        };
        f("author", &self.author);
        f("editor", &self.editor);
        f("title", &self.title);
        let cont_key = match kind.as_str() {
            "article" => "journal",
            "inproceedings" | "conference" | "incollection" | "inbook" => "booktitle",
            "mastersthesis" | "phdthesis" => "school",
            "techreport" | "manual" => "institution",
            "book" | "proceedings" | "booklet" => "publisher",
            _ => "howpublished",
        };
        f(cont_key, &self.container);
        if cont_key != "publisher" && self.publisher != self.container {
            f("publisher", &self.publisher);
        }
        f("year", &self.year);
        f("volume", &self.volume);
        f("number", &self.number);
        f("pages", &self.pages);
        f("address", &self.address);
        f("edition", &self.edition);
        f("doi", &self.doi);
        f("isbn", &self.isbn);
        f("issn", &self.issn);
        f("url", &self.url);
        f("note", &self.note);
        s.push_str("}\n");
        s
    }
}

#[derive(Default)]
pub struct BibMap {
    pub entries: Vec<Entry>,
    /// Reader warnings, `file: message`.
    pub warnings: Vec<String>,
}

impl BibMap {
    /// Key of the entry that `k` names, following aliases left by unification.
    pub fn canonical_key(&self, k: &str) -> String {
        self.entries
            .iter()
            .find(|e| e.aliases.iter().any(|a| a.eq_ignore_ascii_case(k)))
            .map(|e| e.key.clone())
            .unwrap_or_else(|| k.to_string())
    }

    /// Merges duplicates by identifier (no hard conflicts only); see `dedup`.
    pub fn unified(mut self) -> BibMap {
        self.entries = crate::dedup::unify(&self.entries).0;
        self
    }
}

impl BibMap {
    /// Reads every `*.bib` and `*.ris` (up to `max_depth` levels) under `dirs`, plus explicitly named files.
    pub fn load(dirs: &[PathBuf], files: &[PathBuf], max_depth: usize) -> BibMap {
        let mut paths: Vec<PathBuf> = files.to_vec();
        for d in dirs {
            for e in WalkDir::new(d).max_depth(max_depth).into_iter().flatten() {
                if e.file_type().is_file()
                    && e.path()
                        .extension()
                        .map(|x| x.eq_ignore_ascii_case("bib") || x.eq_ignore_ascii_case("ris"))
                        .unwrap_or(false)
                    || e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(".csl.json")
                {
                    paths.push(e.path().to_path_buf());
                }
            }
        }
        let mut paths: Vec<PathBuf> = paths
            .into_iter()
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
            .collect();
        paths.sort();
        paths.dedup();
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        for p in paths {
            match std::fs::read(&p) {
                Ok(b) => {
                    let text = String::from_utf8_lossy(&b);
                    let is_ris = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ris"));
                    let is_json = p
                        .extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("json"));
                    let (es, ws) = if is_json {
                        crate::csljson::parse(&text).unwrap_or_else(|err| (vec![], vec![err]))
                    } else if is_ris {
                        crate::ris::parse(&text)
                    } else {
                        parse_with_warnings(&text)
                    };
                    let src = p.file_name().map(|n| n.to_string_lossy().into_owned());
                    entries.extend(es.into_iter().map(|mut e| {
                        e.source = src.clone().unwrap_or_default();
                        e
                    }));
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
                    warnings.extend(
                        ws.into_iter()
                            .map(|w| format!("{}: {w}", name.as_deref().unwrap_or("?"))),
                    );
                }
                Err(e) => eprintln!("warning: cannot read {}: {e}", p.display()),
            }
        }
        BibMap { entries, warnings }
    }

    /// Match by citation key == file name (without extension), or by the BibTeX `file` field.
    pub fn find(&self, doc_path: &str) -> Option<&Entry> {
        let p = Path::new(doc_path);
        let stem = p.file_stem()?.to_string_lossy().to_lowercase();
        let fname = p.file_name()?.to_string_lossy().to_lowercase();
        self.entries
            .iter()
            .find(|e| e.key.to_lowercase() == stem)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|e| !e.file.is_empty() && e.file.to_lowercase().contains(&fname))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
@comment{ignored}
@article{smith2019,
  author = {Smith, John and Doe, Jane and Roe, Rick},
  title = {Recognition of {Foreign} Qualifications: a {\"u}ber-review},
  journal = "Journal of Education",
  year = 2019,
  doi = {10.1000/xyz},
  file = {Full Text:C\:\\Users\\a\\Zotero\\storage\\AB12\\smith_review.pdf:application/pdf}
}
@book{lisbon97,
  author = {Council of Europe},
  title = {Lisbon Recognition Convention},
  publisher = {Council of Europe},
  date = {1997-04-11}
}
"#;

    #[test]
    fn parses_entries_and_cleans_latex() {
        let e = parse(SAMPLE);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].key, "smith2019");
        assert_eq!(e[0].year, "2019");
        assert_eq!(e[0].container, "Journal of Education");
        assert!(
            e[0].title
                .starts_with("Recognition of Foreign Qualifications")
        );
        assert!(e[0].title.contains("uber-review"));
        assert_eq!(e[1].year, "1997");
        assert_eq!(e[1].container, "Council of Europe");
    }

    #[test]
    fn formats_citations() {
        let e = parse(SAMPLE);
        assert_eq!(e[0].cite(), "Smith et al. (2019)");
        assert_eq!(e[1].cite(), "Council of Europe (1997)");
        let person = Entry {
            author: "Jane Roe".into(),
            year: "2001".into(),
            ..Default::default()
        };
        assert_eq!(person.cite(), "Roe (2001)");
        let two = Entry {
            author: "A, X and B, Y".into(),
            year: "2020".into(),
            ..Default::default()
        };
        assert_eq!(two.cite(), "A & B (2020)");
        assert!(
            e[0].reference().contains("(2019)") && e[0].reference().contains("doi.org/10.1000/xyz")
        );
    }

    #[test]
    fn matches_by_key_or_file_field() {
        let m = BibMap {
            entries: parse(SAMPLE),
            ..Default::default()
        };
        assert_eq!(m.find("Library/sub/Lisbon97.pdf").unwrap().key, "lisbon97");
        assert_eq!(m.find("Library/smith_review.pdf").unwrap().key, "smith2019");
        assert!(m.find("Library/unknown.pdf").is_none());
    }

    #[test]
    fn ris_and_csl_roundtrip() {
        let e = &parse(
            "@article{smith2019, author={Smith, Jane and Doe, John}, title={On proof}, journal={Law Rev}, year={2019}, doi={10.1/x}}",
        )[0];
        let ris = e.to_ris();
        assert!(
            ris.starts_with("TY  - JOUR\n")
                && ris.contains("AU  - Smith, Jane\n")
                && ris.contains("PY  - 2019")
                && ris.ends_with("ER  - \n")
        );
        let c = e.to_csl();
        assert_eq!(c["id"], "smith2019");
        assert_eq!(c["author"][0]["family"], "Smith");
        assert_eq!(c["issued"]["date-parts"][0][0], 2019);
        assert_eq!(parse(&e.to_bibtex())[0].title, "On proof");
    }

    #[test]
    fn entry_type_survives_exports() {
        let e = &parse("@phdthesis{t1, author={A, B}, title={T}, school={U}, year={2020}}")[0];
        assert!(e.to_ris().starts_with("TY  - THES\n"));
        assert_eq!(e.to_csl()["type"], "thesis");
        assert!(e.to_bibtex().starts_with("@phdthesis{t1"));
        let m = &parse("@thesis{t2, title={T}, type={Master's thesis}}")[0];
        assert_eq!(m.genre, Some(Genre::Master));
        let u = &parse("@software{s1, title={X}}")[0];
        assert!(u.to_bibtex().starts_with("@misc{") && u.to_ris().starts_with("TY  - GEN"));
        assert_eq!(u.to_csl()["type"], "document");
        let man = &parse("@manual{m1, title={X}}")[0];
        assert_eq!(man.to_csl()["type"], "report");
        assert!(man.to_bibtex().starts_with("@manual{"));
    }

    #[test]
    fn names_flow_into_citations_and_csl() {
        let e = &parse(
            "@book{b, author={von Beethoven, Ludwig and Doe, Jr., John}, title={T}, year={1800}}",
        )[0];
        assert_eq!(e.cite(), "von Beethoven & Doe (1800)");
        let c = e.to_csl();
        assert_eq!(c["author"][0]["family"], "Beethoven");
        assert_eq!(c["author"][0]["non-dropping-particle"], "von");
        assert_eq!(c["author"][1]["suffix"], "Jr.");
        let o = &parse("@misc{o, author={Doe, John and others}, year={2001}}")[0];
        assert_eq!(o.cite(), "Doe et al. (2001)");
        assert!(!o.to_ris().contains("others"));
        let b = &parse("@misc{q, author={{Barnes and Noble, Inc}}, year={2001}}")[0];
        assert_eq!(b.cite(), "Barnes and Noble, Inc (2001)");
        assert_eq!(b.to_csl()["author"][0]["literal"], "Barnes and Noble, Inc");
    }

    #[test]
    fn full_reader_feeds_entries() {
        let (e, w) = parse_with_warnings(
            "@string{tl = \"Law Review\"}\n@inproceedings{c, crossref = {p1}, title = {T}, author={A, B}}\n@proceedings{p1, booktitle = {Proc. X}, year = 2005}\n@article{a, journal = tl # \" II\", pages={1--5}, month = jul, editor={E, F}}\n@misc{z, journal = zzz}",
        );
        assert_eq!(e[0].container, "Proc. X");
        assert_eq!(e[0].year, "2005");
        assert_eq!(e[0].origin.as_ref().unwrap().1, "inproceedings");
        assert_eq!(e[2].container, "Law Review II");
        assert_eq!((e[2].pages.as_str(), e[2].month.as_str()), ("1--5", "July"));
        assert_eq!(e[2].editor, "E, F");
        assert_eq!(e[3].container, "zzz");
        assert!(w.iter().any(|x| x.contains("zzz")));
    }
}
