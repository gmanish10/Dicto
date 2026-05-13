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
        let inner = Arc::new(Mutex::new(Inner {
            samples: Vec::with_capacity(sample_rate as usize * 4),
        }));
        let inner_for_cb = inner.clone();

        let err_fn = |err| tracing::error!(?err, "audio stream error");

        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    inner_for_cb.lock().samples.extend_from_slice(data);
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let mut guard = inner_for_cb.lock();
                    guard.samples.reserve(data.len());
                    for &s in data {
                        guard.samples.push(s as f32 / i16::MAX as f32);
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    let mut guard = inner_for_cb.lock();
                    guard.samples.reserve(data.len());
                    for &s in data {
                        guard.samples.push((s as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    }
                },
                err_fn,
                None,
            ),
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
