mod audio_capture;
mod buffering;
mod engine;
mod protocol;
mod vad;
mod worker;

fn main() {
    if let Err(err) = worker::run_stdio() {
        eprintln!("openwhisper-asr-rs worker error: {err}");
        std::process::exit(1);
    }
}
