//! Minimal BibTeX reader (Zotero / Better BibTeX exports). Used only to make citations read
//! "Author (Year)" and to build a proper reference list. It never touches the index format:
//! the `.bib` files are read at run time, so editing metadata needs no re-indexing.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub key: String,
    pub author: String,
    pub year: String,
    pub title: String,
    pub container: String,
    pub doi: String,
    pub file: String,
}

fn clean(s: &str) -> String {
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

fn parse_entry(body: &str) -> Option<Entry> {
    let (key, rest) = body.split_once(',')?;
    let c: Vec<char> = rest.chars().collect();
    let mut e = Entry {
        key: key.trim().to_string(),
        ..Default::default()
    };
    let mut i = 0;
    while i < c.len() {
        while i < c.len() && (c[i].is_whitespace() || c[i] == ',') {
            i += 1;
        }
        let ns = i;
        while i < c.len() && (c[i].is_alphanumeric() || c[i] == '-' || c[i] == '_') {
            i += 1;
        }
        if i == ns {
            i += 1;
            continue;
        }
        let name: String = c[ns..i].iter().collect::<String>().to_lowercase();
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        if i >= c.len() || c[i] != '=' {
            continue;
        }
        i += 1;
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        let mut val = String::new();
        if i < c.len() && c[i] == '{' {
            let mut d = 0;
            while i < c.len() {
                match c[i] {
                    '{' => d += 1,
                    '}' => d -= 1,
                    _ => {}
                }
                if d == 0 {
                    i += 1;
                    break;
                }
                val.push(c[i]);
                i += 1;
            }
            if !val.is_empty() {
                val.remove(0); // drop the opening brace
            }
        } else if i < c.len() && c[i] == '"' {
            i += 1;
            let mut d = 0;
            while i < c.len() && !(c[i] == '"' && d == 0) {
                match c[i] {
                    '{' => d += 1,
                    '}' => d -= 1,
                    _ => {}
                }
                val.push(c[i]);
                i += 1;
            }
            i += 1;
        } else {
            while i < c.len() && c[i] != ',' {
                val.push(c[i]);
                i += 1;
            }
        }
        let v = clean(val.trim());
        match name.as_str() {
            "author" => e.author = v,
            "editor" if e.author.is_empty() => e.author = v,
            "year" => e.year = v,
            "date" if e.year.is_empty() => e.year = v.chars().take(4).collect(),
            "title" => e.title = v,
            "journal" | "journaltitle" | "booktitle" => e.container = v,
            "publisher" | "school" | "institution" if e.container.is_empty() => e.container = v,
            "doi" => e.doi = v,
            "file" => e.file = val.trim().to_string(), // raw: Windows paths must not be LaTeX-cleaned
            _ => {}
        }
    }
    Some(e)
}

pub fn parse(text: &str) -> Vec<Entry> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] != '@' {
            i += 1;
            continue;
        }
        i += 1;
        let ts = i;
        while i < c.len() && c[i].is_alphanumeric() {
            i += 1;
        }
        let typ: String = c[ts..i].iter().collect::<String>().to_lowercase();
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        if i >= c.len() || c[i] != '{' {
            continue;
        }
        let start = i + 1;
        let mut depth = 0;
        while i < c.len() {
            match c[i] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            i += 1;
            if depth == 0 {
                break;
            }
        }
        if matches!(typ.as_str(), "comment" | "string" | "preamble") || i < start + 1 {
            continue;
        }
        let body: String = c[start..i - 1].iter().collect();
        if let Some(e) = parse_entry(&body) {
            out.push(e);
        }
    }
    out
}

fn last_name(a: &str) -> String {
    let a = a.trim();
    match a.split_once(',') {
        Some((last, _)) => last.trim().to_string(),
        None => {
            // Institutional authors ("Council of Europe") are cited whole, people by surname.
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
            let lower = a.to_lowercase();
            if lower.split_whitespace().any(|w| ORG.contains(&w)) {
                a.to_string()
            } else {
                a.split_whitespace().last().unwrap_or(a).to_string()
            }
        }
    }
}

impl Entry {
    fn authors(&self) -> Vec<&str> {
        self.author
            .split(" and ")
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect()
    }

    /// "Smith (2019)", "Smith & Jones (2019)", "Smith et al. (2019)".
    pub fn cite(&self) -> String {
        let a = self.authors();
        let who = match a.len() {
            0 => String::new(),
            1 => last_name(a[0]),
            2 => format!("{} & {}", last_name(a[0]), last_name(a[1])),
            _ => format!("{} et al.", last_name(a[0])),
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
        if !self.author.is_empty() {
            s.push_str(&self.authors().join("; "));
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
    /// RIS record (Zotero, Mendeley, EndNote import it). Entry type is not kept by the reader:
    /// JOUR when a container is known, GEN otherwise.
    pub fn to_ris(&self) -> String {
        let mut s = String::new();
        s.push_str(if self.container.is_empty() {
            "TY  - GEN\n"
        } else {
            "TY  - JOUR\n"
        });
        s.push_str(&format!("ID  - {}\n", self.key));
        for a in self.authors() {
            s.push_str(&format!("AU  - {a}\n"));
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
        let authors: Vec<serde_json::Value> = self
            .authors()
            .iter()
            .map(|a| {
                if let Some((fam, giv)) = a.split_once(',') {
                    serde_json::json!({"family": fam.trim(), "given": giv.trim()})
                } else {
                    let parts: Vec<&str> = a.split_whitespace().collect();
                    if parts.len() >= 2 {
                        serde_json::json!({"family": parts[parts.len()-1], "given": parts[..parts.len()-1].join(" ")})
                    } else {
                        serde_json::json!({"literal": a})
                    }
                }
            })
            .collect();
        let mut v = serde_json::json!({
            "id": self.key,
            "type": if self.container.is_empty() { "document" } else { "article-journal" },
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
        v
    }

    /// Minimal BibTeX (only the fields the reader keeps).
    pub fn to_bibtex(&self) -> String {
        let kind = if self.container.is_empty() {
            "misc"
        } else {
            "article"
        };
        let mut s = format!("@{kind}{{{},\n", self.key);
        let mut f = |k: &str, v: &str| {
            if !v.is_empty() {
                s.push_str(&format!("  {k} = {{{v}}},\n"));
            }
        };
        f("author", &self.author);
        f("title", &self.title);
        f("journal", &self.container);
        f("year", &self.year);
        f("doi", &self.doi);
        s.push_str("}\n");
        s
    }
}

#[derive(Default)]
pub struct BibMap {
    pub entries: Vec<Entry>,
}

impl BibMap {
    /// Reads every `*.bib` (up to `max_depth` levels) under `dirs`, plus explicitly named files.
    pub fn load(dirs: &[PathBuf], files: &[PathBuf], max_depth: usize) -> BibMap {
        let mut paths: Vec<PathBuf> = files.to_vec();
        for d in dirs {
            for e in WalkDir::new(d).max_depth(max_depth).into_iter().flatten() {
                if e.file_type().is_file()
                    && e.path()
                        .extension()
                        .map(|x| x.eq_ignore_ascii_case("bib"))
                        .unwrap_or(false)
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
        for p in paths {
            match std::fs::read(&p) {
                Ok(b) => entries.extend(parse(&String::from_utf8_lossy(&b))),
                Err(e) => eprintln!("warning: cannot read {}: {e}", p.display()),
            }
        }
        BibMap { entries }
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
}
