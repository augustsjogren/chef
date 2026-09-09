//! Naked local chat — stream OpenAI-compatible completions via a stove burner.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::repo_root;
use crate::stove::{self, Burner};

const DISPATCH_TIMEOUT: Duration = Duration::from_secs(300);
const READ_SLICE: Duration = Duration::from_millis(500);
const SIMMER_WAIT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub id: String,
    pub burner: Burner,
    pub model: String,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub updated_at: String,
}

impl Transcript {
    pub fn new(burner: Burner, model: String) -> Self {
        Self {
            id: chrono::Local::now().format("%Y%m%d-%H%M%S").to_string(),
            burner,
            model,
            messages: Vec::new(),
            updated_at: now_stamp(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Content(String),
    Done,
    Error(String),
}

pub fn load_active() -> Result<Option<Transcript>> {
    let active = chats_dir().join("active");
    let Ok(id) = fs::read_to_string(&active) else {
        return Ok(None);
    };
    let id = id.trim();
    if id.is_empty() {
        return Ok(None);
    }
    let path = chats_dir().join(format!("{id}.json"));
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&raw)?))
}

pub fn save_transcript(t: &Transcript) -> Result<()> {
    let dir = chats_dir();
    fs::create_dir_all(&dir)?;
    let mut t = t.clone();
    t.updated_at = now_stamp();
    let path = dir.join(format!("{}.json", t.id));
    fs::write(&path, serde_json::to_string_pretty(&t)? + "\n")?;
    fs::write(dir.join("active"), format!("{}\n", t.id))?;
    Ok(())
}

/// Warm the burner if needed, then stream a completion. `messages` is the full
/// naked thread (no injected system prompt).
pub fn stream_reply(
    burner: Burner,
    model: &str,
    messages: &[ChatMessage],
    cancel: &AtomicBool,
    mut on_delta: impl FnMut(&str),
) -> Result<()> {
    anyhow::ensure!(
        messages
            .iter()
            .any(|m| m.role == "user" && !m.content.trim().is_empty()),
        "empty chat"
    );
    check_cancel(cancel)?;
    stove::ensure_simmering(burner, SIMMER_WAIT)?;
    check_cancel(cancel)?;
    let port = stove::burner_port(burner);
    stream_chat(port, model, messages, cancel, &mut on_delta)
}

pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') {
        return None;
    }
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(StreamEvent::Done);
    }
    let v: Value = serde_json::from_str(data).ok()?;
    if let Some(err) = v.get("error") {
        return Some(StreamEvent::Error(err.to_string()));
    }
    let content = v
        .pointer("/choices/0/delta/content")
        .and_then(|c| c.as_str())
        .or_else(|| {
            v.pointer("/choices/0/message/content")
                .and_then(|c| c.as_str())
        });
    match content {
        Some(c) if !c.is_empty() => Some(StreamEvent::Content(c.to_string())),
        _ => None,
    }
}

/// Incremental chunked-transfer decoder. Body bytes (not chunk metadata) go to `out`.
pub fn feed_chunked(state: &mut ChunkState, input: &[u8], out: &mut Vec<u8>) {
    for &b in input {
        if state.done {
            break;
        }
        match state.phase {
            ChunkPhase::Size => {
                if b == b'\n' {
                    let hex = {
                        let raw = String::from_utf8_lossy(&state.line);
                        raw.split(';').next().unwrap_or("").trim().to_string()
                    };
                    state.line.clear();
                    if hex.is_empty() {
                        continue;
                    }
                    let size = usize::from_str_radix(&hex, 16).unwrap_or(0);
                    if size == 0 {
                        state.done = true;
                    } else {
                        state.phase = ChunkPhase::Data(size);
                    }
                } else if b != b'\r' {
                    state.line.push(b);
                }
            }
            ChunkPhase::Data(left) => {
                out.push(b);
                if left <= 1 {
                    state.phase = ChunkPhase::Cr;
                } else {
                    state.phase = ChunkPhase::Data(left - 1);
                }
            }
            ChunkPhase::Cr => {
                state.phase = ChunkPhase::Lf;
            }
            ChunkPhase::Lf => {
                state.phase = ChunkPhase::Size;
                if b != b'\n' && b != b'\r' {
                    // stray byte after chunk — treat as start of next size line
                    state.line.push(b);
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ChunkState {
    phase: ChunkPhase,
    line: Vec<u8>,
    done: bool,
}

#[derive(Debug, Default)]
enum ChunkPhase {
    #[default]
    Size,
    Data(usize),
    Cr,
    Lf,
}

fn stream_chat(
    port: u16,
    model: &str,
    messages: &[ChatMessage],
    cancel: &AtomicBool,
    on_delta: &mut impl FnMut(&str),
) -> Result<()> {
    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
        .collect();
    let body = serde_json::json!({
        "model": model,
        "messages": msgs,
        "max_tokens": 8192,
        "stream": true,
    })
    .to_string();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).context("connect chat")?;
    stream
        .set_read_timeout(Some(READ_SLICE))
        .context("read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .context("write timeout")?;

    let req = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .context("write chat request")?;

    let started = Instant::now();
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let mut headers_done = false;
    let mut chunked = false;
    let mut chunk = ChunkState::default();
    let mut sse = String::new();
    let mut finished = false;

    loop {
        check_cancel(cancel)?;
        if started.elapsed() > DISPATCH_TIMEOUT {
            bail!("chat timed out");
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if !headers_done {
                    raw.extend_from_slice(&buf[..n]);
                    if let Some((status, is_chunked, rest)) = split_headers(&raw) {
                        if !(200..300).contains(&status) {
                            let preview = String::from_utf8_lossy(rest);
                            let json = extract_error(&preview);
                            bail!("chat HTTP {status}: {json}");
                        }
                        headers_done = true;
                        chunked = is_chunked;
                        feed_body(chunked, &mut chunk, rest, &mut sse, on_delta, &mut finished)?;
                        if finished {
                            return Ok(());
                        }
                    }
                } else {
                    feed_body(
                        chunked,
                        &mut chunk,
                        &buf[..n],
                        &mut sse,
                        on_delta,
                        &mut finished,
                    )?;
                    if finished {
                        return Ok(());
                    }
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => return Err(e).context("read chat stream"),
        }
    }

    if !finished {
        drain_sse(&mut sse, on_delta, &mut finished)?;
    }
    if !finished {
        if let Some(text) = parse_non_stream_json(&sse) {
            if !text.is_empty() {
                on_delta(&text);
            }
        }
    }
    if !headers_done {
        bail!("chat: no HTTP response");
    }
    Ok(())
}

fn feed_body(
    chunked: bool,
    chunk: &mut ChunkState,
    input: &[u8],
    sse: &mut String,
    on_delta: &mut impl FnMut(&str),
    finished: &mut bool,
) -> Result<()> {
    if chunked {
        let mut decoded = Vec::new();
        feed_chunked(chunk, input, &mut decoded);
        sse.push_str(&String::from_utf8_lossy(&decoded));
    } else {
        sse.push_str(&String::from_utf8_lossy(input));
    }
    drain_sse(sse, on_delta, finished)
}

fn drain_sse(sse: &mut String, on_delta: &mut impl FnMut(&str), finished: &mut bool) -> Result<()> {
    while let Some(idx) = sse.find('\n') {
        let line: String = sse.drain(..=idx).collect();
        match parse_sse_line(line.trim_end()) {
            Some(StreamEvent::Content(c)) => on_delta(&c),
            Some(StreamEvent::Done) => {
                *finished = true;
                return Ok(());
            }
            Some(StreamEvent::Error(e)) => bail!("model error: {e}"),
            None => {}
        }
    }
    Ok(())
}

fn split_headers(buf: &[u8]) -> Option<(u16, bool, &[u8])> {
    let sep = buf.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&buf[..sep]).ok()?;
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())?;
    let chunked = head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    Some((status, chunked, &buf[sep + 4..]))
}

fn parse_non_stream_json(body: &str) -> Option<String> {
    let start = body.find('{')?;
    let end = body.rfind('}')?;
    if end < start {
        return None;
    }
    let v: Value = serde_json::from_str(&body[start..=end]).ok()?;
    v.pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .map(str::to_string)
}

fn extract_error(body: &str) -> String {
    let json = body.find('{').and_then(|s| {
        body.rfind('}')
            .map(|e| if e >= s { &body[s..=e] } else { body })
    });
    json.unwrap_or(body).chars().take(200).collect()
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        bail!("chat stopped");
    }
    Ok(())
}

fn now_stamp() -> String {
    chrono::Local::now().to_rfc3339()
}

fn chats_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(dir) = TEST_CHATS_DIR
        .try_with(|slot| slot.borrow().clone())
        .ok()
        .flatten()
    {
        return dir;
    }
    repo_root().join(".kitchen").join("chats")
}

#[cfg(test)]
thread_local! {
    static TEST_CHATS_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn set_test_chats_dir(dir: Option<PathBuf>) {
    TEST_CHATS_DIR.with(|slot| *slot.borrow_mut() = dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_content_and_done() {
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#),
            Some(StreamEvent::Content("hi".into()))
        );
        assert_eq!(parse_sse_line("data: [DONE]"), Some(StreamEvent::Done));
        assert_eq!(parse_sse_line(": keep-alive"), None);
        assert_eq!(parse_sse_line(""), None);
    }

    #[test]
    fn sse_error() {
        match parse_sse_line(r#"data: {"error":"nope"}"#) {
            Some(StreamEvent::Error(e)) => assert!(e.contains("nope")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn chunked_roundtrip() {
        let mut state = ChunkState::default();
        let mut out = Vec::new();
        feed_chunked(
            &mut state,
            b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n",
            &mut out,
        );
        assert_eq!(String::from_utf8(out).unwrap(), "hello world");
        assert!(state.done);
    }

    #[test]
    fn chunked_split_across_reads() {
        let mut state = ChunkState::default();
        let mut out = Vec::new();
        feed_chunked(&mut state, b"5\r\nhe", &mut out);
        feed_chunked(&mut state, b"llo\r\n0\r\n\r\n", &mut out);
        assert_eq!(String::from_utf8(out).unwrap(), "hello");
        assert!(state.done);
    }

    #[test]
    fn non_stream_json_content() {
        let body = r#"{"choices":[{"message":{"content":"full reply"}}]}"#;
        assert_eq!(parse_non_stream_json(body).as_deref(), Some("full reply"));
    }

    #[test]
    fn transcript_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        set_test_chats_dir(Some(tmp.path().to_path_buf()));
        let mut t = Transcript::new(Burner::Mlx, "qwen".into());
        t.messages.push(ChatMessage::user("hi"));
        save_transcript(&t).unwrap();
        let loaded = load_active().unwrap().unwrap();
        assert_eq!(loaded.model, "qwen");
        assert_eq!(loaded.messages[0].content, "hi");
        set_test_chats_dir(None);
    }
}
