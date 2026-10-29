//! Client for a local model server. Two wire protocols:
//!   * Ollama        (`/api/chat`, `/api/embed`, NDJSON streaming)
//!   * OpenAI-style  (`/v1/chat/completions`, `/v1/embeddings`, SSE streaming) — used by
//!     Jan, LM Studio, llama.cpp `llama-server`, vLLM, etc.
//! Everything stays on localhost by default; nothing is sent to any cloud service.

use crate::index::normalize;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Api {
    /// Ollama native API (default port 11434).
    Ollama,
    /// Any OpenAI-compatible server; give the base URL with --host.
    Openai,
    /// Jan: OpenAI-compatible server at http://127.0.0.1:1337/v1 unless --host says otherwise.
    Jan,
}

impl Api {
    pub fn default_host(self) -> &'static str {
        match self {
            Api::Ollama => "http://localhost:11434",
            Api::Openai => "http://127.0.0.1:8080/v1",
            Api::Jan => "http://127.0.0.1:1337/v1",
        }
    }
    fn is_openai(self) -> bool {
        !matches!(self, Api::Ollama)
    }
}

pub struct Llm {
    host: String,
    pub api: Api,
    pub model: String,
    pub num_ctx: u32,
    key: Option<String>,
    agent: ureq::Agent,
}

fn normalize_host(h: &str, api: Api) -> String {
    let h = h.trim().trim_end_matches('/');
    let mut h = if h.contains("://") { h.to_string() } else { format!("http://{h}") };
    // OpenAI-style servers expect a /v1 prefix; accept a bare host:port for convenience.
    if api.is_openai() {
        let after_scheme = h.split("://").nth(1).unwrap_or("");
        if !after_scheme.contains('/') {
            h.push_str("/v1");
        }
    }
    h
}

impl Llm {
    pub fn new(api: Api, host: &str, model: &str, num_ctx: u32, key: Option<String>) -> Llm {
        // "Big and slow" is fine: generous read timeout (a 70B model on a laptop CPU is patient work).
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(3600))
            .build();
        Llm { host: normalize_host(host, api), api, model: model.to_string(), num_ctx, key: key.filter(|k| !k.is_empty()), agent }
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    fn req(&self, method: &str, path: &str) -> ureq::Request {
        let r = self.agent.request(method, &format!("{}{}", self.host, path));
        match &self.key {
            Some(k) => r.set("Authorization", &format!("Bearer {k}")),
            None => r,
        }
    }

    fn map_err(&self, e: ureq::Error, model: &str) -> anyhow::Error {
        match e {
            ureq::Error::Status(code, r) => {
                let msg = r.into_string().unwrap_or_default();
                let hint = if self.api == Api::Ollama {
                    format!("is the model pulled? try: ollama pull {model}")
                } else {
                    format!("is the model '{model}' loaded/served and able to do this? (run `tezaur models` to list what the server offers)")
                };
                anyhow!("server returned HTTP {code}: {msg}\n({hint})")
            }
            e => {
                let how = match self.api {
                    Api::Ollama => "Start it with `ollama serve`, or set --host / OLLAMA_HOST.",
                    Api::Jan => "In Jan: Settings > Local API Server > Start Server (default port 1337).",
                    Api::Openai => "Check that your server is running and --host points at its base URL.",
                };
                anyhow!("cannot reach the model server at {} ({e}).\n{how}", self.host)
            }
        }
    }

    fn post(&self, path: &str, body: Value) -> Result<ureq::Response> {
        let model = body.get("model").and_then(|m| m.as_str()).unwrap_or(&self.model).to_string();
        self.req("POST", path).send_json(body).map_err(|e| self.map_err(e, &model))
    }

    /// Model ids the server offers.
    pub fn models(&self) -> Result<Vec<String>> {
        let path = if self.api.is_openai() { "/models" } else { "/api/tags" };
        let v: Value = self.req("GET", path).call().map_err(|e| self.map_err(e, &self.model))?.into_json()?;
        let (arr, key) = if self.api.is_openai() { (v.get("data"), "id") } else { (v.get("models"), "name") };
        Ok(arr
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|m| m.get(key).and_then(|s| s.as_str()).map(String::from)).collect())
            .unwrap_or_default())
    }

    /// Streams the reply, calling `on_token` for each fragment; returns the full text.
    pub fn chat(&self, system: &str, user: &str, mut on_token: impl FnMut(&str)) -> Result<String> {
        let messages = json!([
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]);
        let mut full = String::new();
        if self.api.is_openai() {
            // Context size is a server-side model setting for OpenAI-style servers; it cannot be requested here.
            let body = json!({ "model": self.model, "stream": true, "temperature": 0.2, "messages": messages });
            let resp = self.post("/chat/completions", body)?;
            for line in BufReader::new(resp.into_reader()).lines() {
                let line = line.context("connection to the model server dropped mid-answer")?;
                let Some(data) = line.trim().strip_prefix("data:") else { continue };
                let data = data.trim();
                if data == "[DONE]" {
                    break;
                }
                if data.is_empty() {
                    continue;
                }
                let v: Value = serde_json::from_str(data).context("unexpected stream line from server")?;
                if let Some(err) = v.get("error") {
                    bail!("server error: {err}");
                }
                if let Some(t) = v.pointer("/choices/0/delta/content").and_then(|t| t.as_str()) {
                    if !t.is_empty() {
                        on_token(t);
                        full.push_str(t);
                    }
                }
            }
        } else {
            let body = json!({
                "model": self.model,
                "stream": true,
                "options": { "num_ctx": self.num_ctx, "temperature": 0.2 },
                "messages": messages
            });
            let resp = self.post("/api/chat", body)?;
            for line in BufReader::new(resp.into_reader()).lines() {
                let line = line.context("connection to Ollama dropped mid-answer")?;
                if line.trim().is_empty() {
                    continue;
                }
                let v: Value = serde_json::from_str(&line).context("unexpected stream line from Ollama")?;
                if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                    bail!("Ollama error: {err}");
                }
                if let Some(t) = v.pointer("/message/content").and_then(|t| t.as_str()) {
                    if !t.is_empty() {
                        on_token(t);
                        full.push_str(t);
                    }
                }
                if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    break;
                }
            }
        }
        Ok(full)
    }

    /// Embeds a batch of texts; vectors come back L2-normalised.
    pub fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        let (path, rows): (&str, Vec<Value>) = if self.api.is_openai() {
            let v: Value = self.post("/embeddings", json!({ "model": model, "input": inputs }))?.into_json()?;
            let mut data = v.get("data").and_then(|e| e.as_array()).cloned().ok_or_else(|| anyhow!("no embeddings in reply"))?;
            data.sort_by_key(|d| d.get("index").and_then(|i| i.as_u64()).unwrap_or(0));
            ("openai", data.into_iter().map(|d| d.get("embedding").cloned().unwrap_or(Value::Null)).collect())
        } else {
            let v: Value = self.post("/api/embed", json!({ "model": model, "input": inputs }))?.into_json()?;
            ("ollama", v.get("embeddings").and_then(|e| e.as_array()).cloned().ok_or_else(|| anyhow!("no embeddings in reply"))?)
        };
        let _ = path;
        if rows.len() != inputs.len() {
            bail!("embedding count mismatch ({} vs {})", rows.len(), inputs.len());
        }
        rows.iter()
            .map(|row| {
                let mut f: Vec<f32> = row
                    .as_array()
                    .ok_or_else(|| anyhow!("bad embedding row"))?
                    .iter()
                    .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                    .collect();
                normalize(&mut f);
                Ok(f)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// Minimal fake server: answers one request, closes the connection. Returns (host, captured request head+body).
    fn fake_server(response_body: &'static str) -> (String, Arc<Mutex<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        let cap = captured.clone();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                loop {
                    let n = s.read(&mut tmp).unwrap();
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..p]).to_lowercase();
                        let cl: usize = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:").map(|v| v.trim().parse().unwrap()))
                            .unwrap_or(0);
                        if buf.len() >= p + 4 + cl {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                *cap.lock().unwrap() = String::from_utf8_lossy(&buf).to_string();
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{response_body}");
            }
        });
        (format!("127.0.0.1:{}", addr.port()), captured)
    }

    #[test]
    fn streams_ollama_chat_tokens() {
        let (host, _) = fake_server(
            "{\"message\":{\"content\":\"Hel\"},\"done\":false}\n{\"message\":{\"content\":\"lo [1]\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true}\n",
        );
        let llm = Llm::new(Api::Ollama, &host, "m", 2048, None);
        let mut seen = Vec::new();
        let full = llm.chat("sys", "usr", |t| seen.push(t.to_string())).unwrap();
        assert_eq!(full, "Hello [1]");
        assert_eq!(seen, vec!["Hel", "lo [1]"]);
    }

    #[test]
    fn streams_openai_sse_and_sends_key() {
        let (host, cap) = fake_server(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"Ja\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"n [2]\"}}]}\n\ndata: [DONE]\n\n",
        );
        let llm = Llm::new(Api::Jan, &host, "my-model", 2048, Some("secret".into()));
        let mut seen = Vec::new();
        let full = llm.chat("sys", "usr", |t| seen.push(t.to_string())).unwrap();
        assert_eq!(full, "Jan [2]");
        assert_eq!(seen, vec!["Ja", "n [2]"]);
        let req = cap.lock().unwrap().to_lowercase();
        assert!(req.starts_with("post /v1/chat/completions"), "bare host gets /v1: {req}");
        assert!(req.contains("authorization: bearer secret"));
        assert!(req.contains("\"model\":\"my-model\""));
    }

    #[test]
    fn embeds_and_normalises_ollama() {
        let (host, _) = fake_server("{\"embeddings\":[[3.0,4.0],[0.0,2.0]]}");
        let llm = Llm::new(Api::Ollama, &host, "m", 2048, None);
        let e = llm.embed("e", &["a".into(), "b".into()]).unwrap();
        assert!((e[0][0] - 0.6).abs() < 1e-6 && (e[0][1] - 0.8).abs() < 1e-6);
        assert!((e[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn embeds_openai_in_index_order() {
        let (host, _) = fake_server("{\"data\":[{\"index\":1,\"embedding\":[0.0,2.0]},{\"index\":0,\"embedding\":[3.0,4.0]}]}");
        let llm = Llm::new(Api::Openai, &format!("http://{host}/v1"), "m", 2048, None);
        let e = llm.embed("e", &["a".into(), "b".into()]).unwrap();
        assert!((e[0][0] - 0.6).abs() < 1e-6, "index 0 first");
        assert!((e[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn lists_models_for_both_protocols() {
        let (h1, _) = fake_server("{\"data\":[{\"id\":\"qwen3-8b\"},{\"id\":\"llama-3.1\"}]}");
        assert_eq!(Llm::new(Api::Jan, &h1, "m", 1, None).models().unwrap(), vec!["qwen3-8b", "llama-3.1"]);
        let (h2, _) = fake_server("{\"models\":[{\"name\":\"llama3.1:8b\"}]}");
        assert_eq!(Llm::new(Api::Ollama, &h2, "m", 1, None).models().unwrap(), vec!["llama3.1:8b"]);
    }

    #[test]
    fn unreachable_server_gives_helpful_error() {
        let err = Llm::new(Api::Ollama, "127.0.0.1:1", "m", 2048, None).chat("s", "u", |_| {}).unwrap_err().to_string();
        assert!(err.contains("ollama serve"));
        let err = Llm::new(Api::Jan, "127.0.0.1:1", "m", 2048, None).chat("s", "u", |_| {}).unwrap_err().to_string();
        assert!(err.contains("Local API Server"));
    }
}
