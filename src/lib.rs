use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use tracing::{debug, info};

use crate::score::Score;

pub mod score;

pub struct Bot;

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, _ctx: Context, msg: Message) {
        match msg.content.parse::<Score>() {
            Ok(score) => {
                info!(?score, "parsed score");
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a score");
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}
