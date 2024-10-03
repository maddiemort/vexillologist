use std::{error::Error, sync::Arc};

use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, info, trace, warn};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;

use crate::score::Score;

pub mod score;

pub async fn run(token: SecretString) {
    // Use intents to only receive guild message events.
    let mut shard = Shard::new(
        ShardId::ONE,
        token.expose_secret().clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    );

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token.expose_secret().clone()));

    // Since we only care about new messages, make the cache only
    // cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Process each event as they come in.
    loop {
        let event = shard.next_event().await;

        let Ok(event) = event else {
            warn!(error = %event.unwrap_err(), "error receiving event");

            continue;
        };

        debug!("received event");

        // Update the cache with the event.
        cache.update(&event);

        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => match msg.content.parse::<Score>() {
            Ok(score) => {
                info!(?score, "parsed score");
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a score");
            }
        },
        // Other events here...
        _ => {}
    }

    Ok(())
}
