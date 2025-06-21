use std::{env, str::FromStr, time::Duration};

use serenity::{gateway::ActivityData, prelude::*};
use sqlx::PgPool;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use url::Url;
use vexillologist::Bot;

#[derive(Copy, Clone, Debug)]
enum Environment {
    Development,
    Production,
}

impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Ok(Environment::Development),
            "prod" | "production" => Ok(Environment::Production),
            _ => Err(format!("`{}` is not a valid environment name", s)),
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Production => write!(f, "production"),
        }
    }
}

#[tokio::main]
async fn main() {
    match dotenvy::dotenv() {
        Ok(_) => (),
        Err(error) if error.not_found() => (),
        Err(error) => panic!("failed to read from .env file: {}", error),
    }

    let loki_layer = env::var("LOKI_URL")
        .ok()
        .map(|raw_url| {
            let environment = env::var("ENVIRONMENT")
                .expect("environment name should have been provided")
                .parse::<Environment>()
                .expect("environment name should have been a recognised one");

            let (layer, task) = tracing_loki::builder()
                .label("service", "vexillologist")
                .expect("should be able to add service label")
                .label("environment", environment.to_string())
                .expect("should be able to add environment label")
                .build_url(Url::parse(&raw_url).expect("Loki URL should be a valid URL"))
                .expect("should be able to build tracing_loki layer and task");

            tokio::spawn(task);

            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                error!(%panic_info, "process panicked");
                // Wait for the Loki task to export the logs that include this panic. This is a
                // crude solution, but it's because this panic hook isn't async, so
                // we can't cleanly request shutdown from the Loki task.
                std::thread::sleep(Duration::from_secs(10));
                default_hook(panic_info);
            }));

            layer
        })
        .or_else(|| {
            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                error!(%panic_info, "process panicked");
                default_hook(panic_info);
            }));

            None
        });

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
        .with(loki_layer)
        .init();

    info!("beginning initialization");

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
