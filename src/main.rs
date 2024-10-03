use std::env;

use serenity::{gateway::ActivityData, prelude::*};
use sqlx::PgPool;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use vexillologist::Bot;

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    let fmt_layer = fmt::layer().with_timer(fmt::time::uptime());
    #[cfg(not(debug_assertions))]
    let fmt_layer = fmt::layer();

    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(fmt_layer)
        .init();

    info!("beginning initialization");

    match dotenvy::dotenv() {
        Ok(path) => info!(path = %path.display(), "successfully read from .env file"),
        Err(error) if error.not_found() => info!("no .env file found, continuing"),
        Err(error) => error!(%error, "failed to read from .env file"),
    }

    let discord_token = env::var("DISCORD_TOKEN").expect("discord token should have been provided");
    let connection_string = env::var("CONNECTION_STRING")
        .expect("database connection string should have been provided");

    let db_pool = match PgPool::connect(&connection_string).await {
        Ok(pool) => {
            info!("connected to database");
            pool
        }
        Err(error) => {
            error!(%error, "failed to connect to database");
            return;
        }
    };

    match sqlx::migrate!().run(&db_pool).await {
        Ok(_) => {
            info!("finished running migrations");
        }
        Err(error) => {
            error!(%error, "failed to run migrations");
            return;
        }
    }

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&discord_token, intents)
        .event_handler(Bot { db_pool })
        .activity(ActivityData::custom("Watching for scores"))
        .await
        .expect("should have constructed client");

    client.start().await.unwrap();
}
