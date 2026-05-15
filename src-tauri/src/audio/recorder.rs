use crate::commands::MicrophoneInfo;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RecorderError {
    #[error("no input device available")]
    NoInputDevice,
    #[error("preferred microphone '{0}' not found")]
    PreferredNotFound(String),
    #[error("cpal error: {0}")]
    Cpal(String),
    #[error("unsupported sample format: {0:?}")]
    UnsupportedFormat(SampleFormat),
    #[error("recorder is not running")]
    NotRunning,
}

/// Hard ceiling on how much audio we'll keep in memory while recording,
/// expressed in seconds × the device's sample rate × channels. Beyond
/// this cap we drop the oldest samples instead of growing the buffer
/// indefinitely. Sized to match the coordinator's `max_recording_seconds`
/// upper bound (`DEFAULT_MAX_RECORDING_S * 2 == 600 s`) so the coordinator
/// still has the most recent ≤600 s of audio to decide whether to keep
/// or discard.
///
/// The coordinator already rejects recordings that exceed
/// `max_recording_seconds` outright; this cap is a memory safety belt
/// for the worst case (e.g. macOS misses the modifier-release event,
/// the hotkey watchdog hasn't fired yet, and audio keeps streaming for
/// minutes). Without it, ~5 minutes of stereo 48 kHz f32 audio is ~110 MB
/// and an hour is ~1.3 GB; with it we cap at ~220 MB regardless of how
/// long the stream stays open.
const MAX_BUFFERED_SECONDS: usize = 600;

/// Captures audio from the user's microphone into an in-memory buffer.
/// Stop() returns the recorded samples in the device's native rate, interleaved if
/// multichannel — callers must resample to 16 kHz mono via `resample::to_16k_mono`.
pub struct Recorder {
    inner: Arc<Mutex<Inner>>,
    _stream: Option<Stream>,
    pub sample_rate: u32,
    pub channels: u16,
}

struct Inner {
    samples: Vec<f32>,
    /// Maximum number of interleaved samples we'll buffer. Older samples
    /// are dropped when this is exceeded.
    capacity: usize,
}

impl Inner {
    /// Append samples, dropping the oldest if we'd exceed `capacity`.
    /// Worst-case keeps the most recent `capacity` samples.
    fn push_capped(&mut self, data: &[f32]) {
        if data.is_empty() {
            return;
        }
        if data.len() >= self.capacity {
            // Single callback already overflows: keep only the tail.
            self.samples.clear();
            let start = data.len() - self.capacity;
            self.samples.extend_from_slice(&data[start..]);
            return;
        }
        let total = self.samples.len() + data.len();
        if total > self.capacity {
            let drop_n = total - self.capacity;
            self.samples.drain(..drop_n);
        }
        self.samples.extend_from_slice(data);
    }
}

impl Recorder {
    /// Starts a stream on the user's selected microphone (or default).
    pub fn start(preferred: Option<&str>) -> Result<Self, RecorderError> {
        let host = cpal::default_host();
        let device = pick_device(&host, preferred)?;
        let config = device
            .default_input_config()
            .map_err(|e| RecorderError::Cpal(e.to_string()))?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let stream_config: StreamConfig = config.config();
        let capacity = (sample_rate as usize)
            .saturating_mul(channels as usize)
            .saturating_mul(MAX_BUFFERED_SECONDS);
        let inner = Arc::new(Mutex::new(Inner {
            samples: Vec::with_capacity((sample_rate as usize).saturating_mul(4)),
            capacity,
        }));
        let inner_for_cb = inner.clone();

        let err_fn = |err| tracing::error!(?err, "audio stream error");

        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    inner_for_cb.lock().push_capped(data);
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                // `scratch` is owned by the closure and reused across
                // callbacks, so the audio thread doesn't heap-allocate a
                // fresh buffer on every block.
                let mut scratch: Vec<f32> = Vec::new();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        scratch.clear();
                        scratch.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                        inner_for_cb.lock().push_capped(&scratch);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let mut scratch: Vec<f32> = Vec::new();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        scratch.clear();
                        scratch.extend(
                            data.iter()
                                .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                        );
                        inner_for_cb.lock().push_capped(&scratch);
                    },
                    err_fn,
                    None,
                )
            }
            fmt => return Err(RecorderError::UnsupportedFormat(fmt)),
        }
        .map_err(|e| RecorderError::Cpal(e.to_string()))?;

        stream
            .play()
            .map_err(|e| RecorderError::Cpal(e.to_string()))?;

        Ok(Self {
            inner,
            _stream: Some(stream),
            sample_rate,
            channels,
        })
    }

    /// Stop recording and consume the buffer. Returns interleaved native-rate samples.
    pub fn stop(mut self) -> Vec<f32> {
        // Dropping the stream stops the OS callbacks.
        drop(self._stream.take());
        let mut guard = self.inner.lock();
        std::mem::take(&mut guard.samples)
    }

    /// Mono-mix `interleaved` samples (in channel-major order) using simple average.
    pub fn mono_mix(interleaved: &[f32], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return interleaved.to_vec();
        }
        let ch = channels as usize;
        let frames = interleaved.len() / ch;
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut sum = 0.0f32;
            for c in 0..ch {
                sum += interleaved[frame * ch + c];
            }
            mono.push(sum / ch as f32);
        }
        mono
    }
}

fn pick_device(host: &cpal::Host, preferred: Option<&str>) -> Result<Device, RecorderError> {
    if let Some(name) = preferred {
        for device in host
            .input_devices()
            .map_err(|e| RecorderError::Cpal(e.to_string()))?
        {
            if device.name().ok().as_deref() == Some(name) {
                return Ok(device);
            }
        }
        return Err(RecorderError::PreferredNotFound(name.to_string()));
    }
    host.default_input_device()
        .ok_or(RecorderError::NoInputDevice)
}

/// Enumerate input devices for the Settings page.
pub fn list_input_devices() -> Result<Vec<MicrophoneInfo>, RecorderError> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut out = Vec::new();
    for device in host
        .input_devices()
        .map_err(|e| RecorderError::Cpal(e.to_string()))?
    {
        if let Ok(name) = device.name() {
            let is_default = Some(name.as_str()) == default_name.as_deref();
            out.push(MicrophoneInfo { name, is_default });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::Inner;

    #[test]
    fn push_capped_below_capacity_appends() {
        let mut inner = Inner {
            samples: Vec::new(),
            capacity: 10,
        };
        inner.push_capped(&[1.0, 2.0, 3.0]);
        inner.push_capped(&[4.0, 5.0]);
        assert_eq!(inner.samples, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn push_capped_drops_oldest_when_exceeding() {
        let mut inner = Inner {
            samples: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            capacity: 6,
        };
        inner.push_capped(&[6.0, 7.0, 8.0]);
        // 5 + 3 = 8 samples but capacity is 6 → drop 2 oldest.
        assert_eq!(inner.samples, vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn push_capped_single_callback_larger_than_capacity_keeps_tail() {
        let mut inner = Inner {
            samples: vec![0.0, 0.0],
            capacity: 4,
        };
        inner.push_capped(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(inner.samples, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn push_capped_empty_is_noop() {
        let mut inner = Inner {
            samples: vec![1.0, 2.0],
            capacity: 4,
        };
        inner.push_capped(&[]);
        assert_eq!(inner.samples, vec![1.0, 2.0]);
    }
}
