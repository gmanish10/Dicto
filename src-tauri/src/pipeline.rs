use crate::{
    audio::{recorder::Recorder, resample},
    config::{provider_display_name, SttProvider},
    dictionary, hotkey,
    inject::{paste::ClipboardPasteInjector, Injector},
    keychain::{self, ApiKey},
    menubar,
    state::{PipelineState, SharedState},
    transcribe::{
        groq::GroqTranscriber, local::LocalWhisper, openai::OpenAiTranscriber, Transcriber,
    },
};
use crossbeam_channel::{bounded, Sender};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Listener};

/// Minimum recording length we'll bother transcribing. Anything shorter is
/// almost certainly an accidental tap.
const MIN_RECORDING_MS: u128 = 500;

/// Hard cap on recording. Prevents runaway from stuck keys or sleep/wake glitches.
const DEFAULT_MAX_RECORDING_S: u32 = 300;

/// Commands the recorder service thread accepts.
enum RecCommand {
    Start {
        preferred: Option<String>,
        ack: Sender<Result<(), String>>,
    },
    Stop {
        result: Sender<Option<RecordedAudio>>,
    },
}

struct RecordedAudio {
    pcm: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

fn try_claim_utterance_slot(in_flight: &AtomicBool) -> bool {
    in_flight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

fn release_utterance_slot(in_flight: &AtomicBool) {
    in_flight.store(false, Ordering::Release);
}

/// Run the dedicated recorder thread. Owning the cpal `Stream` here keeps the
/// rest of the pipeline `Send` so it can live on the tokio runtime.
fn spawn_recorder_service() -> Sender<RecCommand> {
    let (tx, rx) = bounded::<RecCommand>(8);
    thread::Builder::new()
        .name("dicto-recorder".into())
        .spawn(move || {
            let mut current: Option<Recorder> = None;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    RecCommand::Start { preferred, ack } => {
                        if current.is_some() {
                            let _ = ack.send(Err("already recording".into()));
                            continue;
                        }
                        match Recorder::start(preferred.as_deref()) {
                            Ok(rec) => {
                                current = Some(rec);
                                let _ = ack.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = ack.send(Err(e.to_string()));
                            }
                        }
                    }
                    RecCommand::Stop { result } => {
                        if let Some(rec) = current.take() {
                            let sample_rate = rec.sample_rate;
                            let channels = rec.channels;
                            let pcm = rec.stop();
                            let _ = result.send(Some(RecordedAudio {
                                pcm,
                                sample_rate,
                                channels,
                            }));
                        } else {
                            let _ = result.send(None);
                        }
                    }
                }
            }
        })
        .expect("failed to spawn recorder service thread");
    tx
}

/// Spawn the long-lived coordinator task. Listens to the hotkey channel,
/// delegates audio capture to the recorder service, and dispatches per-utterance
/// transcription/polish/inject tasks.
///
/// **Idempotent**: a `compare_exchange` on `state.runtime_started` ensures
/// the recorder thread, the CGEventTap, and the coordinator loop only
/// start once per process. Subsequent calls (from `start_runtime` IPC,
/// or a re-entrant code path during dev reload) are no-ops. This matters
/// because the redesigned onboarding flow defers the first spawn until
/// the user has granted permissions inside step 2 — and the IPC fires
/// from React, which can re-mount and retry.
pub fn spawn_coordinator(app: AppHandle, state: SharedState) {
    if state
        .runtime_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        tracing::debug!("spawn_coordinator: already started, skipping");
        return;
    }
    tracing::info!("spawn_coordinator: starting runtime threads");
    spawn_coordinator_inner(app, state);
}

fn spawn_coordinator_inner(app: AppHandle, state: SharedState) {
    // Shared hotkey config that the rdev listener reads; updated when settings change.
    let hotkey_config: Arc<RwLock<Option<hotkey::ParsedHotkey>>> = Arc::new(RwLock::new(
        hotkey::listener::parse(&state.config.read().hotkey.chord),
    ));
    let paused = Arc::new(RwLock::new(state.config.read().paused));

    {
        let state_for_listen = state.clone();
        let hotkey_for_listen = hotkey_config.clone();
        let paused_for_listen = paused.clone();
        app.listen("settings:updated", move |_event| {
            let cfg = state_for_listen.config.read();
            *hotkey_for_listen.write() = hotkey::listener::parse(&cfg.hotkey.chord);
            *paused_for_listen.write() = cfg.paused;
        });
    }

    // Eagerly load the local whisper model so first transcription is fast.
    let local_whisper: Arc<RwLock<Option<Arc<LocalWhisper>>>> = Arc::new(RwLock::new(None));
    {
        let app_clone = app.clone();
        let state_clone = state.clone();
        let local_clone = local_whisper.clone();
        tauri::async_runtime::spawn(async move {
            let cfg = state_clone.config.read().clone();
            match crate::model::resolve_path(&app_clone, &cfg.model_name) {
                Ok(path) => match LocalWhisper::load(&path, &cfg.language) {
                    Ok(w) => {
                        *local_clone.write() = Some(Arc::new(w));
                        tracing::info!(model = %cfg.model_name, "loaded local whisper model");
                    }
                    Err(e) => tracing::warn!(error = %e, "failed to load local whisper model"),
                },
                Err(_) => tracing::warn!(
                    model = %cfg.model_name,
                    "local whisper model not found — local provider unavailable until downloaded"
                ),
            }
        });
    }

    // Start dedicated worker threads.
    let recorder_tx = spawn_recorder_service();

    // Start the macOS CGEventTap-based hotkey listener (our replacement for
    // rdev, which panics through extern "C" on macOS 26 keycodes it doesn't
    // recognize).
    #[cfg(target_os = "macos")]
    hotkey::mac_tap::spawn(state.hotkey_tx.clone(), hotkey_config, paused);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = hotkey_config;
        let _ = paused;
    }

    // Coordinator: keeps no `!Send` state, so it can live on tokio.
    let hotkey_rx = state.hotkey_rx.clone();
    let app_for_loop = app.clone();
    let state_for_loop = state.clone();
    let local_whisper_for_loop = local_whisper.clone();
    let recorder_for_loop = recorder_tx.clone();
    let utterance_in_flight = Arc::new(AtomicBool::new(false));

    tauri::async_runtime::spawn(async move {
        let mut recording_started_at: Option<Instant> = None;
        // App that was frontmost when the user pressed the hotkey.
        // Carried through to the inject step so the polished transcript
        // pastes back into the same window the user was typing in, even
        // if focus drifted during transcription / polish.
        let mut paste_target: Option<crate::inject::target::TargetApp> = None;

        loop {
            // Block on the hotkey channel without holding any !Send state.
            let rx = hotkey_rx.clone();
            let event = match tokio::task::spawn_blocking(move || rx.recv()).await {
                Ok(Ok(ev)) => ev,
                _ => break,
            };

            match event {
                hotkey::HotkeyEvent::Down => {
                    if state_for_loop.config.read().paused {
                        continue;
                    }
                    if recording_started_at.is_some() {
                        continue;
                    }
                    if !try_claim_utterance_slot(&utterance_in_flight) {
                        tracing::debug!("utterance already in flight; ignoring hotkey down");
                        continue;
                    }
                    let preferred = state_for_loop.config.read().microphone_name.clone();
                    let (ack_tx, ack_rx) = bounded::<Result<(), String>>(1);
                    if recorder_for_loop
                        .send(RecCommand::Start {
                            preferred,
                            ack: ack_tx,
                        })
                        .is_err()
                    {
                        tracing::error!("recorder service is gone");
                        release_utterance_slot(&utterance_in_flight);
                        break;
                    }
                    let ack = tokio::task::spawn_blocking(move || {
                        ack_rx.recv_timeout(Duration::from_secs(2))
                    })
                    .await;
                    match ack {
                        Ok(Ok(Ok(()))) => {
                            recording_started_at = Some(Instant::now());
                            // Snapshot the frontmost app NOW, before our
                            // own UI (overlay window) appears and before
                            // the user has any reason to switch contexts.
                            paste_target = crate::inject::target::capture_frontmost();
                            if let Some(t) = paste_target {
                                tracing::debug!(pid = t.pid(), "captured paste target");
                            }
                            crate::chime::play_start(state_for_loop.config.read().play_start_chime);
                            state_for_loop.set_pipeline_state(PipelineState::Recording);
                            menubar::update_state_indicator(&app_for_loop, &state_for_loop);
                            let _ = app_for_loop.emit("pipeline:recording-started", ());
                        }
                        Ok(Ok(Err(e))) => {
                            release_utterance_slot(&utterance_in_flight);
                            tracing::error!(error = %e, "failed to start recorder");
                            let _ = app_for_loop.emit("pipeline:error", format!("Recorder: {e}"));
                        }
                        _ => {
                            release_utterance_slot(&utterance_in_flight);
                            tracing::error!("recorder start ack timed out");
                        }
                    }
                }
                hotkey::HotkeyEvent::Up => {
                    let Some(started_at) = recording_started_at.take() else {
                        continue;
                    };
                    // Fire the stop chime the moment the user releases.
                    // If we waited until after the recorder Stop ack the
                    // sound would lag behind the actual key release by
                    // ~50 ms and feel disconnected.
                    crate::chime::play_stop(state_for_loop.config.read().play_stop_chime);
                    let (result_tx, result_rx) = bounded::<Option<RecordedAudio>>(1);
                    if recorder_for_loop
                        .send(RecCommand::Stop { result: result_tx })
                        .is_err()
                    {
                        release_utterance_slot(&utterance_in_flight);
                        break;
                    }
                    let audio = tokio::task::spawn_blocking(move || {
                        result_rx
                            .recv_timeout(Duration::from_secs(5))
                            .ok()
                            .flatten()
                    })
                    .await
                    .ok()
                    .flatten();

                    let Some(audio) = audio else {
                        release_utterance_slot(&utterance_in_flight);
                        state_for_loop.set_pipeline_state(PipelineState::Idle);
                        menubar::update_state_indicator(&app_for_loop, &state_for_loop);
                        continue;
                    };
                    let duration = started_at.elapsed();

                    if duration.as_millis() < MIN_RECORDING_MS {
                        tracing::debug!(ms = duration.as_millis(), "recording too short, ignored");
                        release_utterance_slot(&utterance_in_flight);
                        state_for_loop.set_pipeline_state(PipelineState::Idle);
                        menubar::update_state_indicator(&app_for_loop, &state_for_loop);
                        continue;
                    }

                    let max_s = state_for_loop
                        .config
                        .read()
                        .max_recording_seconds
                        .clamp(1, DEFAULT_MAX_RECORDING_S * 2);
                    if duration.as_secs() > max_s as u64 {
                        // SAFETY: A recording longer than `max_s` is almost
                        // always a stuck-modifier (CGEventTap missing the
                        // Fn / Cmd release event). Transcribing + pasting
                        // who-knows-how-much audio into the user's next
                        // keystroke target is a privacy + injection
                        // hazard. Discard outright and warn.
                        tracing::warn!(
                            elapsed_s = duration.as_secs(),
                            max_s,
                            "recording exceeded max duration — discarding audio (likely stuck hotkey)"
                        );
                        let msg = format!(
                            "Recording exceeded {} s and was discarded. Possible stuck hotkey \u{2014} re-press your shortcut to try again.",
                            max_s
                        );
                        let _ = app_for_loop.emit("pipeline:warning", msg.clone());
                        let _ = app_for_loop.emit("pipeline:toast", msg.clone());
                        crate::notify::notify_if_hidden(
                            &app_for_loop,
                            "Dicto \u{2014} recording discarded",
                            &msg,
                        );
                        state_for_loop.set_pipeline_state(PipelineState::Idle);
                        menubar::update_state_indicator(&app_for_loop, &state_for_loop);
                        release_utterance_slot(&utterance_in_flight);
                        continue;
                    }

                    state_for_loop.set_pipeline_state(PipelineState::Transcribing);
                    menubar::update_state_indicator(&app_for_loop, &state_for_loop);
                    let _ = app_for_loop.emit("pipeline:transcribing-started", ());

                    let app_clone = app_for_loop.clone();
                    let state_clone = state_for_loop.clone();
                    let local_whisper_clone = local_whisper_for_loop.clone();
                    let target_for_run = paste_target.take();
                    let utterance_in_flight_clone = utterance_in_flight.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(err) = run_utterance(
                            app_clone.clone(),
                            state_clone.clone(),
                            local_whisper_clone,
                            audio.pcm,
                            audio.sample_rate,
                            audio.channels,
                            duration.as_millis() as i64,
                            target_for_run,
                        )
                        .await
                        {
                            tracing::error!(error = %err, "utterance failed");
                            let _ = app_clone.emit("pipeline:error", err.to_string());
                        }
                        state_clone.set_pipeline_state(PipelineState::Idle);
                        menubar::update_state_indicator(&app_clone, &state_clone);
                        let _ = app_clone.emit("pipeline:idle", ());
                        release_utterance_slot(&utterance_in_flight_clone);
                    });
                }
            }
        }
    });
}

// 8 args is one over clippy's default threshold of 7; bundling them into a
// struct would just add ceremony around a single-call-site function. Pure
// data flow, no shared state — the `allow` is the right knob here.
#[allow(clippy::too_many_arguments)]
async fn run_utterance(
    app: AppHandle,
    state: SharedState,
    local_whisper: Arc<RwLock<Option<Arc<LocalWhisper>>>>,
    pcm: Vec<f32>,
    // Note on positional args: see the call site for the order. The
    // `paste_target` was added at the end to keep diff churn minimal.
    // It threads the frontmost app captured at hotkey-down through to
    // the inject step so Cmd+V lands in the right window even if focus
    // drifted during transcribe/polish.
    sample_rate: u32,
    channels: u16,
    duration_ms: i64,
    paste_target: Option<crate::inject::target::TargetApp>,
) -> anyhow::Result<()> {
    // Resample to 16k mono off the async runtime.
    let pcm16 =
        tokio::task::spawn_blocking(move || resample::to_16k_mono(&pcm, sample_rate, channels))
            .await??;

    // Build the whisper prompt from custom vocabulary.
    let custom_words = state.history.list_custom_words().unwrap_or_default();
    let prompt = dictionary::prompt::build(&custom_words);
    let prompt_ref: Option<&str> = if prompt.is_empty() {
        None
    } else {
        Some(prompt.as_str())
    };

    let stt_provider = state.config.read().stt_provider;
    let transcriber: Box<dyn Transcriber> = match stt_provider {
        SttProvider::Local => {
            let guard = local_whisper.read();
            let Some(w) = guard.clone() else {
                drop(guard);
                anyhow::bail!("Local Whisper model not loaded. Download it from Settings.");
            };
            drop(guard);
            Box::new(LocalWhisperHandle(w))
        }
        SttProvider::Groq => {
            let key = keychain::get(ApiKey::Groq)
                .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
            let language = state.config.read().language.clone();
            Box::new(GroqTranscriber::new(key, language))
        }
        SttProvider::OpenAi => {
            let key = keychain::get(ApiKey::Openai)
                .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
            let language = state.config.read().language.clone();
            Box::new(OpenAiTranscriber::new(key, language))
        }
    };

    let raw = transcriber.transcribe(&pcm16, prompt_ref).await?;
    let stt_name = transcriber.name().to_string();
    if raw.trim().is_empty() {
        tracing::debug!("empty transcription, skipping");
        return Ok(());
    }

    // Polish — delegate provider selection + fallback to the resolver.
    let polish_provider = state.config.read().polish_provider;
    let recent_corrections = state.history.recent_corrections(5).unwrap_or_default();
    let resolution = crate::polish::resolve(polish_provider, &state.polish_ctx.read());
    let polisher = resolution.polisher;

    if let Some(requested) = resolution.downgraded_from {
        let toast = format!(
            "{} cleanup wasn't available — used {} instead. Adjust in Settings → Cleanup.",
            provider_display_name(requested),
            provider_display_name(resolution.effective),
        );
        let _ = app.emit("pipeline:toast", toast.clone());
        crate::notify::notify_if_hidden(&app, "Dicto — cleanup fell back", &toast);
    }

    let polished = match polisher.polish(&raw, &recent_corrections).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(error = %e, "polish failed; using raw transcript");
            raw.clone()
        }
    };

    // Apply user-defined word replacements.
    let replacements = state.history.list_replacements().unwrap_or_default();
    let final_text = dictionary::apply_replacements(&polished, &replacements);

    // For the actual paste, append a trailing space or newline so the
    // user can keep dictating without manual cursor work. History keeps
    // the un-suffixed `final_text` for clean re-reads.
    let injectable = crate::inject::format_for_injection(&final_text);

    // Refocus the app the user was in when they triggered the hotkey,
    // then give macOS a beat to process the activation before we post
    // Cmd+V. Without this, long transcribe/polish times let focus drift
    // and the paste lands wherever happens to be frontmost.
    if let Some(target) = paste_target {
        let activated = crate::inject::target::activate(target);
        tracing::debug!(pid = target.pid(), activated, "refocused paste target");
        if activated {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        }
    }

    // Inject into the focused app — unless onboarding is still in
    // progress, in which case the Try-it step is showing the result
    // inside Dicto's own window and we deliberately don't want to
    // paste into whatever happens to be frontmost (typically a
    // browser the user has open from reading the docs).
    if state.config.read().onboarding_completed {
        let injector = ClipboardPasteInjector;
        match injector.inject(&injectable) {
            Ok(()) => {}
            Err(crate::inject::InjectError::SecureInputActive) => {
                let msg = "Secure input detected — text copied to clipboard.";
                let _ = app.emit("pipeline:toast", msg);
                crate::notify::notify_if_hidden(&app, "Dicto — paste blocked", msg);
            }
            Err(e) => return Err(e.into()),
        }
    } else {
        tracing::debug!("onboarding active — skipping paste, Try-it panel will show result");
    }

    let polish_name = polisher.name();
    let id = state.history.insert_transcript(
        &raw,
        &final_text,
        duration_ms,
        &stt_name,
        Some(polish_name),
    )?;
    let _ = state.history.prune_to(200);

    let _ = app.emit(
        "transcript:new",
        serde_json::json!({
            "id": id,
            "raw": raw,
            "polished": final_text,
            "stt_provider": stt_name,
            "polish_provider": polish_name,
            "duration_ms": duration_ms,
        }),
    );

    Ok(())
}

/// Wrap an `Arc<LocalWhisper>` so it satisfies the `Transcriber` trait.
struct LocalWhisperHandle(Arc<LocalWhisper>);

#[async_trait::async_trait]
impl Transcriber for LocalWhisperHandle {
    async fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        prompt: Option<&str>,
    ) -> Result<String, crate::transcribe::TranscribeError> {
        self.0.transcribe(pcm_16k_mono, prompt).await
    }
    fn name(&self) -> &'static str {
        "local"
    }
    fn requires_network(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utterance_slot_rejects_overlap_until_released() {
        let in_flight = AtomicBool::new(false);

        assert!(try_claim_utterance_slot(&in_flight));
        assert!(!try_claim_utterance_slot(&in_flight));

        release_utterance_slot(&in_flight);
        assert!(try_claim_utterance_slot(&in_flight));
    }
}
