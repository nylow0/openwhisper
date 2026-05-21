mod audio_capture;
mod buffering;
mod engine;
mod protocol;
mod vad;
mod worker;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    worker::run_stdio()
}
