//! `tezaur serve`: a small local HTTP server. It hands out the web app (`tezaur.html`), a JSON API over the
//! index (so the page gets native PDF/EPUB reading, OCR and embeddings), index download/upload for syncing between
//! machines, named workspaces, and a same-origin proxy to the model server (no CORS setup anywhere).

use std::io::Read as _;
use crate::bib::BibMap;
use crate::index::{coverage, Searcher, Store};
use crate::{embed_pending, embed_query, ingest, mesh, prompt, Ctx};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const HTML: &str = include_str!("../tezaur.html");
/// Bodies are read into memory, so cap them: a text/JSON request at 8 MB, an upload (file, index, bundle) at 512 MB.
const MAX_TEXT: u64 = 8 << 20;
const MAX_UPLOAD: u64 = 512 << 20;
const OPENAPI: &str = include_str!("../variants/editors/rest/openapi.json");

pub struct Opts {
    pub bind: String,
    pub port: u16,
    pub token: Option<String>,
    pub workspaces: Option<PathBuf>,
}

fn hdr(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|x| u8::from_str_radix(x, 16).ok());
                match h {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn split_url(url: &str) -> (String, Vec<(String, String)>) {
    let (path, q) = url.split_once('?').unwrap_or((url, ""));
    let params = q
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (pct_decode(k), pct_decode(v))
        })
        .collect();
    (pct_decode(path), params)
}

fn get<'a>(p: &'a [(String, String)], k: &str) -> Option<&'a str> {
    p.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}

fn safe_name(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).take(40).collect()
}

fn json_resp(code: u16, v: Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(v.to_string().into_bytes()).with_status_code(StatusCode(code)).with_header(hdr("Content-Type", "application/json; charset=utf-8"))
}

/// (index file, library base folder) for a workspace; the default workspace is the CLI's own index.
fn workspace(ctx: &Ctx, o: &Opts, ws: Option<&str>) -> Result<(PathBuf, PathBuf)> {
    match (ws.map(safe_name).filter(|w| !w.is_empty() && w != "default"), &o.workspaces) {
        (Some(w), Some(dir)) => {
            std::fs::create_dir_all(dir.join(&w))?;
            Ok((dir.join(format!("{w}.idx")), dir.join(&w)))
        }
        (Some(_), None) => bail!("this server was started without --workspaces"),
        (None, _) => Ok((ctx.index.clone(), ctx.base.clone())),
    }
}

fn list_workspaces(ctx: &Ctx, o: &Opts) -> Vec<String> {
    let mut v = vec!["default".to_string()];
    if let Some(d) = &o.workspaces {
        if let Ok(rd) = std::fs::read_dir(d) {
            let mut w: Vec<String> = rd
                .flatten()
                .filter_map(|e| e.path().file_stem().map(|s| (e.path(), s.to_string_lossy().to_string())))
                .filter(|(p, _)| p.extension().is_some_and(|x| x == "idx"))
                .map(|(_, s)| s)
                .collect();
            w.sort();
            v.extend(w);
        }
    }
    let _ = ctx;
    v
}

/// Comparison whose time does not depend on where the strings first differ.
fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn authorised(req: &Request, o: &Opts, params: &[(String, String)]) -> bool {
    let Some(t) = &o.token else { return true };
    if get(params, "token").map(|x| ct_eq(x, t)).unwrap_or(false) {
        return true;
    }
    let want = format!("Bearer {t}");
    req.headers().iter().any(|h| h.field.equiv("Authorization") && ct_eq(h.value.as_str(), &want))
}

/// Without a token the server trusts whoever can reach it, so a web page must not be able to reach it through DNS rebinding:
/// require a loopback or literal-IP Host header (a hostname an attacker controls would resolve to 127.0.0.1 but is not one of these).
fn host_ok(req: &Request, o: &Opts) -> bool {
    if o.token.is_some() {
        return true;
    }
    let Some(h) = req.headers().iter().find(|h| h.field.equiv("Host")).map(|h| h.value.as_str().to_lowercase()) else { return true };
    let name = if let Some(r) = h.strip_prefix('[') { r.split(']').next().unwrap_or("").to_string() } else { h.split(':').next().unwrap_or("").to_string() };
    name == "localhost" || name.parse::<std::net::IpAddr>().is_ok() || name == o.bind.to_lowercase()
}

fn bib_for(ctx: &Ctx, base: &Path) -> BibMap {
    BibMap::load(&[base.to_path_buf()], &ctx.bib_files, if ctx.data.is_some() { 3 } else { 1 })
}

fn api(ctx: &Ctx, o: &Opts, method: &Method, path: &str, params: &[(String, String)], req: &mut Request) -> Result<Response<std::io::Cursor<Vec<u8>>>> {
    let (index, base) = workspace(ctx, o, get(params, "ws"))?;
    match (method, path) {
        (Method::Get, "/api/health") => {
            let store = Store::load(&index)?;
            Ok(json_resp(200, json!({
                "ok": true, "server": true, "version": env!("CARGO_PKG_VERSION"),
                "workspaces": list_workspaces(ctx, o), "docs": store.docs.len(), "chunks": store.chunks.len(),
                "embeddings": store.embed_model, "model": ctx.llm.model, "api": format!("{:?}", ctx.llm.api).to_lowercase(),
                "ocr": std::process::Command::new("tesseract").arg("--version").output().is_ok(),
            })))
        }
        (Method::Get, "/api/sources") => {
            let store = Store::load(&index)?;
            let bib = bib_for(ctx, &base);
            let docs: Vec<Value> = store.docs.iter().enumerate().map(|(i, d)| {
                json!({"path": d.path, "title": d.title, "pages": d.pages,
                       "chunks": store.chunks.iter().filter(|c| c.doc == i).count(),
                       "cite": bib.find(&d.path).map(|e| e.cite())})
            }).collect();
            let keys: Vec<Value> = bib.entries.iter().map(|e| json!({"key": e.key, "cite": e.cite(), "title": e.title, "year": e.year})).collect();
            Ok(json_resp(200, json!({"docs": docs, "bib": bib.entries.len(), "keys": keys})))
        }
        (Method::Get, "/api/bib") => {
            let bib = bib_for(ctx, &base);
            Ok(match get(params, "fmt").unwrap_or("csl") {
                "ris" => Response::from_data(bib.entries.iter().map(|e| e.to_ris()).collect::<Vec<_>>().join("\n").into_bytes()).with_header(hdr("Content-Type", "application/x-research-info-systems")),
                "bibtex" => Response::from_data(bib.entries.iter().map(|e| e.to_bibtex()).collect::<Vec<_>>().join("\n").into_bytes()).with_header(hdr("Content-Type", "application/x-bibtex")),
                _ => Response::from_data(serde_json::to_vec(&bib.entries.iter().map(|e| e.to_csl()).collect::<Vec<_>>())?).with_header(hdr("Content-Type", "application/json")),
            })
        }
        (Method::Get, "/api/search") | (Method::Post, "/api/search") => {
            let q = get(params, "q").unwrap_or("").trim().to_string();
            if q.is_empty() {
                bail!("missing q");
            }
            let k = get(params, "k").and_then(|v| v.parse().ok()).unwrap_or(8usize).clamp(1, 40);
            let store = Store::load(&index)?;
            let bib = bib_for(ctx, &base);
            let allow: Option<HashSet<usize>> = get(params, "paths").filter(|p| !p.is_empty()).map(|p| {
                let want: HashSet<&str> = p.split('\n').collect();
                store.docs.iter().enumerate().filter(|(_, d)| want.contains(d.path.as_str())).map(|(i, _)| i).collect()
            });
            let emb = embed_query(ctx, &store, &q);
            let hits = Searcher::new(&store).search(&q, emb.as_deref(), k, 3, allow.as_ref());
            let out: Vec<Value> = hits.iter().enumerate().map(|(i, h)| {
                let c = &store.chunks[h.chunk];
                let d = &store.docs[c.doc];
                json!({"n": i + 1, "label": prompt::label(&store, &bib, h), "text": c.text, "page": c.page, "path": d.path,
                       "title": d.title, "cov": coverage(&q, &c.text), "score": h.score})
            }).collect();
            let mut resp = json!({"hits": out});
            if get(params, "mesh") == Some("1") {
                let c = mesh::load_cfg(&base);
                let (remote, errors) = mesh::federated_search(&c.peers, &q, k);
                resp["peer_hits"] = json!(remote);
                resp["peer_errors"] = json!(errors);
            }
            Ok(json_resp(200, resp))
        }
        (Method::Get, "/api/ask") | (Method::Post, "/api/ask") => {
            let q = get(params, "q").unwrap_or("").trim().to_string();
            if q.is_empty() {
                bail!("missing q");
            }
            let k = get(params, "k").and_then(|v| v.parse().ok()).unwrap_or(8usize).clamp(1, 30);
            let store = Store::load(&index)?;
            let bib = bib_for(ctx, &base);
            Ok(json_resp(200, crate::ask_json(ctx, &store, &bib, get(params, "mode").unwrap_or("ask"), &q, k, get(params, "lang"))?))
        }
        (Method::Post, "/api/audit") | (Method::Post, "/api/coverage") | (Method::Post, "/api/novelty") | (Method::Post, "/api/refs") => {
            let text = get(params, "text").unwrap_or("").to_string();
            if text.trim().is_empty() {
                bail!("missing text");
            }
            let store = Store::load(&index)?;
            let bib = bib_for(ctx, &base);
            Ok(json_resp(200, match path {
                "/api/audit" => {
                    let mut f = crate::audit::audit(&store, &bib, &text);
                    let semantic = matches!(get(params, "semantic"), Some("1") | Some("true"));
                    let judged = if semantic { Some(crate::semantic_audit(ctx, &mut f)) } else { None };
                    json!({"findings": f.iter().map(|x| x.to_json()).collect::<Vec<_>>(), "problems": f.iter().filter(|x| x.is_problem()).count(), "markdown": crate::audit::audit_markdown(&f),
                           "semantic": judged.map(|(c, ch)| json!({"judged": c, "changed": ch, "model": ctx.llm.model}))})
                }
                "/api/refs" => {
                    let mut v = crate::audit::refs_report(&store, &bib, &text);
                    v["markdown"] = json!(crate::audit::refs_markdown(&v));
                    v
                }
                "/api/coverage" => {
                    let mut v = crate::audit::coverage_matrix(&store, &bib, &crate::audit::outline_headings(&text), &|h| crate::embed_query(ctx, &store, h));
                    v["markdown"] = json!(crate::audit::coverage_markdown(&v));
                    v
                }
                _ => {
                    let mut v = crate::audit::novelty(&store, &bib, &text);
                    v["markdown"] = json!(crate::audit::novelty_markdown(&v));
                    v
                }
            }))
        }
        (Method::Post, "/api/receipt") => {
            let r: Value = serde_json::from_str(get(params, "receipt").unwrap_or("{}"))?;
            let store = Store::load(&index)?;
            let (ok, lines) = crate::audit::verify_receipt(Some(&store), &r, get(params, "answer"));
            Ok(json_resp(200, json!({"ok": ok, "checks": lines})))
        }
        (Method::Get, "/api/mesh/info") => {
            let c = mesh::load_cfg(&base);
            let store = Store::load(&index)?;
            Ok(json_resp(200, json!({"id": c.id, "name": c.name, "shared": mesh::shared_docs(&c, &store).len(),
                                     "peers": c.peers.iter().map(|p| &p.name).collect::<Vec<_>>(), "relay": c.relay})))
        }
        (Method::Get, "/api/mesh/manifest") => {
            let c = mesh::load_cfg(&base);
            Ok(json_resp(200, mesh::manifest(&c, &Store::load(&index)?)))
        }
        (Method::Get, "/api/mesh/doc") => {
            let c = mesh::load_cfg(&base);
            let store = Store::load(&index)?;
            let want = get(params, "path").unwrap_or("");
            match store.doc_by_path(want).filter(|&i| mesh::is_shared(&c, &store.docs[i])) {
                Some(i) => Ok(json_resp(200, mesh::doc_payload(&store, i))),
                None => bail!("no such shared document"),
            }
        }
        (Method::Get, "/api/mesh/search") => {
            let c = mesh::load_cfg(&base);
            let q = get(params, "q").unwrap_or("").trim().to_string();
            if q.is_empty() {
                bail!("missing q");
            }
            let k = get(params, "k").and_then(|v| v.parse().ok()).unwrap_or(8usize).clamp(1, 40);
            let store = Store::load(&index)?;
            Ok(json_resp(200, json!({"node": c.name, "hits": mesh::shared_search(&c, &store, &bib_for(ctx, &base), &q, k)})))
        }
        (Method::Get, "/api/mesh/bundle") => {
            let c = mesh::load_cfg(&base);
            let store = Store::load(&index)?;
            let bytes = mesh::bundle_export(&c, &store, &bib_for(ctx, &base), false)?;
            Ok(Response::from_data(bytes).with_header(hdr("Content-Type", "application/zip")).with_header(hdr("Content-Disposition", "attachment; filename=\"share.tzb\"")))
        }
        (Method::Get, "/api/mesh/peers") => {
            let c = mesh::load_cfg(&base);
            Ok(json_resp(200, json!({"peers": c.peers.iter().map(|p| json!({"name": p.name, "url": p.url, "proxy": p.proxy})).collect::<Vec<_>>()})))
        }
        (Method::Post, "/api/mesh/peers") => {
            let mut b = String::new();
            std::io::Read::take(req.as_reader(), MAX_TEXT).read_to_string(&mut b)?;
            let v: Value = serde_json::from_str(&b)?;
            let (name, url) = (v["name"].as_str().unwrap_or("").trim(), v["url"].as_str().unwrap_or("").trim());
            if name.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
                bail!("need name and an http(s) url");
            }
            let mut c = mesh::load_cfg(&base);
            c.peers.retain(|p| p.name != name);
            c.peers.push(mesh::Peer { name: name.into(), url: url.into(), token: v["token"].as_str().map(String::from), proxy: v["proxy"].as_str().map(String::from) });
            mesh::save_cfg(&base, &c)?;
            Ok(json_resp(200, json!({"ok": true})))
        }
        (Method::Post, "/api/mesh/pull") => {
            let c = mesh::load_cfg(&base);
            let want = get(params, "peer");
            let mut store = Store::load(&index)?;
            let (mut added, mut held, mut errors) = (0, 0, Vec::new());
            for p in c.peers.iter().filter(|p| want.map(|w| w == p.name).unwrap_or(true)) {
                match mesh::pull(&mut store, p) {
                    Ok((a, h)) => { added += a; held += h; }
                    Err(e) => errors.push(format!("{e:#}")),
                }
            }
            if added > 0 {
                store.save(&index)?;
                embed_pending(ctx, &mut store, &index);
                store.save(&index)?;
            }
            Ok(json_resp(200, json!({"added": added, "already_held": held, "errors": errors})))
        }
        (Method::Post, "/api/mesh/import") => {
            let mut b = Vec::new();
            std::io::Read::take(req.as_reader(), MAX_UPLOAD).read_to_end(&mut b)?;
            let mut store = Store::load(&index)?;
            let (added, skipped, origin) = mesh::bundle_import(&mut store, &base, &b)?;
            if added > 0 {
                store.save(&index)?;
                embed_pending(ctx, &mut store, &index);
                store.save(&index)?;
            }
            Ok(json_resp(200, json!({"added": added, "already_held": skipped, "from": origin})))
        }
        (Method::Get, "/api/read") => {
            let want = get(params, "path").unwrap_or("");
            let store = Store::load(&index)?;
            let bib = bib_for(ctx, &base);
            let Some(di) = store.docs.iter().position(|d| d.path == want) else { bail!("no such document") };
            let d = &store.docs[di];
            let name = bib.find(&d.path).map(|e| e.cite()).unwrap_or_else(|| d.title.clone());
            let chunks: Vec<Value> = store.chunks.iter().filter(|c| c.doc == di).map(|c| json!({"page": c.page, "text": c.text})).collect();
            Ok(json_resp(200, json!({"title": d.title, "label": name, "chunks": chunks})))
        }
        (Method::Post, "/api/ingest") => {
            let name = Path::new(get(params, "name").unwrap_or("upload.txt")).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.is_empty() || !ingest::is_supported(Path::new(&name)) {
                bail!("unsupported or missing file name");
            }
            let dir = base.join("Library");
            std::fs::create_dir_all(&dir)?;
            let dest = dir.join(&name);
            let mut body = Vec::new();
            std::io::Read::take(req.as_reader(), MAX_UPLOAD).read_to_end(&mut body)?;
            std::fs::write(&dest, &body)?;
            let mut store = Store::load(&index)?;
            let force = get(params, "force") == Some("1");
            let (status, chunks) = match ingest::ingest_file(&mut store, &dest, &base, force) {
                Ok(ingest::Outcome::Added { chunks }) => ("added", chunks),
                Ok(ingest::Outcome::Unchanged) => ("unchanged", 0),
                Ok(ingest::Outcome::NoText) => ("no-text", 0),
                Err(e) => bail!("{e}"),
            };
            if status == "added" {
                store.save(&index)?;
                embed_pending(ctx, &mut store, &index);
                store.save(&index)?;
            }
            Ok(json_resp(200, json!({"status": status, "chunks": chunks})))
        }
        (Method::Delete, "/api/doc") => {
            let want = get(params, "path").unwrap_or("");
            let mut store = Store::load(&index)?;
            let Some(di) = store.doc_by_path(want) else { bail!("no such document") };
            store.remove_doc(di);
            store.save(&index)?;
            Ok(json_resp(200, json!({"removed": want})))
        }
        _ => Ok(json_resp(404, json!({"error": "not found"}))),
    }
}

/// Streams the request to the configured model server and the answer back (SSE / NDJSON pass through).
fn proxy(host: String, key: Option<String>, mut req: Request, tail: String) {
    let mut body = Vec::new();
    let _ = std::io::Read::take(req.as_reader(), MAX_UPLOAD).read_to_end(&mut body);
    let method = req.method().as_str().to_string();
    let url = format!("{}{}", host.trim_end_matches('/'), tail);
    let mut r = ureq::request(&method, &url);
    for h in req.headers() {
        if h.field.equiv("Content-Type") || h.field.equiv("Accept") {
            r = r.set(h.field.as_str().as_str(), h.value.as_str());
        }
    }
    if let Some(k) = key {
        r = r.set("Authorization", &format!("Bearer {k}"));
    }
    let res = if body.is_empty() && method == "GET" { r.call() } else { r.send_bytes(&body) };
    let resp = match res {
        Ok(x) => x,
        Err(ureq::Error::Status(_, x)) => x,
        Err(e) => {
            let _ = req.respond(json_resp(502, json!({"error": format!("cannot reach the model server at {host}: {e}")})));
            return;
        }
    };
    let status = resp.status();
    let ct = resp.header("Content-Type").unwrap_or("application/octet-stream").to_string();
    let reader = resp.into_reader();
    let _ = req.respond(Response::new(StatusCode(status), vec![hdr("Content-Type", &ct), hdr("Cache-Control", "no-cache")], reader, None, None));
}

pub fn serve(ctx: &Ctx, o: &Opts) -> Result<()> {
    let local = o.bind == "127.0.0.1" || o.bind == "localhost" || o.bind == "::1";
    if !local && o.token.is_none() {
        bail!("binding to {} exposes your library to the network — add --token SECRET", o.bind);
    }
    let server = Server::http(format!("{}:{}", o.bind, o.port)).map_err(|e| anyhow::anyhow!("cannot listen on {}:{} — {e}", o.bind, o.port))?;
    let host = ctx.llm.host().to_string();
    let key = ctx.llm.key().map(String::from);
    eprintln!("TEZAUR on http://{}:{}/{}  (index {}; model server {})", o.bind, o.port, o.token.as_ref().map(|t| format!("?token={t}")).unwrap_or_default(), ctx.index.display(), host);
    if !local || std::env::var("TEZAUR_MESH_BEACON").is_ok() {
        mesh::start_beacon(&mesh::load_cfg(&ctx.base), o.port, mesh::DISCOVERY_PORT);
    }
    for mut req in server.incoming_requests() {
        let (path, params) = split_url(req.url());
        let method = req.method().clone();
        if method == Method::Get && (path == "/" || path == "/index.html") {
            let _ = req.respond(Response::from_string(HTML).with_header(hdr("Content-Type", "text/html; charset=utf-8")));
            continue;
        }
        if method == Method::Options && path.starts_with("/api/mesh/") {
            let _ = req.respond(Response::empty(204).with_header(hdr("Access-Control-Allow-Origin", "*")).with_header(hdr("Access-Control-Allow-Headers", "Authorization, Content-Type")).with_header(hdr("Access-Control-Allow-Methods", "GET, POST, OPTIONS")));
            continue;
        }
        if req.body_length().map(|n| n as u64 > MAX_UPLOAD).unwrap_or(false) {
            let _ = req.respond(json_resp(413, json!({"error": "request body too large"})));
            continue;
        }
        if !host_ok(&req, o) {
            let _ = req.respond(json_resp(403, json!({"error": "unexpected Host header (DNS-rebinding protection); use 127.0.0.1 or localhost, or start the server with --token"})));
            continue;
        }
        if !authorised(&req, o, &params) {
            let _ = req.respond(json_resp(401, json!({"error": "token required"})));
            continue;
        }
        if method == Method::Get && path == "/openapi.json" {
            let _ = req.respond(Response::from_string(OPENAPI).with_header(hdr("Content-Type", "application/json")));
            continue;
        }
        if method == Method::Post && path == "/mcp" {
            // MCP over HTTP (JSON responses, no streaming): remote-connector style clients post one JSON-RPC message per request.
            let mut b = String::new();
            let _ = std::io::Read::take(req.as_reader(), MAX_TEXT).read_to_string(&mut b);
            let resp = match crate::mcp_handle(ctx, &b) {
                Some(r) => Response::from_string(r).with_header(hdr("Content-Type", "application/json")).with_status_code(200),
                None => Response::from_string("").with_status_code(202),
            };
            let _ = req.respond(resp);
            continue;
        }
        if let Some(tail) = path.strip_prefix("/llm") {
            let tail = format!("{}{}", tail, req.url().split_once('?').map(|(_, q)| format!("?{q}")).unwrap_or_default());
            let (h, k) = (host.clone(), key.clone());
            std::thread::spawn(move || proxy(h, k, req, tail));
            continue;
        }
        if path == "/api/index" && (method == Method::Get || method == Method::Put) {
            let r = workspace(ctx, o, get(&params, "ws")).and_then(|(index, _)| {
                if method == Method::Get {
                    Ok(Response::from_data(std::fs::read(&index)?).with_header(hdr("Content-Type", "application/octet-stream")))
                } else {
                    let mut b = Vec::new();
                    std::io::Read::take(req.as_reader(), MAX_UPLOAD).read_to_end(&mut b)?;
                    // Refuse anything that is not a loadable index before overwriting the real one.
                    let tmp = index.with_extension("incoming");
                    std::fs::write(&tmp, &b)?;
                    if let Err(e) = Store::load(&tmp) {
                        let _ = std::fs::remove_file(&tmp);
                        bail!("not a valid TEZAUR index: {e}");
                    }
                    std::fs::rename(&tmp, &index)?;
                    Ok(Response::from_data(b"{\"ok\":true}".to_vec()).with_header(hdr("Content-Type", "application/json")))
                }
            });
            match r {
                Ok(resp) => {
                    let _ = req.respond(resp);
                }
                Err(e) => {
                    let _ = req.respond(json_resp(400, json!({"error": format!("{e:#}")})));
                }
            }
            continue;
        }
        // JSON bodies are accepted for POST /api/ask and /api/search (no URL-encoding needed in clients).
        let mut params = params;
        if method == Method::Post && matches!(path.as_str(), "/api/ask" | "/api/search" | "/api/audit" | "/api/coverage" | "/api/novelty" | "/api/refs" | "/api/receipt") {
            let mut b = String::new();
            let _ = std::io::Read::take(req.as_reader(), MAX_TEXT).read_to_string(&mut b);
            if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&b) {
                for (k, v) in m {
                    let val = match v { Value::String(x) => x, Value::Number(n) => n.to_string(), Value::Array(a) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("\n"), o @ Value::Object(_) => o.to_string(), _ => continue };
                    params.push((k, val));
                }
            }
        }
        let mut resp = match api(ctx, o, &method, &path, &params, &mut req) {
            Ok(r) => r,
            Err(e) => json_resp(400, json!({"error": format!("{e:#}")})),
        };
        if path.starts_with("/api/mesh/") {
            resp = resp.with_header(hdr("Access-Control-Allow-Origin", "*"));
        }
        let _ = req.respond(resp);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_urls_and_params() {
        let (p, q) = split_url("/api/search?q=burden+of%20proof&k=3&paths=Library%2Fa.txt%0ALibrary%2Fb.pdf&x=%C8%98");
        assert_eq!(p, "/api/search");
        assert_eq!(get(&q, "q"), Some("burden of proof"));
        assert_eq!(get(&q, "paths"), Some("Library/a.txt\nLibrary/b.pdf"));
        assert_eq!(get(&q, "x"), Some("Ș"));
        assert_eq!(pct_decode("100%"), "100%"); // dangling % must not panic
        assert_eq!(pct_decode("%zz"), "%zz");
    }

    #[test]
    fn workspace_names_are_sanitised() {
        assert_eq!(safe_name("../../etc/passwd"), "etcpasswd");
        assert_eq!(safe_name("cap-1_a"), "cap-1_a");
        assert_eq!(safe_name("???"), "");
    }
}
