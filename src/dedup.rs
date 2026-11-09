//! Duplicate references by identifier (spec 07): same canonical DOI, or same ISBN-13 for whole
//! books. Only identifiers decide; no fuzzy title matching. Conflicting metadata (different year,
//! or titles sharing fewer than half of their tokens) is reported and never merged.

use crate::bib::Entry;
use crate::ids::{self, IdKind, Verdict};
use crate::kinds::{self, Format};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Default)]
pub struct Group {
    /// Positions in the input list, ascending (load order).
    pub indices: Vec<usize>,
    pub keys: Vec<String>,
    pub sources: Vec<String>,
    /// "same DOI" / "same ISBN".
    pub reasons: Vec<&'static str>,
    /// Hard conflicts (year / title); such a group is not unified.
    pub conflicts: Vec<String>,
    /// Other fields whose non-empty values differ (the group is still unified).
    pub field_diffs: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub groups: Vec<Group>,
    /// Identifiers present but unusable (bad syntax / bad check digit): `key: reason`.
    pub invalid: Vec<String>,
    /// Entries with no usable identifier at all.
    pub unverifiable: usize,
    /// Same key on entries that are not the same work.
    pub key_collisions: Vec<String>,
}

fn is_whole_book(e: &Entry) -> bool {
    let (csl, genre) = match &e.origin {
        Some((f, t)) => (kinds::to_csl(*f, t).0, kinds::to_csl(*f, t).1),
        None => (
            if e.container.is_empty() {
                "document"
            } else {
                "article-journal"
            },
            None,
        ),
    };
    let origin = e.origin.as_ref().map(|(f, t)| (*f, t.as_str()));
    let bt = kinds::emit(Format::Bibtex, csl, genre.or(e.genre), origin);
    matches!(bt.as_str(), "book" | "proceedings" | "manual" | "booklet")
}

fn isbn13_of(e: &Entry) -> Result<Option<String>, String> {
    if e.isbn.trim().is_empty() {
        return Ok(None);
    }
    match ids::check_isbn(&e.isbn) {
        Some(f) if f.verdict == Verdict::Valid => Ok(Some(if f.kind == IdKind::Isbn10 {
            ids::isbn10_to_13(&f.canonical).unwrap_or(f.canonical)
        } else {
            f.canonical
        })),
        Some(f) if f.verdict == Verdict::BadChecksum => Err("ISBN with a wrong check digit".into()),
        _ => Err("ISBN is not well formed".into()),
    }
}

fn find(p: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while p[r] != r {
        r = p[r];
    }
    let mut c = x;
    while p[c] != r {
        let n = p[c];
        p[c] = r;
        c = n;
    }
    r
}

fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn compared_fields(e: &Entry) -> [(&'static str, &str); 17] {
    [
        ("author", &e.author),
        ("editor", &e.editor),
        ("title", &e.title),
        ("year", &e.year),
        ("container", &e.container),
        ("publisher", &e.publisher),
        ("doi", &e.doi),
        ("isbn", &e.isbn),
        ("issn", &e.issn),
        ("url", &e.url),
        ("volume", &e.volume),
        ("number", &e.number),
        ("pages", &e.pages),
        ("address", &e.address),
        ("edition", &e.edition),
        ("month", &e.month),
        ("howpublished", &e.howpublished),
    ]
}

/// Finds duplicate groups. Does not modify anything.
pub fn analyze(entries: &[Entry]) -> Report {
    let n = entries.len();
    let mut rep = Report::default();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut by_doi: HashMap<String, usize> = HashMap::new();
    let mut by_isbn: HashMap<String, usize> = HashMap::new();
    let mut reasons: Vec<BTreeSet<&'static str>> = vec![BTreeSet::new(); n];
    for (i, e) in entries.iter().enumerate() {
        let mut usable = false;
        if !e.doi.trim().is_empty() {
            match ids::normalize_doi(&e.doi) {
                Some(f) => {
                    usable = true;
                    match by_doi.get(&f.canonical) {
                        Some(&j) => {
                            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                            parent[a] = b;
                            reasons[i].insert("same DOI");
                            reasons[j].insert("same DOI");
                        }
                        None => {
                            by_doi.insert(f.canonical, i);
                        }
                    }
                }
                None => rep.invalid.push(format!(
                    "{}: DOI `{}` is not valid DOI syntax",
                    e.key, e.doi
                )),
            }
        }
        match isbn13_of(e) {
            Ok(Some(isbn)) if is_whole_book(e) => {
                usable = true;
                match by_isbn.get(&isbn) {
                    Some(&j) => {
                        let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                        parent[a] = b;
                        reasons[i].insert("same ISBN");
                        reasons[j].insert("same ISBN");
                    }
                    None => {
                        by_isbn.insert(isbn, i);
                    }
                }
            }
            Ok(Some(_)) => usable = true, // a chapter's ISBN does not decide, but it is an identifier
            Ok(None) => {}
            Err(why) => rep.invalid.push(format!("{}: {why} (`{}`)", e.key, e.isbn)),
        }
        if !usable {
            rep.unverifiable += 1;
        }
    }
    let mut roots: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        match roots.iter_mut().find(|(x, _)| *x == r) {
            Some((_, v)) => v.push(i),
            None => roots.push((r, vec![i])),
        }
    }
    for (_, idx) in roots.into_iter().filter(|(_, v)| v.len() > 1) {
        let first = &entries[idx[0]];
        let mut g = Group {
            keys: idx.iter().map(|&i| entries[i].key.clone()).collect(),
            sources: idx.iter().map(|&i| entries[i].source.clone()).collect(),
            reasons: idx
                .iter()
                .flat_map(|&i| reasons[i].iter().copied())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            indices: idx.clone(),
            ..Default::default()
        };
        let ft: Vec<String> = crate::index::tokenize(&first.title);
        for &i in &idx[1..] {
            let e = &entries[i];
            if !first.year.is_empty() && !e.year.is_empty() && first.year != e.year {
                g.conflicts.push(format!(
                    "year differs: {} ({}) vs {} ({})",
                    first.year, first.key, e.year, e.key
                ));
            }
            let et = crate::index::tokenize(&e.title);
            if !ft.is_empty() && !et.is_empty() {
                let common = ft.iter().filter(|t| et.contains(t)).count();
                if (common as f32) < ft.len().max(et.len()) as f32 / 2.0 {
                    g.conflicts.push(format!(
                        "titles share too few words: `{}` vs `{}`",
                        first.title, e.title
                    ));
                }
            }
            for ((name, a), (_, b)) in compared_fields(first).iter().zip(compared_fields(e)) {
                if !a.trim().is_empty()
                    && !b.trim().is_empty()
                    && norm(a) != norm(b)
                    && !matches!(*name, "year" | "title")
                {
                    g.field_diffs.push(format!("{name}: `{a}` vs `{b}`"));
                }
            }
        }
        rep.groups.push(g);
    }
    // same key on different works
    let in_group: BTreeSet<usize> = rep
        .groups
        .iter()
        .flat_map(|g| g.indices.iter().copied())
        .collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let k = e.key.to_lowercase();
        if let Some(&j) = seen.get(&k) {
            let same = rep
                .groups
                .iter()
                .any(|g| g.indices.contains(&i) && g.indices.contains(&j));
            if !same && (!in_group.contains(&i) || !in_group.contains(&j) || !same) {
                rep.key_collisions.push(format!(
                    "key `{}` is used for different works ({} and {})",
                    e.key, entries[j].source, e.source
                ));
            }
        } else {
            seen.insert(k, i);
        }
    }
    rep
}

fn merged(group: &[&Entry]) -> Entry {
    let filled = |e: &Entry| {
        compared_fields(e)
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .count()
    };
    let mut base = 0;
    for (i, e) in group.iter().enumerate() {
        if filled(e) > filled(group[base]) {
            base = i;
        }
    }
    let mut out = group[0].clone();
    out.aliases = group[1..].iter().map(|e| e.key.clone()).collect();
    macro_rules! pick {
        ($($f:ident),*) => {$({
            let b = group[base].$f.trim();
            let cands: Vec<&str> = group.iter().map(|e| e.$f.trim()).filter(|v| !v.is_empty()).collect();
            let differs = cands.iter().any(|v| norm(v) != norm(cands[0]));
            out.$f = if cands.is_empty() {
                String::new()
            } else if !b.is_empty() && !differs {
                b.to_string()
            } else {
                cands[0].to_string() // conflict: the first entry's value stays
            };
        })*};
    }
    pick!(
        title,
        year,
        container,
        publisher,
        doi,
        isbn,
        issn,
        url,
        volume,
        number,
        pages,
        address,
        edition,
        month,
        howpublished,
        note
    );
    // author / editor travel with their raw (brace-preserving) text
    let a = group.iter().find(|e| !e.author.is_empty());
    if let Some(a) = a {
        let src = if !group[base].author.is_empty()
            && !group
                .iter()
                .any(|e| !e.author.is_empty() && norm(&e.author) != norm(&group[base].author))
        {
            group[base]
        } else {
            a
        };
        out.author = src.author.clone();
        out.raw_author = src.raw_author.clone();
    }
    if let Some(e) = group.iter().find(|e| !e.editor.is_empty()) {
        out.editor = e.editor.clone();
    }
    if out.file.is_empty()
        && let Some(f) = group.iter().find(|e| !e.file.is_empty())
    {
        out.file = f.file.clone();
    }
    out
}

/// Unifies every group without hard conflicts: one entry per group (key of the first, other keys
/// kept as aliases). Returns the new list and the report.
pub fn unify(entries: &[Entry]) -> (Vec<Entry>, Report) {
    let rep = analyze(entries);
    let mut drop: BTreeSet<usize> = BTreeSet::new();
    let mut replace: HashMap<usize, Entry> = HashMap::new();
    for g in rep.groups.iter().filter(|g| g.conflicts.is_empty()) {
        let members: Vec<&Entry> = g.indices.iter().map(|&i| &entries[i]).collect();
        replace.insert(g.indices[0], merged(&members));
        drop.extend(g.indices[1..].iter().copied());
    }
    let out = entries
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop.contains(i))
        .map(|(i, e)| replace.remove(&i).unwrap_or_else(|| e.clone()))
        .collect();
    (out, rep)
}

impl Report {
    pub fn render(&self, total: usize) -> String {
        let mut s = format!(
            "{} entries, {} duplicate group(s), {} without a usable identifier\n",
            total,
            self.groups.len(),
            self.unverifiable
        );
        for g in &self.groups {
            s.push_str(&format!(
                "- {} [{}] from {}\n",
                g.keys.join(", "),
                g.reasons.join(" + "),
                g.sources.join(", ")
            ));
            for c in &g.conflicts {
                s.push_str(&format!("    CONFLICT (not unified): {c}\n"));
            }
            for d in &g.field_diffs {
                s.push_str(&format!("    differs: {d}\n"));
            }
        }
        for i in &self.invalid {
            s.push_str(&format!("invalid identifier: {i}\n"));
        }
        for k in &self.key_collisions {
            s.push_str(&format!("key collision: {k}\n"));
        }
        s
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "groups": self.groups.iter().map(|g| serde_json::json!({
                "keys": g.keys, "sources": g.sources, "reasons": g.reasons,
                "conflicts": g.conflicts, "field_diffs": g.field_diffs,
            })).collect::<Vec<_>>(),
            "invalid": self.invalid,
            "unverifiable": self.unverifiable,
            "key_collisions": self.key_collisions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(src: &str) -> Entry {
        crate::bib::parse(src).remove(0)
    }

    #[test]
    fn same_doi_ignoring_ascii_case() {
        let l = vec![
            e("@article{a, title={Proof}, year={2019}, doi={10.1000/ABC}, journal={J}}"),
            e("@article{b, title={Proof}, year={2019}, doi={10.1000/abc}, journal={J}}"),
        ];
        let r = analyze(&l);
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].reasons, vec!["same DOI"]);
        assert!(r.groups[0].conflicts.is_empty());
    }

    #[test]
    fn same_isbn_for_whole_books_only() {
        let l = vec![
            e("@book{a, title={Atlas}, year={2000}, isbn={978-92-95055-12-4}}"),
            e("@book{b, title={Atlas}, year={2000}, isbn={9789295055124}}"),
        ];
        assert_eq!(analyze(&l).groups[0].reasons, vec!["same ISBN"]);
        let m = vec![
            e("@book{a, title={Atlas}, year={2000}, isbn={9789295055124}}"),
            e(
                "@incollection{c, title={Chapter}, year={2000}, isbn={9789295055124}, booktitle={Atlas}}",
            ),
        ];
        assert!(analyze(&m).groups.is_empty());
        // ISBN-10 is converted first
        let t = vec![
            e("@book{a, title={X}, isbn={0-306-40615-2}}"),
            e("@book{b, title={X}, isbn={9780306406157}}"),
        ];
        assert_eq!(analyze(&t).groups.len(), 1);
    }

    #[test]
    fn year_conflict_is_reported_not_unified() {
        let l = vec![
            e("@article{a, title={Proof}, year={2019}, doi={10.1000/abc}}"),
            e("@article{b, title={Proof}, year={2017}, doi={10.1000/abc}}"),
        ];
        let (out, r) = unify(&l);
        assert_eq!(r.groups[0].conflicts.len(), 1);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn unify_keeps_aliases_and_fills_gaps() {
        let l = vec![
            e("@article{a, title={Proof}, doi={10.1000/abc}}"),
            e(
                "@article{b, title={Proof}, year={2019}, doi={10.1000/ABC}, journal={Law Rev}, author={Smith, Jane}}",
            ),
            e("@misc{z, title={Other}}"),
        ];
        let (out, r) = unify(&l);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].key, "a");
        assert_eq!(out[0].aliases, vec!["b"]);
        assert_eq!(
            (out[0].year.as_str(), out[0].container.as_str()),
            ("2019", "Law Rev")
        );
        assert_eq!(out[0].cite(), "Smith (2019)");
        assert_eq!(r.unverifiable, 1);
    }

    #[test]
    fn invalid_identifiers_do_not_group() {
        let l = vec![
            e("@book{a, title={X}, isbn={978-92-95055-12-5}}"),
            e("@book{b, title={X}, isbn={978-92-95055-12-5}}"),
            e("@article{c, title={Y}, doi={11.1000/abc}}"),
            e("@article{d, title={Y}, doi={11.1000/abc}}"),
        ];
        let r = analyze(&l);
        assert!(r.groups.is_empty());
        assert_eq!(r.invalid.len(), 4);
        assert_eq!(r.unverifiable, 4);
    }

    #[test]
    fn transitive_groups_and_key_collisions() {
        let l = vec![
            e("@book{a, title={T}, doi={10.1000/q}}"),
            e("@book{b, title={T}, doi={10.1000/q}, isbn={9780306406157}}"),
            e("@book{c, title={T}, isbn={0-306-40615-2}}"),
            e("@misc{a, title={Entirely different}, doi={10.1000/other}}"),
        ];
        let r = analyze(&l);
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].keys, vec!["a", "b", "c"]);
        assert_eq!(r.key_collisions.len(), 1);
        assert!(r.render(4).contains("same DOI + same ISBN"));
    }

    #[test]
    fn alias_resolves_to_the_unified_entry() {
        let map = crate::bib::BibMap {
            entries: vec![
                e("@article{a, title={Proof}, year={2019}, doi={10.1000/abc}}"),
                e("@article{b, title={Proof}, year={2019}, doi={10.1000/ABC}}"),
            ],
            ..Default::default()
        }
        .unified();
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.canonical_key("B"), "a");
        assert_eq!(map.canonical_key("a"), "a");
        assert_eq!(map.canonical_key("zzz"), "zzz");
    }
}
