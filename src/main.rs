use std::env;

use secrecy::SecretString;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    info!("beginning initialization...");

    match dotenvy::dotenv() {
        Ok(path) => info!(path = %path.display(), "successfully read from .env file"),
        Err(error) if error.not_found() => info!("no .env file found, continuing"),
        Err(error) => error!(%error, "failed to read from .env file"),
    }

    let token = env::var("DISCORD_TOKEN").expect("discord token should have been provided");
    let token = SecretString::new(token);

    vexillologist::run(token).await;
}
