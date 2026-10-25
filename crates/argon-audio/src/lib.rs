//! Audio for Argon: a small mixer with sample voices and a chip-style
//! synthesizer, driven by commands from the program thread, output through
//! cpal. The mixer is pure Rust with no device dependency so it is unit
//! testable; `AudioEngine` wires it to a real output stream (or to nothing
//! when there is no audio device, e.g. headless CI).

pub mod decode;
pub mod mixer;

pub use decode::{decode, SoundData};
pub use mixer::{AudioCmd, Mixer, ToneParams, Wave};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Sender};

pub struct AudioEngine {
    tx: Option<Sender<AudioCmd>>,
    _stream: Option<cpal::Stream>,
    pub sample_rate: u32,
}

impl AudioEngine {
    /// Open the default output device. Never fails: without a device the
    /// engine silently drops commands.
    pub fn new() -> AudioEngine {
        match AudioEngine::open() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("audio disabled: {e}");
                AudioEngine { tx: None, _stream: None, sample_rate: 48000 }
            }
        }
    }

    pub fn disabled() -> AudioEngine {
        AudioEngine { tx: None, _stream: None, sample_rate: 48000 }
    }

    pub fn is_enabled(&self) -> bool {
        self.tx.is_some()
    }

    fn open() -> Result<AudioEngine, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let mut mixer = Mixer::new(sample_rate);
        let mut scratch: Vec<f32> = Vec::new();
        let err_fn = |e| log::warn!("audio stream error: {e}");
        let stream_config: cpal::StreamConfig = config.clone().into();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |out: &mut [f32], _| {
                    while let Ok(cmd) = rx.try_recv() {
                        mixer.handle(cmd);
                    }
                    render_into(&mut mixer, &mut scratch, out.len() / channels, channels, |i, v| out[i] = v);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |out: &mut [i16], _| {
                    while let Ok(cmd) = rx.try_recv() {
                        mixer.handle(cmd);
                    }
                    render_into(&mut mixer, &mut scratch, out.len() / channels, channels, |i, v| {
                        out[i] = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                    });
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &stream_config,
                move |out: &mut [u16], _| {
                    while let Ok(cmd) = rx.try_recv() {
                        mixer.handle(cmd);
                    }
                    render_into(&mut mixer, &mut scratch, out.len() / channels, channels, |i, v| {
                        out[i] = ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16
                    });
                },
                err_fn,
                None,
            ),
            other => return Err(format!("unsupported sample format {other:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(AudioEngine { tx: Some(tx), _stream: Some(stream), sample_rate })
    }

    pub fn send(&self, cmd: AudioCmd) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(cmd);
        }
    }

    pub fn send_all(&self, cmds: impl IntoIterator<Item = AudioCmd>) {
        if let Some(tx) = &self.tx {
            for c in cmds {
                let _ = tx.send(c);
            }
        }
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        AudioEngine::new()
    }
}

/// Render stereo frames from the mixer and spread them over `channels`.
fn render_into(mixer: &mut Mixer, scratch: &mut Vec<f32>, frames: usize, channels: usize, mut write: impl FnMut(usize, f32)) {
    scratch.clear();
    scratch.resize(frames * 2, 0.0);
    mixer.render(scratch);
    for f in 0..frames {
        let (l, r) = (scratch[f * 2], scratch[f * 2 + 1]);
        for c in 0..channels {
            let v = match c {
                0 => l,
                1 => r,
                _ => (l + r) * 0.5,
            };
            write(f * channels + c, v);
        }
    }
}
