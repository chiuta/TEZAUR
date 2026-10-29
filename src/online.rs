//! Opt-in online lookups: literature search and reference checking against OpenAlex and Crossref.
//! This is the only module that sends anything off the computer, and only when the user runs `lit` or `refs --online`.
//! Base URLs can be redirected with TEZAUR_OPENALEX / TEZAUR_CROSSREF (used by the tests, and by institutional mirrors).
//! Set TEZAUR_MAILTO to join the services' "polite pool".

use crate::bib::Entry;
use crate::index::coverage;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::time::Duration;

fn openalex() -> String {
    std::env::var("TEZAUR_OPENALEX").unwrap_or_else(|_| "https://api.openalex.org".into()).trim_end_matches('/').to_string()
}
fn crossref() -> String {
    std::env::var("TEZAUR_CROSSREF").unwrap_or_else(|_| "https://api.crossref.org".into()).trim_end_matches('/').to_string()
}

fn get(url: &str, query: &[(&str, &str)]) -> Result<Option<Value>> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(8)).timeout_read(Duration::from_secs(25)).build();
    let mut r = agent.get(url).set("User-Agent", "tezaur (https://trom.tf; local thesis assistant)");
    for (k, v) in query {
        r = r.query(k, v);
    }
    if let Ok(m) = std::env::var("TEZAUR_MAILTO") {
        r = r.query("mailto", &m);
    }
    match r.call() {
        Ok(resp) => Ok(Some(resp.into_json()?)),
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(ureq::Error::Status(c, _)) => Err(anyhow!("{url}: HTTP {c}")),
        Err(e) => Err(anyhow!("{url}: {e}")),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Work {
    pub title: String,
    pub authors: Vec<String>,
    pub year: String,
    pub venue: String,
    pub doi: String,
    pub cited_by: u64,
    pub retracted: bool,
}

fn strip_doi(d: &str) -> String {
    d.trim().trim_start_matches("https://doi.org/").trim_start_matches("http://doi.org/").trim_start_matches("doi:").to_string()
}

impl Work {
    fn from_openalex(w: &Value) -> Work {
        Work {
            title: w["title"].as_str().or(w["display_name"].as_str()).unwrap_or("").to_string(),
            authors: w["authorships"].as_array().map(|a| a.iter().filter_map(|x| x["author"]["display_name"].as_str().map(String::from)).collect()).unwrap_or_default(),
            year: w["publication_year"].as_u64().map(|y| y.to_string()).unwrap_or_default(),
            venue: w["primary_location"]["source"]["display_name"].as_str().unwrap_or("").to_string(),
            doi: w["doi"].as_str().map(strip_doi).unwrap_or_default(),
            cited_by: w["cited_by_count"].as_u64().unwrap_or(0),
            retracted: w["is_retracted"].as_bool().unwrap_or(false),
        }
    }

    fn from_crossref(w: &Value) -> Work {
        let year = w["issued"]["date-parts"][0][0].as_u64().map(|y| y.to_string()).unwrap_or_default();
        let retracted = w["update-to"].as_array().map(|a| a.iter().any(|u| u["type"].as_str() == Some("retraction"))).unwrap_or(false)
            || w["subtype"].as_str() == Some("retraction");
        Work {
            title: w["title"][0].as_str().unwrap_or("").to_string(),
            authors: w["author"].as_array().map(|a| a.iter().map(|x| format!("{} {}", x["given"].as_str().unwrap_or(""), x["family"].as_str().unwrap_or("")).trim().to_string()).collect()).unwrap_or_default(),
            year,
            venue: w["container-title"][0].as_str().unwrap_or("").to_string(),
            doi: w["DOI"].as_str().map(strip_doi).unwrap_or_default(),
            cited_by: w["is-referenced-by-count"].as_u64().unwrap_or(0),
            retracted,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({"title": self.title, "authors": self.authors, "year": self.year, "venue": self.venue, "doi": self.doi, "cited_by": self.cited_by, "retracted": self.retracted})
    }

    pub fn line(&self) -> String {
        let a = match self.authors.len() {
            0 => String::new(),
            1 => self.authors[0].clone(),
            _ => format!("{} et al.", self.authors[0]),
        };
        format!("{a} ({}). {}. {}{}{}", self.year, self.title, self.venue, if self.doi.is_empty() { String::new() } else { format!(" https://doi.org/{}", self.doi) }, if self.retracted { "  ⚠ RETRACTED" } else { "" })
    }

    pub fn key(&self) -> String {
        let last: String = self.authors.first().and_then(|a| a.split_whitespace().last()).unwrap_or("anon").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        let word: String = self.title.split_whitespace().find(|w| w.len() > 3).unwrap_or("work").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        format!("{}{}{}", if last.is_empty() { "anon".into() } else { last }, self.year, word)
    }

    pub fn to_bibtex(&self) -> String {
        Entry { key: self.key(), author: self.authors.join(" and "), year: self.year.clone(), title: self.title.clone(), container: self.venue.clone(), doi: self.doi.clone(), file: String::new() }.to_bibtex()
    }
}

/// Search OpenAlex; fall back to Crossref when OpenAlex is unreachable or empty.
pub fn search(query: &str, n: usize) -> Result<Vec<Work>> {
    let n = n.clamp(1, 50).to_string();
    let first = get(&format!("{}/works", openalex()), &[("search", query), ("per-page", &n)]);
    if let Ok(Some(v)) = &first {
        let w: Vec<Work> = v["results"].as_array().map(|a| a.iter().map(Work::from_openalex).collect()).unwrap_or_default();
        if !w.is_empty() {
            return Ok(w);
        }
    }
    match get(&format!("{}/works", crossref()), &[("query", query), ("rows", &n)]) {
        Ok(Some(v)) => Ok(v["message"]["items"].as_array().map(|a| a.iter().map(Work::from_crossref).collect()).unwrap_or_default()),
        Ok(None) => Ok(vec![]),
        Err(e2) => Err(match first {
            Err(e1) => anyhow!("OpenAlex: {e1}; Crossref: {e2}"),
            _ => e2,
        }),
    }
}

fn lookup_doi(doi: &str) -> Result<Option<Work>> {
    let d = strip_doi(doi);
    if let Ok(Some(v)) = get(&format!("{}/works/https://doi.org/{}", openalex(), d), &[]) {
        return Ok(Some(Work::from_openalex(&v)));
    }
    Ok(get(&format!("{}/works/{}", crossref(), d), &[])?.map(|v| Work::from_crossref(&v["message"])))
}

/// Existence, metadata and retraction status for each reference. Statuses: ok, retracted, metadata-mismatch, not-found, unchecked, error.
pub fn check_entries(entries: &[&Entry]) -> Value {
    let mut out = Vec::new();
    for e in entries {
        let r = (|| -> Result<(String, String)> {
            if e.doi.is_empty() && e.title.is_empty() {
                return Ok(("unchecked".into(), "no DOI and no title in the .bib entry".into()));
            }
            let found = if !e.doi.is_empty() {
                lookup_doi(&e.doi)?
            } else {
                search(&e.title, 3)?.into_iter().find(|w| coverage(&e.title, &w.title) >= 0.8 && coverage(&w.title, &e.title) >= 0.8)
            };
            let Some(w) = found else {
                return Ok(("not-found".into(), if e.doi.is_empty() { "no work with this title in OpenAlex/Crossref".into() } else { format!("DOI {} does not resolve in OpenAlex or Crossref", e.doi) }));
            };
            if w.retracted {
                return Ok(("retracted".into(), format!("“{}” is marked as retracted", w.title)));
            }
            let mut diffs = Vec::new();
            if !e.title.is_empty() && !w.title.is_empty() && coverage(&e.title, &w.title) < 0.6 {
                diffs.push(format!("title in the .bib differs from “{}”", w.title));
            }
            if !e.year.is_empty() && !w.year.is_empty() && e.year != w.year {
                diffs.push(format!("year {} vs {}", e.year, w.year));
            }
            Ok(if diffs.is_empty() { ("ok".into(), format!("{} ({})", w.title, w.year)) } else { ("metadata-mismatch".into(), diffs.join("; ")) })
        })();
        let (status, note) = r.unwrap_or_else(|er| ("error".into(), er.to_string()));
        out.push(json!({"key": e.key, "status": status, "note": note}));
    }
    json!(out)
}

pub fn check_markdown(v: &Value) -> String {
    let mut s = String::from("## Online check (OpenAlex / Crossref)\n\n");
    let items = v.as_array().cloned().unwrap_or_default();
    let bad: Vec<&Value> = items.iter().filter(|x| x["status"] != "ok").collect();
    s.push_str(&format!("{} references checked, {} need attention. A DOI missing from these services is not proof the work is fake (new, local or non-indexed works exist).\n\n", items.len(), bad.len()));
    for x in bad {
        s.push_str(&format!("- **{}** `{}` — {}\n", x["status"].as_str().unwrap_or(""), x["key"].as_str().unwrap_or(""), x["note"].as_str().unwrap_or("")));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openalex_and_crossref() {
        let a = Work::from_openalex(&json!({"title":"Deep Learning","publication_year":2015,"doi":"https://doi.org/10.1/x","authorships":[{"author":{"display_name":"Yann LeCun"}}],"is_retracted":true,"cited_by_count":5,"primary_location":{"source":{"display_name":"Nature"}}}));
        assert_eq!((a.doi.as_str(), a.year.as_str(), a.retracted), ("10.1/x", "2015", true));
        assert_eq!(a.key(), "lecun2015deep");
        assert!(a.to_bibtex().contains("10.1/x"));
        let c = Work::from_crossref(&json!({"title":["T"],"DOI":"10.2/y","issued":{"date-parts":[[2001]]},"author":[{"given":"A","family":"B"}],"update-to":[{"type":"retraction"}]}));
        assert!(c.retracted);
        assert_eq!(c.year, "2001");
    }
}
