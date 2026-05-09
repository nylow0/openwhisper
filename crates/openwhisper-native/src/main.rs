use std::io::{self, BufRead, Write};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "health.check")]
    HealthCheck,
    #[serde(rename = "health.ok")]
    HealthOk { timestamp: u64 },
    #[serde(rename = "echo")]
    Echo { text: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("OpenWhisper Native Helper starting...");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        
        match serde_json::from_str::<Message>(&line) {
            Ok(Message::HealthCheck) => {
                let response = Message::HealthOk { 
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() 
                };
                let json = serde_json::to_string(&response)?;
                writeln!(stdout_lock, "{}", json)?;
                stdout_lock.flush()?;
            }
            Ok(Message::Echo { text }) => {
                log::info!("Echo: {}", text);
                let response = Message::Echo { text };
                let json = serde_json::to_string(&response)?;
                writeln!(stdout_lock, "{}", json)?;
                stdout_lock.flush()?;
            }
            Err(e) => {
                log::error!("Failed to parse message: {}", e);
            }
        }
    }

    log::info!("OpenWhisper Native Helper shutting down...");
    Ok(())
}