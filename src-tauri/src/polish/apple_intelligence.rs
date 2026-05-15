//! Apple Intelligence polish via a long-lived Swift sidecar.
//!
//! The sidecar binary `dicto-apple-polish` lives next to the main app
//! binary (Tauri bundles it through `externalBin`) and exposes
//! Apple's Foundation Models framework — the on-device LLM that powers
//! Apple Intelligence on macOS 26+. We keep one sidecar process alive
//! per app, serialize polish calls through a `tokio::Mutex`, and talk
//! to it via line-delimited JSON over stdin / stdout.
//!
//! ## Lifecycle
//!
//! - Constructed by the resolver when (a) the host is macOS 26+ and
//!   (b) the sidecar binary exists at the expected path. The sidecar
//!   itself decides whether Apple Intelligence is *enabled* — the
//!   resolver doesn't need to know.
//! - The sidecar process is spawned lazily on the first polish call.
//!   Cold spawn is ~80 ms; subsequent calls reuse the process.
//! - If the sidecar dies mid-session we discard the cached process and
//!   re-spawn on the next call. (Foundation Models can fail with a
//!   transient `modelNotReady` early in app launch and stabilize later.)
//!
//! ## Protocol
//!
//! On startup the sidecar writes one ready message to stdout:
//! ```json
//! {"ready": true, "availability": "available" | "<reason>"}
//! ```
//! For each polish call we write one request line:
//! ```json
//! {"id": "...", "system": "...", "user": "..."}
//! ```
//! and read one response line:
//! ```json
//! {"id": "...", "ok": true, "text": "..."}
//! ```
//! or:
//! ```json
//! {"id": "...", "ok": false, "error": "..."}
//! ```

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use super::{prompt, sanity_check, Correction, PolishError, Polisher};

/// Hard upper bound on a single polish call. Apple Intelligence is
/// usually sub-second; a 30 s timeout catches a wedged sidecar without
/// stranding the user mid-recording loop.
const POLISH_TIMEOUT: Duration = Duration::from_secs(30);

/// Cap on the ready handshake. The sidecar emits the ready message
/// immediately on startup — anything beyond 5 s means something is wrong.
const READY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct AppleIntelligencePolisher {
    binary_path: PathBuf,
    state: Arc<Mutex<Option<Sidecar>>>,
}

struct Sidecar {
    // Held to keep the process alive (kill_on_drop fires when this struct
    // drops). We never read `child` directly — communication happens
    // through the stdin/stdout halves stored alongside.
    #[allow(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[derive(Serialize)]
struct PolishRequest<'a> {
    id: &'a str,
    system: &'a str,
    user: &'a str,
}

#[derive(Deserialize)]
struct PolishResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct ReadyMessage {
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    availability: String,
}

impl AppleIntelligencePolisher {
    pub fn new(binary_path: PathBuf) -> Self {
        Self {
            binary_path,
            state: Arc::new(Mutex::new(None)),
        }
    }

    /// Spawn the sidecar (if not already running) and exchange one
    /// request/response. The mutex guarantees we never interleave bytes
    /// between concurrent calls.
    async fn exchange(&self, system: &str, user: &str) -> Result<String, PolishError> {
        let mut guard = self.state.lock().await;

        // (Re)spawn if dead or never started.
        if guard.is_none() {
            *guard = Some(spawn(&self.binary_path).await?);
        }

        // Borrow inner sidecar by mutable ref via a helper.
        let sidecar = guard.as_mut().expect("just populated");

        let req = PolishRequest {
            id: "polish",
            system,
            user,
        };
        let mut req_line = serde_json::to_string(&req)
            .map_err(|e| PolishError::Api(format!("encode request: {e}")))?;
        req_line.push('\n');

        let exchange_result = timeout(POLISH_TIMEOUT, async {
            sidecar
                .stdin
                .write_all(req_line.as_bytes())
                .await
                .map_err(|e| PolishError::Api(format!("write sidecar: {e}")))?;
            sidecar
                .stdin
                .flush()
                .await
                .map_err(|e| PolishError::Api(format!("flush sidecar: {e}")))?;

            let mut line = String::new();
            let n = sidecar
                .stdout
                .read_line(&mut line)
                .await
                .map_err(|e| PolishError::Api(format!("read sidecar: {e}")))?;
            if n == 0 {
                return Err(PolishError::Api("sidecar closed stdout".into()));
            }
            Ok(line)
        })
        .await;

        let response_line = match exchange_result {
            Ok(Ok(line)) => line,
            Ok(Err(e)) => {
                // Sidecar I/O broken — drop the cached process so the
                // next call re-spawns.
                *guard = None;
                return Err(e);
            }
            Err(_) => {
                *guard = None;
                return Err(PolishError::Api("apple-polish timed out".into()));
            }
        };

        let resp: PolishResponse = serde_json::from_str(response_line.trim())
            .map_err(|e| PolishError::Api(format!("decode response: {e}")))?;

        if resp.ok {
            resp.text
                .ok_or_else(|| PolishError::Api("missing text in response".into()))
        } else {
            Err(PolishError::Api(
                resp.error.unwrap_or_else(|| "sidecar error".into()),
            ))
        }
    }
}

impl Drop for AppleIntelligencePolisher {
    fn drop(&mut self) {
        // The Tokio Mutex can't be locked synchronously from Drop, so we
        // just rely on Child's own drop behavior to send SIGTERM. The
        // sidecar exits cleanly on stdin close (EOF in its read loop).
    }
}

#[async_trait]
impl Polisher for AppleIntelligencePolisher {
    async fn polish(&self, raw: &str, recent: &[Correction]) -> Result<String, PolishError> {
        // Apple Intelligence runs a small (~3 B) model; the long
        // full-system prompt costs ~700 ms in prompt-processing alone.
        // The compact variant keeps the rules that matter for on-device
        // polish quality and shaves total latency by ~50%.
        let system = prompt::build_compact_system(recent);
        let user = format!("Polish this transcript:\n\n{raw}");
        let started = std::time::Instant::now();
        let out = self.exchange(&system, &user).await?;
        let elapsed = started.elapsed().as_millis();
        let trimmed = out.trim();
        // Privacy: never log transcript content (raw or polished). The
        // dictated text is sensitive by definition — we only record
        // timing and length so dev logs / crash reports can't leak it.
        tracing::info!(
            elapsed_ms = elapsed as u64,
            raw_chars = raw.chars().count(),
            polished_chars = trimmed.chars().count(),
            "apple-polish result"
        );
        if trimmed.is_empty() {
            return Err(PolishError::OutputRejected(
                "empty output from apple-polish",
            ));
        }
        sanity_check(raw, trimmed).map_err(PolishError::OutputRejected)?;
        Ok(trimmed.to_string())
    }

    fn name(&self) -> &'static str {
        "apple_intelligence"
    }
}

async fn spawn(binary_path: &Path) -> Result<Sidecar, PolishError> {
    // Sidecar stderr inherits parent's stderr so its `logErr(...)` lines
    // (timing logs, request-decode errors) end up in the dicto binary's
    // own stderr — which is the dev console in development and the
    // process's stderr in bundled builds. We deliberately don't pipe it,
    // because a piped stream we never drain will eventually backpressure
    // and stall the sidecar.
    let mut child = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            PolishError::Api(format!(
                "spawn apple-polish ({}): {e}",
                binary_path.display()
            ))
        })?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| PolishError::Api("sidecar stdin missing".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PolishError::Api("sidecar stdout missing".into()))?;
    let mut stdout = BufReader::new(stdout);

    // Read the ready handshake.
    let mut line = String::new();
    let ready_read = timeout(READY_TIMEOUT, stdout.read_line(&mut line))
        .await
        .map_err(|_| PolishError::Api("sidecar ready handshake timed out".into()))?
        .map_err(|e| PolishError::Api(format!("read ready: {e}")))?;
    if ready_read == 0 {
        return Err(PolishError::Api("sidecar exited before ready".into()));
    }
    let ready: ReadyMessage = serde_json::from_str(line.trim())
        .map_err(|e| PolishError::Api(format!("decode ready: {e}")))?;
    if !ready.ready {
        return Err(PolishError::Api("sidecar didn't signal ready".into()));
    }
    if ready.availability != "available" {
        // Foundation Models reports unavailability — kill the sidecar
        // and let the resolver fall back to another provider.
        let _ = child.kill().await;
        return Err(PolishError::Api(format!(
            "Apple Intelligence unavailable: {}",
            ready.availability
        )));
    }

    tracing::info!(binary = %binary_path.display(), "apple-polish sidecar ready");

    Ok(Sidecar {
        child,
        stdin,
        stdout,
    })
}
