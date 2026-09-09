//! Local model server burners — MLX and Ollama on the stove.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{expand_path, repo_root};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Burner {
    Mlx,
    Ollama,
}

impl Burner {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "mlx" => Ok(Self::Mlx),
            "ollama" => Ok(Self::Ollama),
            _ => bail!("unknown burner: {s} (try: mlx, ollama)"),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Mlx => "mlx",
            Self::Ollama => "ollama",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mlx => "MLX",
            Self::Ollama => "Ollama",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Self::Mlx => 8080,
            Self::Ollama => 11434,
        }
    }

    fn log_name(self) -> &'static str {
        match self {
            Self::Mlx => "mlx-server",
            Self::Ollama => "ollama-server",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BurnerRecord {
    pid: u32,
    port: u16,
    started_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoveState {
    #[serde(default)]
    burners: HashMap<String, BurnerRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heat {
    Cold,
    Warming,
    Simmering,
}

impl Heat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Warming => "warming",
            Self::Simmering => "simmering",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BurnerStatus {
    pub burner: Burner,
    pub port: u16,
    pub listening: bool,
    pub pid: Option<u32>,
    pub managed: bool,
}

impl BurnerStatus {
    pub fn heat(&self) -> Heat {
        if self.listening {
            Heat::Simmering
        } else if self.pid.is_some() {
            Heat::Warming
        } else {
            Heat::Cold
        }
    }
}

pub fn status_all() -> Result<Vec<BurnerStatus>> {
    [Burner::Mlx, Burner::Ollama]
        .into_iter()
        .map(status_one)
        .collect()
}

pub fn status_one(burner: Burner) -> Result<BurnerStatus> {
    let port = port_for(burner);
    let listening = server_listening(burner, port);
    let state = load_state();
    let record = state.burners.get(burner.id());

    if let Some(r) = record {
        if !process_alive(r.pid) && !listening {
            clear_burner(burner);
        }
    }

    let state = load_state();
    let record = state.burners.get(burner.id());
    let pid = record
        .map(|r| r.pid)
        .filter(|p| process_alive(*p))
        .or_else(|| {
            if listening {
                find_pid_on_port(port)
            } else {
                None
            }
        });
    let managed = record.is_some_and(|r| Some(r.pid) == pid);
    Ok(BurnerStatus {
        burner,
        port,
        listening,
        pid,
        managed,
    })
}

pub fn burner_port(burner: Burner) -> u16 {
    port_for(burner)
}

/// Start the burner if cold, then wait until HTTP is up or timeout.
pub fn ensure_simmering(burner: Burner, timeout: Duration) -> Result<()> {
    match status_one(burner)?.heat() {
        Heat::Simmering => return Ok(()),
        Heat::Cold => {
            simmer(burner)?;
        }
        Heat::Warming => {}
    }

    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if status_one(burner)?.heat() == Heat::Simmering {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    bail!(
        "{} did not reach simmering in time — check {}",
        burner.label(),
        log_path(burner).display()
    );
}

pub fn simmer(burner: Burner) -> Result<String> {
    let st = status_one(burner)?;
    match st.heat() {
        Heat::Simmering => {
            let pid = st.pid.map(|p| p.to_string()).unwrap_or_else(|| "?".into());
            bail!(
                "{} already simmering on :{} (pid {pid})",
                burner.label(),
                st.port
            );
        }
        Heat::Warming => {
            let pid = st.pid.context("warming but pid missing")?;
            bail!(
                "{} still warming up on :{} (pid {pid}) — check log",
                burner.label(),
                st.port
            );
        }
        Heat::Cold => {}
    }

    let port = st.port;
    let pid = match burner {
        Burner::Mlx => start_mlx(port)?,
        Burner::Ollama => start_ollama(port)?,
    };
    save_burner(burner, pid, port)?;
    Ok(format!("warming {} on :{} (pid {pid})", burner.id(), port))
}

pub fn cool(burner: Burner) -> Result<String> {
    let st = status_one(burner)?;
    match st.heat() {
        Heat::Cold => {
            clear_burner(burner);
            return Ok(format!("{} already cool", burner.id()));
        }
        Heat::Warming | Heat::Simmering => {}
    }

    let pid = st
        .pid
        .or_else(|| find_pid_on_port(st.port))
        .with_context(|| {
            format!(
                ":{} in use but pid unknown — try: lsof -i :{}",
                st.port, st.port
            )
        })?;
    stop_pid(pid)?;

    for _ in 0..30 {
        if !port_listening(st.port) && !process_alive(pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if port_listening(st.port) {
        bail!("pid {pid} sent SIGTERM but :{} still listening", st.port);
    }

    clear_burner(burner);
    Ok(format!("cooled {} (was pid {pid})", burner.id()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub burner: Burner,
    pub model: String,
    pub heat: Heat,
}

/// Models a chat client can pick. MLX always has at least the default path.
/// Ollama appears only while that burner is simmering (never the face when cold).
pub fn model_catalog() -> Result<Vec<CatalogEntry>> {
    let mut out = Vec::new();
    for burner in [Burner::Mlx, Burner::Ollama] {
        let st = status_one(burner)?;
        for model in served_models(burner, &st)? {
            out.push(CatalogEntry {
                burner,
                model,
                heat: st.heat(),
            });
        }
    }
    if !out.iter().any(|e| e.burner == Burner::Mlx) {
        out.insert(
            0,
            CatalogEntry {
                burner: Burner::Mlx,
                model: default_mlx_model(),
                heat: status_one(Burner::Mlx)?.heat(),
            },
        );
    }
    Ok(out)
}

pub fn served_models(burner: Burner, st: &BurnerStatus) -> Result<Vec<String>> {
    if st.heat() != Heat::Simmering {
        return Ok(match burner {
            Burner::Mlx => vec![default_mlx_model()],
            Burner::Ollama => Vec::new(),
        });
    }
    match fetch_v1_models(st.port) {
        Ok(list) if !list.is_empty() => Ok(list),
        Ok(_) | Err(_) if burner == Burner::Mlx => Ok(vec![default_mlx_model()]),
        _ => Ok(Vec::new()),
    }
}

pub fn default_mlx_model() -> String {
    std::env::var("MODEL")
        .map(|p| expand_path(&p).to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            expand_path("~/.lmstudio/models/lmstudio-community/Qwen3.5-9B-MLX-4bit")
                .to_string_lossy()
                .into_owned()
        })
}

pub fn parse_v1_models(body: &str) -> Vec<String> {
    let json = extract_json_object(body).unwrap_or(body);
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(str::to_string))
                .filter(|id| !id.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end >= start {
        Some(&s[start..=end])
    } else {
        None
    }
}

fn fetch_v1_models(port: u16) -> Result<Vec<String>> {
    let raw = http_get_body(port, "/v1/models")?;
    Ok(parse_v1_models(&raw))
}

fn http_get_body(port: u16, path: &str) -> Result<String> {
    use std::io::{Read, Write};

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).context("connect models")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .context("read timeout")?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).context("write models")?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).context("read models")?;
    String::from_utf8(buf).context("models not utf-8")
}

pub fn print_status() -> Result<()> {
    for st in status_all()? {
        let heat = st.heat().label();
        let pid = st.pid.map(|p| p.to_string()).unwrap_or_else(|| "—".into());
        let tag = if st.managed {
            "kitchen"
        } else if st.heat() != Heat::Cold {
            "external"
        } else {
            ""
        };
        let extra = if tag.is_empty() {
            String::new()
        } else {
            format!("  [{tag}]")
        };
        println!(
            "{:<8}  {:<10}  :{:<5}  pid {pid}{extra}",
            st.burner.id(),
            heat,
            st.port
        );
    }
    Ok(())
}

fn port_for(burner: Burner) -> u16 {
    let key = match burner {
        Burner::Mlx => "KITCHEN_MLX_PORT",
        Burner::Ollama => "KITCHEN_OLLAMA_PORT",
    };
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| burner.default_port())
}

fn server_listening(burner: Burner, port: u16) -> bool {
    if http_ready(burner, port) {
        return true;
    }
    if find_pid_on_port(port).is_some() {
        return true;
    }
    tcp_open(port)
}

fn http_ready(burner: Burner, port: u16) -> bool {
    use std::io::{Read, Write};

    let path = match burner {
        Burner::Mlx => "/v1/models",
        Burner::Ollama => "/",
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 12];
    let n = stream.read(&mut buf).unwrap_or(0);
    http_status_ok(&buf[..n])
}

fn http_status_ok(buf: &[u8]) -> bool {
    buf.len() >= 12 && buf[9].is_ascii_digit()
}

fn tcp_open(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

fn port_listening(port: u16) -> bool {
    if find_pid_on_port(port).is_some() {
        return true;
    }
    tcp_open(port)
}

fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_pid_on_port(port: u16) -> Option<u32> {
    let out = Command::new("lsof")
        .args(["-i", &format!(":{port}"), "-sTCP:LISTEN", "-t"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .and_then(|line| line.trim().parse().ok())
}

fn stop_pid(pid: u32) -> Result<()> {
    let out = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("kill")?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("No such process") {
        return Ok(());
    }
    bail!("kill -TERM {pid} failed: {}", stderr.trim());
}

fn start_mlx(port: u16) -> Result<u32> {
    let script = expand_path("~/.pi/start-qwen-server.sh");
    if script.is_file() {
        return spawn_logged(
            Command::new(&script).env("PORT", port.to_string()),
            Burner::Mlx,
        );
    }

    let model = std::env::var("MODEL")
        .map(|p| expand_path(&p))
        .unwrap_or_else(|_| {
            expand_path("~/.lmstudio/models/lmstudio-community/Qwen3.5-9B-MLX-4bit")
        });
    if !model.is_dir() {
        bail!(
            "MLX model not found at {} — set MODEL or install ~/.pi/start-qwen-server.sh",
            model.display()
        );
    }

    let python = expand_path("~/.pi/mlx-server-venv/bin/python");
    if !python.is_file() {
        bail!(
            "MLX venv python not found at {} — run harness notes in harnesses/pi/qwen-local.md",
            python.display()
        );
    }

    spawn_logged(
        Command::new(&python).env("APC_ENABLED", "1").args([
            "-m",
            "mlx_vlm",
            "server",
            "--model",
            &model.to_string_lossy(),
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--max-kv-size",
            "32768",
            "--max-tokens",
            "8192",
            "--max-num-seqs",
            "1",
            "--model-discovery",
            "served",
            "--log-level",
            "INFO",
        ]),
        Burner::Mlx,
    )
}

fn start_ollama(port: u16) -> Result<u32> {
    let ollama = which("ollama")
        .context("ollama not on PATH — install from https://ollama.com or `brew install ollama`")?;
    let host = format!("127.0.0.1:{port}");
    spawn_logged(
        Command::new(&ollama).env("OLLAMA_HOST", host).arg("serve"),
        Burner::Ollama,
    )
}

fn which(bin: &str) -> Result<PathBuf> {
    let out = Command::new("which").arg(bin).output().context("which")?;
    if !out.status.success() {
        bail!("{bin} not found");
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        bail!("{bin} not found");
    }
    Ok(PathBuf::from(path))
}

fn spawn_logged(cmd: &mut Command, burner: Burner) -> Result<u32> {
    fs::create_dir_all(kitchen_dir().join("logs"))?;
    let log = log_path(burner);
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .with_context(|| format!("open log {}", log.display()))?;
        writeln!(
            file,
            "\n--- kitchen stove simmer {} @ {stamp} ---",
            burner.id()
        )?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .with_context(|| format!("open log {}", log.display()))?;

    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(file.try_clone()?))
        .stderr(Stdio::from(file))
        .spawn()
        .with_context(|| format!("start {}", burner.id()))?;

    let pid = child.id();
    std::thread::sleep(Duration::from_millis(300));
    if !process_alive(pid) {
        bail!(
            "{} exited immediately — check {}",
            burner.label(),
            log.display()
        );
    }
    Ok(pid)
}

fn kitchen_dir() -> PathBuf {
    repo_root().join(".kitchen")
}

fn state_path() -> PathBuf {
    kitchen_dir().join("stove.json")
}

pub fn log_path(burner: Burner) -> PathBuf {
    kitchen_dir()
        .join("logs")
        .join(format!("{}.log", burner.log_name()))
}

fn load_state() -> StoveState {
    read_json(&state_path()).unwrap_or_default()
}

fn save_burner(burner: Burner, pid: u32, port: u16) -> Result<()> {
    let mut state = load_state();
    state.burners.insert(
        burner.id().to_string(),
        BurnerRecord {
            pid,
            port,
            started_at: chrono::Local::now().to_rfc3339(),
        },
    );
    write_json(&state_path(), &state)?;
    Ok(())
}

fn clear_burner(burner: Burner) {
    let mut state = load_state();
    state.burners.remove(burner.id());
    let _ = write_json(&state_path(), &state);
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burner_parse() {
        assert_eq!(Burner::parse("mlx").unwrap(), Burner::Mlx);
        assert_eq!(Burner::parse("OLLAMA").unwrap(), Burner::Ollama);
        assert!(Burner::parse("gpt").is_err());
    }

    #[test]
    fn default_ports() {
        assert_eq!(Burner::Mlx.default_port(), 8080);
        assert_eq!(Burner::Ollama.default_port(), 11434);
    }

    #[test]
    fn heat_from_signals() {
        let cold = BurnerStatus {
            burner: Burner::Mlx,
            port: 8080,
            listening: false,
            pid: None,
            managed: false,
        };
        assert_eq!(cold.heat(), Heat::Cold);

        let warming = BurnerStatus {
            burner: Burner::Mlx,
            port: 8080,
            listening: false,
            pid: Some(42),
            managed: true,
        };
        assert_eq!(warming.heat(), Heat::Warming);

        let simmering = BurnerStatus {
            burner: Burner::Mlx,
            port: 8080,
            listening: true,
            pid: Some(42),
            managed: true,
        };
        assert_eq!(simmering.heat(), Heat::Simmering);
    }

    #[test]
    fn http_status_line_ok() {
        assert!(http_status_ok(b"HTTP/1.1 200"));
        assert!(http_status_ok(b"HTTP/1.1 404"));
        assert!(!http_status_ok(b"HTTP/1.1 "));
        assert!(!http_status_ok(b"garbage"));
    }

    #[test]
    fn parse_openai_model_list() {
        let body = r#"{"object":"list","data":[{"id":"qwen"},{"id":"llama"}]}"#;
        assert_eq!(parse_v1_models(body), vec!["qwen", "llama"]);
    }

    #[test]
    fn parse_models_skips_http_prefix() {
        let body = "HTTP/1.1 200 OK\r\n\r\n{\"data\":[{\"id\":\"only\"}]}";
        assert_eq!(parse_v1_models(body), vec!["only"]);
    }
}
