//! TezaurPortable — launcher for the PortableApps.com folder layout.
//!
//!   TezaurPortable/
//!     TezaurPortable.exe          <- this program
//!     App/Tezaur/tezaur.exe       <- the application
//!     App/Ollama/ollama.exe       <- OPTIONAL portable Ollama (unzip the official Windows zip here)
//!     Data/                       <- index, library, settings, Ollama models (all user data)
//!
//! It keeps every path relative to its own location (drive letters may change between runs),
//! points the app and Ollama at Data/, and shuts down an Ollama it started itself on exit.

use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const OLLAMA_ADDR: &str = "127.0.0.1:11434";

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn ollama_up() -> bool {
    let addr: SocketAddr = OLLAMA_ADDR.parse().unwrap();
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// Starts the bundled Ollama (if present and nothing already listens on its port).
fn start_ollama(app: &Path, data: &Path) -> Option<Child> {
    let exe = app.join("Ollama").join(exe_name("ollama"));
    if !exe.exists() || ollama_up() {
        return None;
    }
    let models = data.join("ollama").join("models");
    let _ = std::fs::create_dir_all(&models);
    let mut cmd = Command::new(&exe);
    cmd.arg("serve")
        .env("OLLAMA_MODELS", &models)
        .env("OLLAMA_HOST", OLLAMA_ADDR)
        .current_dir(exe.parent().unwrap())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let child = cmd.spawn().ok()?;
    eprintln!("Starting bundled Ollama (models in {}) …", models.display());
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(20) && !ollama_up() {
        std::thread::sleep(Duration::from_millis(250));
    }
    Some(child)
}

fn pause_and_exit(code: i32) -> ! {
    eprintln!("\nPress Enter to close.");
    let _ = std::io::stdin().read_line(&mut String::new());
    std::process::exit(code)
}

fn main() {
    let exe = std::env::current_exe().expect("cannot locate launcher");
    let root = exe.parent().expect("launcher has no folder").to_path_buf();
    let app = root.join("App");
    let data = root.join("Data");
    let tezaur = app.join("Tezaur").join(exe_name("tezaur"));

    if !tezaur.exists() {
        eprintln!(
            "Cannot find {}\nThe package looks incomplete — re-extract it.",
            tezaur.display()
        );
        pause_and_exit(1);
    }
    let _ = std::fs::create_dir_all(&data);

    let mut server = start_ollama(&app, &data);

    let status = Command::new(&tezaur)
        .args(std::env::args_os().skip(1))
        .env("TEZAUR_DATA", &data)
        .current_dir(&data)
        .status();

    if let Some(s) = server.as_mut() {
        let _ = s.kill();
        let _ = s.wait();
    }

    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("Could not start {}: {e}", tezaur.display());
            pause_and_exit(1);
        }
    }
}
