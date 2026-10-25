//! The mixer: 16 sample voices plus a chip-style synthesizer with 8 held
//! channels and a pool of one-shot `beep` voices. Pure DSP, no device code.

use crate::decode::SoundData;
use std::collections::HashMap;

pub const SAMPLE_VOICES: usize = 16;
pub const SYNTH_CHANNELS: usize = 8;
pub const BEEP_VOICES: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Wave {
    #[default]
    Square,
    Triangle,
    Saw,
    Sine,
    Noise,
}

impl Wave {
    pub fn parse(s: &str) -> Option<Wave> {
        match s.to_ascii_lowercase().as_str() {
            "square" | "pulse" => Some(Wave::Square),
            "triangle" | "tri" => Some(Wave::Triangle),
            "saw" | "sawtooth" => Some(Wave::Saw),
            "sine" | "sin" => Some(Wave::Sine),
            "noise" => Some(Wave::Noise),
            _ => None,
        }
    }
}

/// Parameters for a synth voice (`beep` and `tone`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ToneParams {
    pub wave: Wave,
    pub freq: f32,
    /// 0..1
    pub volume: f32,
    /// Square duty cycle 0..1.
    pub duty: f32,
    /// Frequency change in Hz per second.
    pub slide: f32,
    /// Seconds.
    pub attack: f32,
    pub release: f32,
    /// -1 (left) .. 1 (right)
    pub pan: f32,
}

impl Default for ToneParams {
    fn default() -> Self {
        ToneParams { wave: Wave::Square, freq: 440.0, volume: 0.5, duty: 0.5, slide: 0.0, attack: 0.005, release: 0.02, pan: 0.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioCmd {
    LoadSound { id: u32, data: SoundData },
    UnloadSound(u32),
    Play { voice: u32, sound: u32, volume: f32, pan: f32, pitch: f32, looping: bool },
    SetVoice { voice: u32, volume: Option<f32>, pan: Option<f32>, pitch: Option<f32> },
    StopVoice(u32),
    /// One-shot synth note; `duration` in seconds (not counting release).
    Beep { params: ToneParams, duration: f32 },
    /// Start or retune a held channel (0-based). `None` releases it.
    Tone { channel: u8, params: Option<ToneParams> },
    SetMaster(f32),
    StopAll,
}

impl PartialEq for SoundData {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.samples, &other.samples) && self.rate == other.rate
    }
}

struct SampleVoice {
    id: u32,
    sound: u32,
    pos: f64,
    step: f64,
    volume: f32,
    pan: f32,
    looping: bool,
    active: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Env {
    Attack,
    Hold,
    Release,
    Off,
}

#[derive(Clone, Copy)]
struct SynthVoice {
    p: ToneParams,
    phase: f32,
    env: Env,
    level: f32,
    /// Seconds remaining in Hold for one-shots; negative = until released.
    hold_left: f32,
    lfsr: u32,
    noise_val: f32,
    noise_acc: f32,
}

impl SynthVoice {
    fn off() -> SynthVoice {
        SynthVoice { p: ToneParams::default(), phase: 0.0, env: Env::Off, level: 0.0, hold_left: -1.0, lfsr: 0xACE1, noise_val: 0.0, noise_acc: 0.0 }
    }

    fn start(&mut self, p: ToneParams, hold: f32) {
        self.p = p;
        self.env = Env::Attack;
        self.hold_left = hold;
        if self.level <= 0.0 {
            self.phase = 0.0;
        }
    }

    fn retune(&mut self, p: ToneParams) {
        // Keep phase and envelope; change timbre/pitch.
        self.p = p;
        if self.env == Env::Off || self.env == Env::Release {
            self.env = Env::Attack;
        }
    }

    fn release(&mut self) {
        if self.env != Env::Off {
            self.env = Env::Release;
        }
    }

    /// One mono sample.
    fn next(&mut self, dt: f32) -> f32 {
        if self.env == Env::Off {
            return 0.0;
        }
        // Envelope
        match self.env {
            Env::Attack => {
                let step = if self.p.attack <= 0.0 { 1.0 } else { dt / self.p.attack };
                self.level += step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.env = Env::Hold;
                }
            }
            Env::Hold => {
                if self.hold_left >= 0.0 {
                    self.hold_left -= dt;
                    if self.hold_left <= 0.0 {
                        self.env = Env::Release;
                    }
                }
            }
            Env::Release => {
                let step = if self.p.release <= 0.0 { 1.0 } else { dt / self.p.release };
                self.level -= step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.env = Env::Off;
                    return 0.0;
                }
            }
            Env::Off => {}
        }
        self.p.freq = (self.p.freq + self.p.slide * dt).clamp(1.0, 20000.0);
        let v = match self.p.wave {
            Wave::Square => {
                if self.phase < self.p.duty.clamp(0.01, 0.99) {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Saw => self.phase * 2.0 - 1.0,
            Wave::Triangle => {
                if self.phase < 0.5 {
                    self.phase * 4.0 - 1.0
                } else {
                    3.0 - self.phase * 4.0
                }
            }
            Wave::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Wave::Noise => {
                // Sample-and-hold LFSR noise clocked at the voice frequency.
                self.noise_acc += self.p.freq * dt;
                while self.noise_acc >= 1.0 {
                    self.noise_acc -= 1.0;
                    let bit = (self.lfsr ^ (self.lfsr >> 1)) & 1;
                    self.lfsr = (self.lfsr >> 1) | (bit << 14);
                    self.noise_val = if self.lfsr & 1 == 1 { 1.0 } else { -1.0 };
                }
                self.noise_val
            }
        };
        self.phase += self.p.freq * dt;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        v * self.level * self.p.volume
    }
}

pub struct Mixer {
    rate: u32,
    master: f32,
    sounds: HashMap<u32, SoundData>,
    voices: Vec<SampleVoice>,
    channels: [SynthVoice; SYNTH_CHANNELS],
    beeps: [SynthVoice; BEEP_VOICES],
}

impl Mixer {
    pub fn new(rate: u32) -> Mixer {
        Mixer {
            rate: rate.max(8000),
            master: 0.8,
            sounds: HashMap::new(),
            voices: Vec::new(),
            channels: [SynthVoice::off(); SYNTH_CHANNELS],
            beeps: [SynthVoice::off(); BEEP_VOICES],
        }
    }

    pub fn handle(&mut self, cmd: AudioCmd) {
        match cmd {
            AudioCmd::LoadSound { id, data } => {
                self.sounds.insert(id, data);
            }
            AudioCmd::UnloadSound(id) => {
                self.sounds.remove(&id);
                self.voices.retain(|v| v.sound != id);
            }
            AudioCmd::Play { voice, sound, volume, pan, pitch, looping } => {
                let Some(s) = self.sounds.get(&sound) else { return };
                let step = s.rate as f64 / self.rate as f64 * pitch.max(0.01) as f64;
                self.voices.retain(|v| v.active);
                if self.voices.len() >= SAMPLE_VOICES {
                    self.voices.remove(0);
                }
                self.voices.push(SampleVoice { id: voice, sound, pos: 0.0, step, volume: volume.clamp(0.0, 1.0), pan: pan.clamp(-1.0, 1.0), looping, active: true });
            }
            AudioCmd::SetVoice { voice, volume, pan, pitch } => {
                let rate = self.rate;
                if let Some(v) = self.voices.iter_mut().find(|v| v.id == voice && v.active) {
                    if let Some(vol) = volume {
                        v.volume = vol.clamp(0.0, 1.0);
                    }
                    if let Some(p) = pan {
                        v.pan = p.clamp(-1.0, 1.0);
                    }
                    if let Some(p) = pitch {
                        if let Some(s) = self.sounds.get(&v.sound) {
                            v.step = s.rate as f64 / rate as f64 * p.max(0.01) as f64;
                        }
                    }
                }
            }
            AudioCmd::StopVoice(voice) => {
                for v in &mut self.voices {
                    if v.id == voice {
                        v.active = false;
                    }
                }
            }
            AudioCmd::Beep { params, duration } => {
                let slot = self
                    .beeps
                    .iter()
                    .position(|b| b.env == Env::Off)
                    .or_else(|| self.beeps.iter().position(|b| b.env == Env::Release))
                    .unwrap_or(0);
                self.beeps[slot] = SynthVoice::off();
                self.beeps[slot].start(params, duration.max(0.0));
            }
            AudioCmd::Tone { channel, params } => {
                let ch = &mut self.channels[(channel as usize) % SYNTH_CHANNELS];
                match params {
                    Some(p) => {
                        if ch.env == Env::Off {
                            ch.start(p, -1.0);
                        } else {
                            ch.retune(p);
                        }
                    }
                    None => ch.release(),
                }
            }
            AudioCmd::SetMaster(v) => self.master = v.clamp(0.0, 1.0),
            AudioCmd::StopAll => {
                self.voices.clear();
                for c in &mut self.channels {
                    *c = SynthVoice::off();
                }
                for b in &mut self.beeps {
                    *b = SynthVoice::off();
                }
            }
        }
    }

    /// Is anything producing sound right now?
    pub fn is_silent(&self) -> bool {
        self.voices.iter().all(|v| !v.active)
            && self.channels.iter().all(|c| c.env == Env::Off)
            && self.beeps.iter().all(|b| b.env == Env::Off)
    }

    /// Fill `out` (interleaved stereo f32) with the next frames.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / self.rate as f32;
        for frame in out.chunks_mut(2) {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in &mut self.voices {
                if !v.active {
                    continue;
                }
                let Some(s) = self.sounds.get(&v.sound) else {
                    v.active = false;
                    continue;
                };
                let n = s.samples.len();
                if n == 0 {
                    v.active = false;
                    continue;
                }
                let i = v.pos as usize;
                if i >= n {
                    if v.looping {
                        v.pos -= n as f64;
                        continue;
                    }
                    v.active = false;
                    continue;
                }
                let frac = (v.pos - i as f64) as f32;
                let a = s.samples[i];
                let b = s.samples[if i + 1 < n { i + 1 } else if v.looping { 0 } else { i }];
                let sample = (a + (b - a) * frac) * v.volume;
                let (gl, gr) = pan_gains(v.pan);
                l += sample * gl;
                r += sample * gr;
                v.pos += v.step;
            }
            for c in self.channels.iter_mut().chain(self.beeps.iter_mut()) {
                if c.env == Env::Off {
                    continue;
                }
                let s = c.next(dt);
                let (gl, gr) = pan_gains(c.p.pan);
                l += s * gl;
                r += s * gr;
            }
            frame[0] = soft_clip(l * self.master);
            frame[1] = soft_clip(r * self.master);
        }
        self.voices.retain(|v| v.active);
    }
}

fn pan_gains(pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
    (p.cos(), p.sin())
}

fn soft_clip(x: f32) -> f32 {
    if x.abs() < 0.8 {
        x
    } else {
        x.signum() * (0.8 + (x.abs() - 0.8).tanh() * 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|x| x * x).sum::<f32>() / buf.len().max(1) as f32).sqrt()
    }

    #[test]
    fn beep_makes_sound_then_stops() {
        let mut m = Mixer::new(48000);
        let mut buf = vec![0.0; 48000 * 2];
        m.render(&mut buf);
        assert_eq!(rms(&buf), 0.0);
        m.handle(AudioCmd::Beep { params: ToneParams { freq: 440.0, ..Default::default() }, duration: 0.1 });
        m.render(&mut buf);
        let first = rms(&buf[..9600]); // first 0.1 s
        let later = rms(&buf[48000..]); // after 0.5 s
        assert!(first > 0.1, "{first}");
        assert!(later < 1e-4, "{later}");
        assert!(m.is_silent());
    }

    #[test]
    fn tone_holds_until_released() {
        let mut m = Mixer::new(44100);
        m.handle(AudioCmd::Tone { channel: 2, params: Some(ToneParams { wave: Wave::Triangle, freq: 220.0, ..Default::default() }) });
        let mut buf = vec![0.0; 44100];
        m.render(&mut buf);
        assert!(rms(&buf[20000..]) > 0.1);
        m.handle(AudioCmd::Tone { channel: 2, params: None });
        m.render(&mut buf);
        assert!(rms(&buf[20000..]) < 1e-4);
    }

    #[test]
    fn sample_playback_with_loop_and_pan() {
        let mut m = Mixer::new(10000);
        let data = SoundData { samples: Arc::new(vec![1.0; 100]), rate: 10000 };
        m.handle(AudioCmd::LoadSound { id: 1, data });
        m.handle(AudioCmd::Play { voice: 7, sound: 1, volume: 1.0, pan: -1.0, pitch: 1.0, looping: false });
        let mut buf = vec![0.0; 400];
        m.render(&mut buf);
        // Hard-left: right channel silent; 100 frames of sound then nothing.
        assert!(buf[1].abs() < 1e-5 && buf[0] > 0.5);
        assert!(buf[2 * 150].abs() < 1e-6);
        m.handle(AudioCmd::Play { voice: 8, sound: 1, volume: 0.5, pan: 0.0, pitch: 2.0, looping: true });
        m.render(&mut buf);
        assert!(buf[2 * 199] > 0.1, "looping voice keeps playing");
        m.handle(AudioCmd::StopVoice(8));
        m.render(&mut buf);
        assert!(m.is_silent());
    }

    #[test]
    fn noise_and_all_waves_are_bounded() {
        for w in [Wave::Square, Wave::Triangle, Wave::Saw, Wave::Sine, Wave::Noise] {
            let mut m = Mixer::new(22050);
            m.handle(AudioCmd::Tone { channel: 0, params: Some(ToneParams { wave: w, freq: 1000.0, volume: 1.0, ..Default::default() }) });
            let mut buf = vec![0.0; 22050];
            m.render(&mut buf);
            assert!(buf.iter().all(|x| x.abs() <= 1.0), "{w:?}");
            assert!(rms(&buf[5000..]) > 0.2, "{w:?}");
        }
    }
}
