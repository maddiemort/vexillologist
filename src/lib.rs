use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use sqlx::PgPool;
use tracing::{debug, info};

use crate::score::Score;

pub mod score;

pub struct Bot {
    pub db_pool: PgPool,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.parse::<Score>() {
            Ok(score) => {
                let user_id = msg.author.id;
                let username = msg.author.tag();

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
