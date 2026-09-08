//! Dispatch autonomous tickets to local inference (raw HTTP → MLX on the stove).

use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::config::{expand_path, repo_root};
use crate::kitchen::agent_log;
use crate::stove::{self, Burner};

const DISPATCH_TIMEOUT: Duration = Duration::from_secs(300);
const SIMMER_WAIT: Duration = Duration::from_secs(90);
const READ_SLICE: Duration = Duration::from_millis(500);

fn cancels() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static SLOT: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_cancel(agent_id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    cancels()
        .lock()
        .expect("cancel registry")
        .insert(agent_id.to_string(), flag.clone());
    flag
}

fn unregister_cancel(agent_id: &str) {
    cancels().lock().expect("cancel registry").remove(agent_id);
}

/// Request in-flight dispatch for `agent_id` to stop.
pub fn cancel_dispatch(agent_id: &str) -> bool {
    cancels()
        .lock()
        .expect("cancel registry")
        .get(agent_id)
        .map(|f| {
            f.store(true, Ordering::SeqCst);
            true
        })
        .unwrap_or(false)
}

pub fn dispatch_active(agent_id: &str) -> bool {
    cancels()
        .lock()
        .expect("cancel registry")
        .contains_key(agent_id)
}

/// Run an autonomous ticket: warm MLX, POST chat completion, append reply to agent log.
pub fn dispatch_agent(agent_id: &str, prompt: &str) -> Result<()> {
    let cancel = register_cancel(agent_id);
    let result = dispatch_agent_inner(agent_id, prompt, &cancel);
    let cancelled = cancel.load(Ordering::SeqCst);
    unregister_cancel(agent_id);

    match result {
        Ok(()) if cancelled => {
            let _ = agent_log(agent_id, "done · cancelled");
            bail!("dispatch cancelled");
        }
        Ok(()) => Ok(()),
        Err(_) if cancelled => {
            let _ = agent_log(agent_id, "done · cancelled");
            bail!("dispatch cancelled");
        }
        Err(e) => {
            let _ = agent_log(agent_id, &format!("done · error · {:#}", e));
            Err(e)
        }
    }
}

fn dispatch_agent_inner(
    agent_id: &str,
    prompt: &str,
    cancel: &AtomicBool,
) -> Result<()> {
    let prompt = prompt.trim();
    anyhow::ensure!(!prompt.is_empty(), "empty ticket");
    check_cancel(cancel)?;

    agent_log(agent_id, &format!("dispatch · raw · {prompt}"))?;

    stove::ensure_simmering(Burner::Mlx, SIMMER_WAIT)?;
    check_cancel(cancel)?;

    let port = stove::burner_port(Burner::Mlx);
    let model = default_mlx_model();

    let reply = raw_chat_completion(port, &model, prompt, cancel)?;
    check_cancel(cancel)?;

    let reply = reply.trim();
    if reply.is_empty() {
        agent_log(agent_id, "reply · (empty)")?;
    } else {
        for line in reply.lines() {
            agent_log(agent_id, &format!("reply · {line}"))?;
        }
    }
    agent_log(agent_id, "done · ok")?;
    Ok(())
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        bail!("dispatch cancelled");
    }
    Ok(())
}

fn default_mlx_model() -> String {
    std::env::var("MODEL")
        .map(|p| expand_path(&p).to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            expand_path("~/.lmstudio/models/lmstudio-community/Qwen3.5-9B-MLX-4bit")
                .to_string_lossy()
                .into_owned()
        })
}

fn raw_chat_completion(
    port: u16,
    model: &str,
    prompt: &str,
    cancel: &AtomicBool,
) -> Result<String> {
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 8192,
    });
    let body = body.to_string();
    let raw = http_post_json(port, "/v1/chat/completions", &body, cancel)?;
    parse_chat_reply(&raw)
}

fn http_post_json(port: u16, path: &str, body: &str, cancel: &AtomicBool) -> Result<String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).context("connect MLX")?;
    stream
        .set_read_timeout(Some(READ_SLICE))
        .context("read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .context("write timeout")?;

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .context("write chat request")?;

    let started = std::time::Instant::now();
    let mut resp = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        check_cancel(cancel)?;
        if started.elapsed() > DISPATCH_TIMEOUT {
            bail!("MLX response timed out");
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => resp.extend_from_slice(&buf[..n]),
            Err(e)
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => return Err(e).context("read chat response"),
        }
    }
    let resp = String::from_utf8_lossy(&resp);
    let (_, body) = resp
        .split_once("\r\n\r\n")
        .context("malformed HTTP response")?;
    Ok(body.to_string())
}

fn parse_chat_reply(body: &str) -> Result<String> {
    let v: Value = serde_json::from_str(body).with_context(|| {
        let preview = body.chars().take(200).collect::<String>();
        format!("MLX response not JSON: {preview}")
    })?;
    if let Some(err) = v.get("error") {
        bail!("MLX error: {err}");
    }
    v["choices"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .context("MLX response missing choices[0].message.content")
}

/// Repo root — cwd for future Pi harness dispatch.
pub fn workdir() -> PathBuf {
    repo_root()
}
