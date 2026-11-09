//! Reference-type crosswalk between BibTeX entry types, CSL JSON `type` values and RIS `TY` codes
//! (spec 01). CSL JSON is the hub: every foreign type is first mapped to a CSL type (plus an
//! optional thesis "genre"), then emitted in the target format. When the original type is kept
//! (`origin`) and still valid in the target format, an export to the same format reproduces it.
//! Unknown input never fails: it yields the generic type of each format (misc / document / GEN).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bibtex,
    Csl,
    Ris,
}

/// The 14 BibTeX entry types.
pub const BIBTEX_TYPES: [&str; 14] = [
    "article",
    "book",
    "booklet",
    "inbook",
    "incollection",
    "inproceedings",
    "conference",
    "manual",
    "mastersthesis",
    "misc",
    "phdthesis",
    "proceedings",
    "techreport",
    "unpublished",
];

/// CSL types this crosswalk can produce or consume.
pub const CSL_TYPES: [&str; 15] = [
    "article-journal",
    "article-magazine",
    "article-newspaper",
    "article",
    "book",
    "pamphlet",
    "chapter",
    "paper-conference",
    "report",
    "thesis",
    "manuscript",
    "webpage",
    "dataset",
    "software",
    "document",
];

/// RIS `TY` codes the crosswalk reads or writes (plus the other generic ones from the spec).
pub const RIS_TYPES: [&str; 26] = [
    "JOUR", "EJOUR", "INPR", "MGZN", "NEWS", "BOOK", "EBOOK", "EDBOOK", "CHAP", "ECHAP", "CPAPER",
    "CONF", "RPRT", "THES", "UNPB", "UNPD", "ELEC", "WEB", "BLOG", "DATA", "COMP", "STAND", "PAT",
    "GEN", "STD", "PAMP",
];

/// Thesis kind, kept apart from the type because only BibTeX distinguishes the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    Master,
    Doctoral,
}

fn is_valid(fmt: Format, tok: &str) -> bool {
    match fmt {
        Format::Bibtex => BIBTEX_TYPES.contains(&tok),
        Format::Csl => {
            CSL_TYPES.contains(&tok) || matches!(tok, "standard" | "patent" | "legislation")
        }
        Format::Ris => RIS_TYPES.contains(&tok),
    }
}

/// Canonical spelling of a token in its format (BibTeX lower case, RIS trimmed upper case as
/// written, CSL exact). `None` when empty or not a type this crosswalk knows.
fn canon(fmt: Format, raw: &str) -> Option<String> {
    let t = raw.trim();
    let c = match fmt {
        Format::Bibtex => t.to_lowercase(),
        Format::Csl | Format::Ris => t.to_string(),
    };
    is_valid(fmt, &c).then_some(c)
}

/// Foreign type -> (CSL type, genre). Unknown / empty -> ("document", None).
pub fn to_csl(fmt: Format, raw: &str) -> (&'static str, Option<Genre>) {
    let Some(t) = canon(fmt, raw) else {
        return ("document", None);
    };
    let t = t.as_str();
    match fmt {
        Format::Csl => (
            CSL_TYPES
                .iter()
                .copied()
                .chain(["standard", "patent", "legislation"])
                .find(|x| *x == t)
                .unwrap_or("document"),
            None,
        ),
        Format::Bibtex => match t {
            "article" => ("article-journal", None),
            "book" => ("book", None),
            "booklet" => ("pamphlet", None),
            "inbook" | "incollection" => ("chapter", None),
            "inproceedings" | "conference" => ("paper-conference", None),
            "manual" | "techreport" => ("report", None),
            "mastersthesis" => ("thesis", Some(Genre::Master)),
            "phdthesis" => ("thesis", Some(Genre::Doctoral)),
            "proceedings" => ("book", None),
            "unpublished" => ("manuscript", None),
            _ => ("document", None),
        },
        Format::Ris => (
            match t {
                "JOUR" | "EJOUR" | "INPR" => "article-journal",
                "MGZN" => "article-magazine",
                "NEWS" => "article-newspaper",
                "BOOK" | "EBOOK" | "EDBOOK" | "CONF" => "book",
                "CHAP" | "ECHAP" => "chapter",
                "CPAPER" => "paper-conference",
                "RPRT" => "report",
                "THES" => "thesis",
                "UNPB" | "UNPD" => "manuscript",
                "ELEC" | "WEB" | "BLOG" => "webpage",
                "DATA" => "dataset",
                "COMP" => "software",
                "STAND" => "standard",
                "PAT" => "patent",
                "PAMP" => "pamphlet",
                _ => "document",
            },
            None,
        ),
    }
}

fn from_csl(target: Format, csl: &str, genre: Option<Genre>) -> &'static str {
    match target {
        Format::Csl => CSL_TYPES
            .iter()
            .copied()
            .chain(["standard", "patent", "legislation"])
            .find(|x| *x == csl)
            .unwrap_or("document"),
        Format::Bibtex => match csl {
            "article-journal" | "article-magazine" | "article-newspaper" => "article",
            "book" => "book",
            "pamphlet" => "booklet",
            "chapter" => "incollection",
            "paper-conference" => "inproceedings",
            "report" => "techreport",
            "thesis" => match genre {
                Some(Genre::Master) => "mastersthesis",
                _ => "phdthesis",
            },
            "manuscript" => "unpublished",
            _ => "misc", // includes preprint "article", webpage, dataset, software, ...
        },
        Format::Ris => match csl {
            "article-journal" => "JOUR",
            "article-magazine" => "MGZN",
            "article-newspaper" => "NEWS",
            "book" => "BOOK",
            "pamphlet" => "PAMP",
            "chapter" => "CHAP",
            "paper-conference" => "CPAPER",
            "report" => "RPRT",
            "thesis" => "THES",
            "manuscript" => "UNPB",
            "webpage" => "ELEC",
            "dataset" => "DATA",
            "software" => "COMP",
            "standard" => "STAND",
            "patent" => "PAT",
            _ => "GEN",
        },
    }
}

/// Type to write in `target`. `origin` is the (format, token) the record was read with; if it is
/// in the target format, valid there, and still means the same CSL type, it wins.
pub fn emit(
    target: Format,
    csl: &str,
    genre: Option<Genre>,
    origin: Option<(Format, &str)>,
) -> String {
    if let Some((f, tok)) = origin
        && f == target
        && let Some(c) = canon(f, tok)
        && to_csl(f, &c).0 == csl
    {
        return c;
    }
    from_csl(target, csl, genre).to_string()
}

/// Thesis genre from free text ("master's thesis", "teză de doctorat", "PhD").
pub fn genre_from_text(s: &str) -> Option<Genre> {
    let l = s.to_lowercase();
    if l.contains("master") || l.contains("licen") || l.contains("disertat") {
        Some(Genre::Master)
    } else if l.contains("phd") || l.contains("ph.d") || l.contains("doctor") {
        Some(Genre::Doctoral)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Format::*;

    fn via(from: Format, raw: &str, to: Format) -> String {
        let (c, g) = to_csl(from, raw);
        emit(to, c, g, None)
    }

    #[test]
    fn phd_thesis_chain() {
        let (c, g) = to_csl(Bibtex, "phdthesis");
        assert_eq!(c, "thesis");
        assert_eq!(g, Some(Genre::Doctoral));
        assert_eq!(emit(Ris, c, g, None), "THES");
        assert_eq!(emit(Bibtex, c, Some(Genre::Master), None), "mastersthesis");
    }

    #[test]
    fn conference_chain() {
        assert_eq!(via(Bibtex, "inproceedings", Csl), "paper-conference");
        assert_eq!(via(Bibtex, "inproceedings", Ris), "CPAPER");
    }

    #[test]
    fn origin_kept_or_default() {
        let (c, g) = to_csl(Csl, "report");
        assert_eq!(emit(Bibtex, c, g, Some((Bibtex, "manual"))), "manual");
        assert_eq!(emit(Bibtex, c, g, None), "techreport");
        // origin from another format does not leak
        assert_eq!(emit(Bibtex, c, g, Some((Ris, "RPRT"))), "techreport");
        assert_eq!(
            emit(Bibtex, "book", None, Some((Bibtex, "Conference"))),
            "book"
        );
    }

    #[test]
    fn unknown_type_is_generic() {
        assert_eq!(via(Bibtex, "software", Csl), "document");
        assert_eq!(via(Bibtex, "software", Ris), "GEN");
        assert_eq!(via(Bibtex, "software", Bibtex), "misc");
        for raw in ["", "  ", "Article-Journal"] {
            assert_eq!(via(Csl, raw, Ris), "GEN");
        }
        assert_eq!(via(Ris, "XXXX", Bibtex), "misc");
    }

    #[test]
    fn ris_chains() {
        assert_eq!(via(Ris, "ELEC", Csl), "webpage");
        assert_eq!(via(Ris, "ELEC", Bibtex), "misc");
        let (c, g) = to_csl(Ris, " UNPD ");
        assert_eq!(c, "manuscript");
        assert_eq!(emit(Ris, c, g, Some((Ris, "UNPD"))), "UNPD");
        assert_eq!(emit(Ris, c, g, None), "UNPB");
        assert_eq!(via(Csl, "article-magazine", Bibtex), "article");
        assert_eq!(via(Csl, "article", Bibtex), "misc");
        assert_eq!(via(Ris, "CONF", Csl), "book");
    }

    #[test]
    fn bibtex_case_insensitive_and_conference_origin() {
        let (c, g) = to_csl(Bibtex, "Conference");
        assert_eq!(c, "paper-conference");
        assert_eq!(
            emit(Bibtex, c, g, Some((Bibtex, "Conference"))),
            "conference"
        );
    }

    #[test]
    fn outputs_are_in_target_lists() {
        let mut inputs: Vec<(Format, &str)> = vec![];
        inputs.extend(BIBTEX_TYPES.iter().map(|t| (Bibtex, *t)));
        inputs.extend(CSL_TYPES.iter().map(|t| (Csl, *t)));
        inputs.extend(RIS_TYPES.iter().map(|t| (Ris, *t)));
        inputs.push((Bibtex, "nonsense"));
        for (f, t) in inputs {
            for g in [None, Some(Genre::Master), Some(Genre::Doctoral)] {
                let (c, _) = to_csl(f, t);
                for tf in [Bibtex, Csl, Ris] {
                    let o = emit(tf, c, g, Some((f, t)));
                    assert!(is_valid(tf, &o), "{f:?} {t} -> {tf:?} {o}");
                }
            }
        }
    }

    #[test]
    fn genre_text() {
        assert_eq!(genre_from_text("Master's thesis"), Some(Genre::Master));
        assert_eq!(genre_from_text("PhD thesis"), Some(Genre::Doctoral));
        assert_eq!(genre_from_text("teză de doctorat"), Some(Genre::Doctoral));
        assert_eq!(genre_from_text(""), None);
    }
}
