//! PortableApps.com-style portability: all user data lives in `<root>/Data`, nothing is written
//! to the registry, %APPDATA% or the home directory, and the folder can move between drives.

use std::path::{Path, PathBuf};

/// Layout: `<root>/App/Tezaur/tezaur(.exe)`  ->  data in `<root>/Data` (must already exist),
/// or a `tezaur.portable` marker file next to the executable  ->  `<exe dir>/Data`.
pub fn detect_from_exe(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    if let Some(app) = dir.parent() {
        let is_app = app
            .file_name()
            .map(|n| n.eq_ignore_ascii_case("App"))
            .unwrap_or(false);
        if is_app && let Some(root) = app.parent() {
            let d = root.join("Data");
            if d.is_dir() {
                return Some(d);
            }
        }
    }
    if dir.join("tezaur.portable").exists() {
        return Some(dir.join("Data"));
    }
    None
}

/// `TEZAUR_DATA` (set by the launcher) wins; otherwise auto-detect from the executable's location.
pub fn data_dir() -> Option<PathBuf> {
    let d = match std::env::var_os("TEZAUR_DATA") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => detect_from_exe(&std::env::current_exe().ok()?)?,
    };
    ensure_layout(&d);
    Some(d)
}

const CONF_TEMPLATE: &str = "\
# TEZAUR portable settings. Lines starting with # are ignored.
# Environment variables and command-line flags override this file.
#
# Model server protocol: ollama (default), jan, or openai (LM Studio, llama.cpp server, ...)
#api = ollama
# Server address. Defaults: Ollama http://localhost:11434, Jan http://127.0.0.1:1337/v1
#host = http://127.0.0.1:1337/v1
# Only if the server requires a key (Jan: Settings > Local API Server > API key)
#api_key =
# Generation model id as the server names it (run `tezaur models` to list). Bigger = better, slower.
#model = llama3.1:8b
# Embedding model for hybrid search (optional; multilingual: bge-m3)
#embed_model = bge-m3
# Context window requested from the model
#num_ctx = 16384
";

pub fn ensure_layout(data: &Path) {
    let _ = std::fs::create_dir_all(data.join("settings"));
    let _ = std::fs::create_dir_all(data.join("Library"));
    let conf = data.join("settings").join("tezaur.conf");
    if !conf.exists() {
        let _ = std::fs::write(conf, CONF_TEMPLATE);
    }
}

pub fn parse_conf(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with(';'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let v = v.trim().trim_matches('"').trim();
            (!v.is_empty()).then(|| (k.trim().to_lowercase(), v.to_string()))
        })
        .collect()
}

fn env_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "host" => "TEZAUR_HOST",
        "api" => "TEZAUR_API",
        "api_key" => "TEZAUR_API_KEY",
        "model" => "TEZAUR_MODEL",
        "embed_model" => "TEZAUR_EMBED_MODEL",
        "num_ctx" => "TEZAUR_NUM_CTX",
        _ => return None,
    })
}

/// Loads `Data/settings/tezaur.conf` as defaults (never overrides variables already set).
/// Must run before argument parsing and before any threads are started.
pub fn apply_conf(data: &Path) {
    let Ok(text) = std::fs::read_to_string(data.join("settings").join("tezaur.conf")) else {
        return;
    };
    for (k, v) in parse_conf(&text) {
        if let Some(name) = env_name(&k)
            && std::env::var_os(name).is_none()
        {
            // SAFETY: called once at startup, single-threaded, before clap parses.
            unsafe { std::env::set_var(name, v) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn detects_portableapps_layout() {
        let root = scratch("pa_layout");
        std::fs::create_dir_all(root.join("App/Tezaur")).unwrap();
        std::fs::create_dir_all(root.join("Data")).unwrap();
        let exe = root.join("App/Tezaur/tezaur.exe");
        assert_eq!(detect_from_exe(&exe), Some(root.join("Data")));
        // No Data dir and no marker: not portable.
        let other = scratch("not_portable");
        assert_eq!(detect_from_exe(&other.join("bin/tezaur")), None);
    }

    #[test]
    fn detects_marker_file() {
        let dir = scratch("marker");
        std::fs::write(dir.join("tezaur.portable"), "").unwrap();
        assert_eq!(detect_from_exe(&dir.join("tezaur")), Some(dir.join("Data")));
    }

    #[test]
    fn parses_conf() {
        let c =
            parse_conf("# c\nmodel = \"qwen2.5:14b\"\n\nHOST=127.0.0.1:11434\nbad line\nempty=\n");
        assert_eq!(
            c,
            vec![
                ("model".into(), "qwen2.5:14b".into()),
                ("host".into(), "127.0.0.1:11434".into())
            ]
        );
    }

    #[test]
    fn creates_layout_and_template() {
        let d = scratch("layout_create");
        ensure_layout(&d);
        assert!(d.join("Library").is_dir());
        assert!(
            std::fs::read_to_string(d.join("settings/tezaur.conf"))
                .unwrap()
                .contains("embed_model")
        );
        // A pristine template sets nothing.
        assert!(parse_conf(CONF_TEMPLATE).is_empty());
    }
}
