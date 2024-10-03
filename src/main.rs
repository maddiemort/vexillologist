use anyhow::Context as _;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use sqlx::PgPool;
use tracing::info;
use vexillologist::Bot;

// #[tokio::main]
// async fn main() {
//     tracing_subscriber::fmt()
//         .with_env_filter(
//             EnvFilter::builder()
//                 .with_default_directive(LevelFilter::INFO.into())
//                 .from_env_lossy(),
//         )
//         .init();
//
//     info!("beginning initialization...");
//
//     match dotenvy::dotenv() {
//         Ok(path) => info!(path = %path.display(), "successfully read from .env file"),
//         Err(error) if error.not_found() => info!("no .env file found, continuing"),
//         Err(error) => error!(%error, "failed to read from .env file"),
//     }
//
//     let token = env::var("DISCORD_TOKEN").expect("discord token should have been provided");
//     let token = SecretString::new(token);
//
//     vexillologist::run(token).await;
// }

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres] db_pool: PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    info!("running migrations...");

    sqlx::migrate!()
        .run(&db_pool)
        .await
        .expect("migrations should have succeeded");

    info!("migrations finished");

    // Get the discord token set in `Secrets.toml`
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Bot { db_pool })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
