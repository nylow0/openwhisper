use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ProcessSample {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_set_mb: Option<f64>,
}

impl ProcessSample {
    pub fn capture() -> Self {
        process_sample()
    }
}

#[derive(Debug, Serialize)]
pub struct BenchReport {
    audio_path: PathBuf,
    model: String,
    device: String,
    audio_ms: u64,
    model_setup_ms: u32,
    decode_wall_ms: u32,
    engine_processing_latency_ms: u32,
    realtime_factor: f64,
    profile_memory_mb: f64,
    process_before: ProcessSample,
    process_after: ProcessSample,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_cpu_delta_ms: Option<u64>,
}

impl BenchReport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        audio_path: &Path,
        model: &str,
        device: String,
        model_setup_ms: u32,
        audio_ms: u64,
        decode_wall_ms: u32,
        engine_processing_latency_ms: u32,
        profile_memory_mb: f64,
        process_before: ProcessSample,
        process_after: ProcessSample,
    ) -> Self {
        Self {
            audio_path: audio_path.to_path_buf(),
            model: model.to_string(),
            device,
            audio_ms,
            model_setup_ms,
            decode_wall_ms,
            engine_processing_latency_ms,
            realtime_factor: realtime_factor(audio_ms, decode_wall_ms),
            profile_memory_mb,
            process_cpu_delta_ms: cpu_delta_ms(process_before, process_after),
            process_before,
            process_after,
        }
    }
}

pub fn audio_duration_ms(path: &Path) -> Result<u64> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to read WAV duration from {}", path.display()))?;
    let sample_rate = reader.spec().sample_rate.max(1) as u64;
    Ok(reader.duration() as u64 * 1_000 / sample_rate)
}

fn realtime_factor(audio_ms: u64, decode_wall_ms: u32) -> f64 {
    if audio_ms == 0 {
        return 0.0;
    }
    decode_wall_ms as f64 / audio_ms as f64
}

fn cpu_delta_ms(before: ProcessSample, after: ProcessSample) -> Option<u64> {
    after.cpu_ms?.checked_sub(before.cpu_ms?)
}

#[cfg(windows)]
fn process_sample() -> ProcessSample {
    use std::mem::{size_of, zeroed};

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessTimes(
            process: *mut std::ffi::c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn K32GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    unsafe {
        let process = GetCurrentProcess();
        let mut creation = zeroed();
        let mut exit = zeroed();
        let mut kernel = zeroed();
        let mut user = zeroed();
        let cpu_ms = (GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user)
            != 0)
            .then(|| file_time_ms(kernel.low, kernel.high) + file_time_ms(user.low, user.high));

        let mut counters: ProcessMemoryCounters = zeroed();
        counters.cb = size_of::<ProcessMemoryCounters>() as u32;
        let working_set_mb = (K32GetProcessMemoryInfo(process, &mut counters, counters.cb) != 0)
            .then(|| counters.working_set_size as f64 / 1_048_576.0);

        ProcessSample {
            cpu_ms,
            working_set_mb,
        }
    }
}

#[cfg(windows)]
fn file_time_ms(low: u32, high: u32) -> u64 {
    (((high as u64) << 32) | low as u64) / 10_000
}

#[cfg(not(windows))]
fn process_sample() -> ProcessSample {
    ProcessSample::default()
}

#[cfg(test)]
mod tests {
    use super::{audio_duration_ms, BenchReport, ProcessSample};

    #[test]
    fn bench_report_uses_audio_duration_for_realtime_factor() {
        let report = BenchReport::new(
            std::path::Path::new("sample.wav"),
            "medium_en_q8",
            "cpu".to_string(),
            3,
            2_000,
            500,
            480,
            128.0,
            ProcessSample {
                cpu_ms: Some(10),
                working_set_mb: Some(20.0),
            },
            ProcessSample {
                cpu_ms: Some(25),
                working_set_mb: Some(22.0),
            },
        );

        assert_eq!(report.realtime_factor, 0.25);
        assert_eq!(report.process_cpu_delta_ms, Some(15));
    }

    #[test]
    fn wav_duration_uses_sample_rate_and_channel_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("two-channel.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..32_000 {
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();

        assert_eq!(audio_duration_ms(&path).unwrap(), 1_000);
    }
}
