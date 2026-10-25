//! WAV and OGG Vorbis decoding into mono f32 samples.

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SoundData {
    /// Mono samples in -1..1.
    pub samples: Arc<Vec<f32>>,
    pub rate: u32,
}

impl SoundData {
    pub fn duration(&self) -> f32 {
        self.samples.len() as f32 / self.rate.max(1) as f32
    }
}

pub fn decode(bytes: &[u8]) -> Result<SoundData, String> {
    if bytes.starts_with(b"RIFF") {
        decode_wav(bytes)
    } else if bytes.starts_with(b"OggS") {
        decode_ogg(bytes)
    } else {
        Err("unknown sound format (use WAV or OGG Vorbis)".to_string())
    }
}

fn to_mono(interleaved: impl Iterator<Item = f32>, channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    let mut out = Vec::new();
    let mut acc = 0.0f32;
    let mut n = 0;
    for s in interleaved {
        acc += s;
        n += 1;
        if n == channels {
            out.push(acc / channels as f32);
            acc = 0.0;
            n = 0;
        }
    }
    out
}

fn decode_wav(bytes: &[u8]) -> Result<SoundData, String> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => to_mono(reader.samples::<f32>().map(|s| s.unwrap_or(0.0)), channels),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            to_mono(reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 / max), channels)
        }
    };
    Ok(SoundData { samples: Arc::new(samples), rate: spec.sample_rate })
}

fn decode_ogg(bytes: &[u8]) -> Result<SoundData, String> {
    let mut reader = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let channels = reader.ident_hdr.audio_channels as usize;
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut out = Vec::new();
    while let Some(pkt) = reader.read_dec_packet_itl().map_err(|e| e.to_string())? {
        out.extend(to_mono(pkt.iter().map(|&s| s as f32 / 32768.0), channels));
    }
    Ok(SoundData { samples: Arc::new(out), rate })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_wav() {
        let spec = hound::WavSpec { channels: 2, sample_rate: 22050, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = hound::WavWriter::new(&mut buf, spec).unwrap();
            for i in 0..100 {
                w.write_sample::<i16>(if i % 2 == 0 { 16384 } else { -16384 }).unwrap();
                w.write_sample::<i16>(0).unwrap();
            }
            w.finalize().unwrap();
        }
        let s = decode(buf.get_ref()).unwrap();
        assert_eq!(s.rate, 22050);
        assert_eq!(s.samples.len(), 100);
        assert!((s.samples[0] - 0.25).abs() < 0.01);
        assert!(decode(b"nope").is_err());
    }
}
