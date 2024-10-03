use std::fmt::Write;

use serenity::{
    all::{
        Command, CommandInteraction, CommandOptionType, Interaction, Mention, ResolvedOption,
        ResolvedValue,
    },
    async_trait,
    builder::{
        CreateAllowedMentions, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use sqlx::PgPool;
use tap::Pipe;
use tracing::{debug, error, info, warn};

use crate::{
    leaderboards::{AllTime, Daily},
    persist::ScoreInsertionError,
    score::Score,
};

pub mod geogrid;
pub mod leaderboards;
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
        async fn process_command(
            command: &CommandInteraction,
            db_pool: &PgPool,
        ) -> CreateInteractionResponseMessage {
            let options = command.data.options();
            let Some(ResolvedOption {
                name: "range",
                value: ResolvedValue::String(range),
                ..
            }) = options.first()
            else {
                return CreateInteractionResponseMessage::new()
                    .content("An unexpected error occurred");
            };

            let Some(guild_id) = command.guild_id else {
                warn!("cannot continue without guild ID");
                return CreateInteractionResponseMessage::new()
                    .content("This command can only be run in a server!");
            };

            if *range == "leaderboard_today" {
                let board = geogrid::board_now();
                let today = Daily::calculate_for(db_pool, guild_id, board).await;

                let mut embed = CreateEmbed::new().title("Today's Leaderboard").field(
                    "board",
                    format!("{}", board),
                    true,
                );

                let Ok(today) = today else {
                    error!(
                        error = %today.unwrap_err(),
                        "failed to calculate daily leaderboard"
                    );
                    return CreateInteractionResponseMessage::new()
                        .content("An unexpected error occurred.");
                };

                let mut description = String::new();
                for (i, entry) in today.entries.into_iter().enumerate() {
                    let medal = match i {
                        0 => " 🥇",
                        1 => " 🥈",
                        2 => " 🥉",
                        _ => "",
                    };

                    writeln!(
                        &mut description,
                        "{}. {} ({} pts, {} correct){}",
                        i + 1,
                        Mention::User(entry.user_id),
                        entry.score,
                        entry.correct,
                        medal,
                    )
                    .expect("should be able to write into String");
                }

                embed = embed
                    .description(description)
                    .footer(CreateEmbedFooter::new(
                        "Medals may change with more submissions! Run `/leaderboard` again to see \
                         updated scores.",
                    ));

                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new())
            } else if *range == "leaderboard_all_time" {
                let board = geogrid::board_now();
                let all_time = AllTime::calculate(db_pool, guild_id, board, true, true).await;
                let Ok(all_time) = all_time else {
                    error!(error = %all_time.unwrap_err(), "failed to calculate all-time leaderboard");
                    return CreateInteractionResponseMessage::new()
                        .content("An unexpected error occurred.");
                };

                let mut embed = CreateEmbed::new()
                    .title("All-Time Leaderboard")
                    .field(format!("Includes today's board (#{})?", board), "Yes", true)
                    .field("Includes late submissions?", "Yes", true);

                let mut description = String::new();
                for (i, (user_id, medals)) in all_time.medals_listing.into_iter().enumerate() {
                    writeln!(
                        &mut description,
                        "{}. {}: {}",
                        i + 1,
                        Mention::User(user_id),
                        medals,
                    )
                    .expect("should be able to write into String");
                }

                embed = embed.description(description);

                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new())
            } else {
                CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
            }
        }

        if let Interaction::Command(command) = interaction {
            info!("received command interaction");

            let response = process_command(&command, &self.db_pool)
                .await
                .pipe(CreateInteractionResponse::Message);

            match command.create_response(&ctx.http, response).await {
                Ok(_) => info!("responded to command"),
                Err(error) => error!(%error, "failed to respond to command"),
            }
        }
    }
}
