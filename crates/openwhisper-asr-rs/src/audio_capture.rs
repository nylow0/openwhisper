use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;

pub const WORKER_SAMPLE_RATE_HZ: u32 = 16_000;
const STREAM_CHUNK_QUEUE_CAPACITY: usize = 8;

pub type AudioChunkReceiver = Receiver<Vec<f32>>;
type WorkerWavWriter = hound::WavWriter<std::io::BufWriter<std::fs::File>>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptureDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Vec<CaptureDeviceInfo> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok())
        .unwrap_or_default();

    let Ok(devices) = host.input_devices() else {
        return vec![];
    };

    devices
        .enumerate()
        .map(|(index, device)| {
            let name = device
                .name()
                .unwrap_or_else(|_| format!("Unknown input device #{index}"));
            CaptureDeviceInfo {
                id: format!("input-{index}"),
                is_default: !default_name.is_empty() && name == default_name,
                name,
            }
        })
        .collect()
}

pub fn record_default_input_to_wav(path: &Path, seconds: u64) -> Result<()> {
    let session = start_default_recording_session(path.to_path_buf())?;
    std::thread::sleep(Duration::from_secs(seconds));
    session.stop()?;
    Ok(())
}

pub struct RecordingSession {
    path: PathBuf,
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WorkerWavWriter>>>,
    stream_samples: Option<AudioChunkReceiver>,
}

impl RecordingSession {
    pub fn stop(self) -> Result<PathBuf> {
        drop(self.stream);
        let writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("captured audio writer lock was poisoned"))?
            .take()
            .ok_or_else(|| anyhow::anyhow!("captured audio writer was already closed"))?;
        writer.finalize()?;
        Ok(self.path)
    }

    pub fn cancel(self) {
        drop(self.stream);
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.take();
        }
        let _ = std::fs::remove_file(self.path);
    }

    pub fn take_stream_samples(&mut self) -> Option<AudioChunkReceiver> {
        self.stream_samples.take()
    }
}

pub fn start_default_recording_session(path: PathBuf) -> Result<RecordingSession> {
    start_recording_session(path, false)
}

pub fn start_default_streaming_recording_session(path: PathBuf) -> Result<RecordingSession> {
    start_recording_session(path, true)
}

fn start_recording_session(path: PathBuf, stream_chunks: bool) -> Result<RecordingSession> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("no default input device is available")?;
    let config = device
        .default_input_config()
        .context("failed to read default input device config")?;
    let sample_rate_hz = config.sample_rate().0;
    let channels = config.channels();
    let writer = Arc::new(Mutex::new(Some(create_worker_wav_writer(&path)?)));
    let captured_writer = Arc::clone(&writer);
    let (stream_tx, stream_samples) = if stream_chunks {
        let (tx, rx) = sync_channel(STREAM_CHUNK_QUEUE_CAPACITY);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    let error_callback = |error| log::error!("input stream error: {error}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[f32], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    f32_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::F64 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[f64], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    f64_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i8], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    i8_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i16], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    i16_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i32], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    i32_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::I64 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i64], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    i64_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::U8 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u8], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    u8_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u16], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    u16_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::U32 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u32], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    u32_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::U64 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u64], _| {
                push_mono_samples(
                    &captured_writer,
                    stream_tx.as_ref(),
                    data,
                    channels,
                    sample_rate_hz,
                    u64_sample_to_f32,
                )
            },
            error_callback,
            None,
        ),
        sample_format => anyhow::bail!("unsupported input sample format: {sample_format:?}"),
    }
    .context("failed to build input stream")?;

    stream.play().context("failed to start input stream")?;

    Ok(RecordingSession {
        path,
        stream,
        writer,
        stream_samples,
    })
}

pub fn mixdown_to_mono_f32(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let channel_count = channels.max(1) as usize;
    interleaved
        .chunks(channel_count)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

#[allow(dead_code)]
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| *sample as f32 / i16::MAX as f32)
        .collect()
}

#[allow(dead_code)]
pub fn u16_to_f32(samples: &[u16]) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
        .collect()
}

pub fn resample_linear(samples: &[f32], source_rate_hz: u32, target_rate_hz: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate_hz == target_rate_hz {
        return samples.to_vec();
    }

    let output_len = samples.len() * target_rate_hz as usize / source_rate_hz as usize;
    (0..output_len)
        .map(|index| {
            let source_position = index as f32 * source_rate_hz as f32 / target_rate_hz as f32;
            let left_index = source_position.floor() as usize;
            let right_index = (left_index + 1).min(samples.len() - 1);
            let fraction = source_position - left_index as f32;
            samples[left_index] * (1.0 - fraction) + samples[right_index] * fraction
        })
        .collect()
}

pub fn write_debug_wav(path: &Path, samples: &[f32], sample_rate_hz: u32) -> Result<()> {
    let mut writer = create_wav_writer(path, sample_rate_hz)?;
    write_wav_samples(&mut writer, samples)?;
    writer.finalize()?;
    Ok(())
}

fn create_worker_wav_writer(path: &Path) -> Result<WorkerWavWriter> {
    create_wav_writer(path, WORKER_SAMPLE_RATE_HZ)
}

fn create_wav_writer(path: &Path, sample_rate_hz: u32) -> Result<WorkerWavWriter> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate_hz,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create debug wav at {}", path.display()))
}

fn write_wav_samples(writer: &mut WorkerWavWriter, samples: &[f32]) -> Result<()> {
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        writer.write_sample((clamped * i16::MAX as f32) as i16)?;
    }
    Ok(())
}

fn push_mono_samples<T>(
    writer: &Arc<Mutex<Option<WorkerWavWriter>>>,
    stream_tx: Option<&SyncSender<Vec<f32>>>,
    data: &[T],
    channels: u16,
    source_rate_hz: u32,
    convert: fn(T) -> f32,
) where
    T: Copy,
{
    let channel_count = channels.max(1) as usize;
    let mono = data
        .chunks(channel_count)
        .map(|frame| frame.iter().map(|sample| convert(*sample)).sum::<f32>() / frame.len() as f32)
        .collect::<Vec<_>>();

    let worker_chunk = resample_linear(&mono, source_rate_hz, WORKER_SAMPLE_RATE_HZ);
    let Ok(mut writer) = writer.lock() else {
        return;
    };
    if let Some(writer) = writer.as_mut() {
        if let Err(error) = write_wav_samples(writer, &worker_chunk) {
            log::error!("failed to append captured audio to WAV: {error}");
        }
    }
    drop(writer);

    if let Some(stream_tx) = stream_tx {
        match stream_tx.try_send(worker_chunk) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                log::debug!("dropping ASR streaming audio chunk because decoder is behind");
            }
        }
    }
}

fn f32_to_f32(sample: f32) -> f32 {
    sample
}

fn f64_to_f32(sample: f64) -> f32 {
    sample as f32
}

fn i8_sample_to_f32(sample: i8) -> f32 {
    sample as f32 / i8::MAX as f32
}

fn i16_sample_to_f32(sample: i16) -> f32 {
    sample as f32 / i16::MAX as f32
}

fn i32_sample_to_f32(sample: i32) -> f32 {
    sample as f32 / i32::MAX as f32
}

fn i64_sample_to_f32(sample: i64) -> f32 {
    sample as f32 / i64::MAX as f32
}

fn u8_sample_to_f32(sample: u8) -> f32 {
    (sample as f32 - 128.0) / 128.0
}

fn u16_sample_to_f32(sample: u16) -> f32 {
    (sample as f32 - 32768.0) / 32768.0
}

fn u32_sample_to_f32(sample: u32) -> f32 {
    (sample as f32 - 2147483648.0) / 2147483648.0
}

fn u64_sample_to_f32(sample: u64) -> f32 {
    (sample as f64 - 9223372036854775808.0) as f32 / 9223372036854775808.0_f32
}

#[cfg(test)]
mod tests {
    use super::{i16_to_f32, mixdown_to_mono_f32, resample_linear, u16_to_f32, write_debug_wav};

    #[test]
    fn stereo_mixdown_preserves_timing_while_normalizing_to_worker_mono_format() {
        let mono = mixdown_to_mono_f32(&[1.0, -1.0, 0.5, 0.25], 2);

        assert_eq!(mono, vec![0.0, 0.375]);
    }

    #[test]
    fn integer_pcm_conversion_maps_to_f32_audio_range() {
        assert_eq!(i16_to_f32(&[0]), vec![0.0]);
        assert_eq!(u16_to_f32(&[32768]), vec![0.0]);
    }

    #[test]
    fn debug_wav_is_written_for_replayable_audio_reports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debug.wav");

        write_debug_wav(&path, &[0.0, 0.5, -0.5], 16_000).unwrap();

        assert!(path.exists());
        assert!(path.metadata().unwrap().len() > 44);
    }

    #[test]
    fn resample_linear_converts_device_rate_to_worker_rate() {
        let source = vec![0.0; 48_000];

        let resampled = resample_linear(&source, 48_000, 16_000);

        assert_eq!(resampled.len(), 16_000);
    }
}
