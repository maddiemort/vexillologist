use serenity::{
    all::{Command, CommandOptionType, Interaction, ResolvedOption, ResolvedValue},
    async_trait,
    builder::{
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::{persist::ScoreInsertionError, score::Score};

pub mod geogrid;
pub mod persist;
pub mod score;

pub struct Bot {
    pub db_pool: PgPool,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(username = %ready.user.name, "connected!");

        match Command::create_global_command(
            &ctx.http,
            CreateCommand::new("leaderboard")
                .description("View the leaderboard")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "range",
                        "The time range of the leaderboard to view",
                    )
                    .add_string_choice("Today", "leaderboard_today")
                    .add_string_choice("All Time", "leaderboard_all_time")
                    .required(true),
                ),
        )
        .await
        {
            Ok(_) => info!("created global /leaderboard command"),
            Err(error) => warn!(%error, "failed to create global /leaderboard command"),
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.parse::<Score>() {
            Ok(score) => {
                info!(?score, "parsed score");

                let Some(guild_id) = msg.guild_id else {
                    warn!("cannot continue without guild ID");
                    return;
                };

                match persist::insert_score(&self.db_pool, score, guild_id, &msg.author).await {
                    Ok(inserted_score) => {
                        match msg.react(&ctx.http, '✅').await {
                            Ok(_) => info!(reaction = %'✅', "reacted to new score"),
                            Err(error) => {
                                error!(%error, reaction = %'✅', "failed to react to new score")
                            }
                        }

                        if inserted_score.best_so_far && inserted_score.on_time {
                            match msg.react(&ctx.http, '✨').await {
                                Ok(_) => info!(reaction = %'✨', "reacted to today's best score"),
                                Err(error) => {
                                    error!(
                                        %error,
                                        reaction = %'✨',
                                        "failed to react to today's best score"
                                    )
                                }
                            }
                        }
                    }
                    Err(ScoreInsertionError::Duplicate) => match msg.react(&ctx.http, '🗞').await
                    {
                        Ok(_) => info!(reaction = %'🗞', "reacted to duplicate score"),
                        Err(error) => {
                            error!(%error, reaction = %'🗞', "failed to react to duplicate score")
                        }
                    },
                    Err(error) => {
                        error!(%error, "failed to insert score");

                        match msg
                            .reply_ping(
                                &ctx.http,
                                format!("Failed to record score ({}). Please try again!", error),
                            )
                            .await
                        {
                            Ok(_) => info!("responded to score with error message"),
                            Err(error) => {
                                error!(
                                    %error,
                                    "failed to respond with error message"
                                )
                            }
                        }
                    }
                }
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a score");
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            info!(?command, "received command interaction");

            let response = match command.data.name.as_str() {
                "leaderboard" => match command.data.options().first() {
                    Some(ResolvedOption {
                        value: ResolvedValue::String(_range),
                        name: "range",
                        ..
                    }) => CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("Coming soon!"),
                    ),
                    _ => CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("An unexpected error occurred"),
                    ),
                },
                name => CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Unrecognized command \"{}\"", name)),
                ),
            };

            match command.create_response(&ctx.http, response).await {
                Ok(_) => info!("responded to interaction"),
                Err(error) => error!(%error, "failed to respond to interaction"),
            }
        }
    }
}
