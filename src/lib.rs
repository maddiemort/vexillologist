use indoc::indoc;
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
use sqlx::{Error, PgPool, Row};
use tracing::{debug, error, info, warn};

use crate::{
    persist::{GuildUser, User},
    score::Score,
};

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

                let user_id = msg.author.id;
                let username = msg.author.tag();

                let Some(guild_id) = msg.guild_id else {
                    warn!("cannot continue without guild ID");
                    return;
                };

                let insert_guilds = sqlx::query(indoc! {"
                    INSERT
                        INTO guilds (id)
                        VALUES ($1)
                    ON CONFLICT (id) DO NOTHING;
                "});

                match insert_guilds
                    .bind(guild_id.get() as i64)
                    .execute(&self.db_pool)
                    .await
                {
                    Ok(result) if result.rows_affected() > 0 => info!("inserted new guild"),
                    Ok(_) => info!("guild already exists in guilds table"),
                    Err(error) => error!(%error, "failed to insert guild"),
                }

                let insert_users = sqlx::query(indoc! {"
                    INSERT
                        INTO users (id, username)
                        VALUES ($1, $2)
                    ON CONFLICT (id) DO UPDATE
                        SET username = EXCLUDED.username;
                "});

                match insert_users
                    .bind(user_id.get() as i64)
                    .bind(username)
                    .execute(&self.db_pool)
                    .await
                {
                    Ok(_) => info!("inserted new user or updated existing"),
                    Err(error) => error!(%error, "failed to insert user"),
                }

                let insert_guild_users = sqlx::query(indoc! {"
                    INSERT
                        INTO guild_users (guild_id, user_id)
                        VALUES ($1, $2)
                    ON CONFLICT DO NOTHING;
                "});

                match insert_guild_users
                    .bind(guild_id.get() as i64)
                    .bind(user_id.get() as i64)
                    .execute(&self.db_pool)
                    .await
                {
                    Ok(result) if result.rows_affected() > 0 => info!("inserted new guild user"),
                    Ok(_) => info!("guild user already exists in guild_users table"),
                    Err(error) => error!(%error, "failed to insert guild user"),
                }

                let insert_score = sqlx::query(indoc! {"
                    INSERT
                        INTO scores (guild_id, user_id, correct, board, score, rank, players)
                        VALUES ($1, $2, $3, $4, $5, $6, $7);
                "});

                let accepted = match insert_score
                    .bind(guild_id.get() as i64)
                    .bind(user_id.get() as i64)
                    .bind(score.correct)
                    .bind(score.board)
                    .bind(score.score)
                    .bind(score.rank)
                    .bind(score.players)
                    .execute(&self.db_pool)
                    .await
                {
                    Ok(_) => {
                        info!("inserted new score");
                        match msg.react(&ctx.http, '✅').await {
                            Ok(_) => info!(reaction = %'✅', "reacted to new score"),
                            Err(error) => {
                                error!(%error, reaction = %'✅', "failed to react to new score")
                            }
                        }
                        true
                    }
                    Err(Error::Database(db_err)) if db_err.is_unique_violation() => {
                        info!("score was a duplicate");
                        match msg.react(&ctx.http, '🗞').await {
                            Ok(_) => info!(reaction = %'🗞', "reacted to duplicate score"),
                            Err(error) => {
                                error!(%error, reaction = %'🗞', "failed to react to duplicate score")
                            }
                        }
                        false
                    }
                    Err(error) => {
                        error!(%error, "failed to insert score");
                        false
                    }
                };

                if accepted {
                    let get_best_score = sqlx::query(indoc! {"
                        SELECT user_id FROM scores
                        WHERE
                            guild_id = $1 AND
                            board = $2
                        ORDER BY score ASC
                        LIMIT 1;
                    "});

                    match get_best_score
                        .bind(guild_id.get() as i64)
                        .bind(score.board)
                        .fetch_one(&self.db_pool)
                        .await
                        .and_then(|row| row.try_get::<i64, _>(0))
                    {
                        Ok(best_user_id) => {
                            info!(
                                %best_user_id,
                                board = %score.board,
                                "got best score for this board"
                            );

                            if best_user_id == user_id.get() as i64 {
                                match msg.react(&ctx.http, '✨').await {
                                    Ok(_) => info!(reaction = %'✨', "reacted to best score"),
                                    Err(error) => {
                                        error!(%error, reaction = %'✨', "failed to react to best score")
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            error!(
                                %error,
                                board = %score.board,
                                "failed to get current best score for this board"
                            )
                        }
                    }
                }

                #[cfg(debug_assertions)]
                match sqlx::query_as::<_, User>("SELECT id, username FROM users")
                    .fetch_all(&self.db_pool)
                    .await
                {
                    Ok(users) => {
                        for user in users {
                            debug!(?user, "user");
                        }
                    }
                    Err(error) => {
                        error!(%error, "failed to get users");
                    }
                }

                #[cfg(debug_assertions)]
                match sqlx::query_as::<_, GuildUser>("SELECT guild_id, user_id FROM guild_users")
                    .fetch_all(&self.db_pool)
                    .await
                {
                    Ok(guild_users) => {
                        for guild_user in guild_users {
                            debug!(?guild_user, "guild user");
                        }
                    }
                    Err(error) => {
                        error!(%error, "failed to get guild users");
                    }
                }

                #[cfg(debug_assertions)]
                match sqlx::query_as::<_, Score>(
                    "SELECT correct, board, score, rank, players FROM scores",
                )
                .fetch_all(&self.db_pool)
                .await
                {
                    Ok(scores) => {
                        for score in scores {
                            debug!(?score, "score");
                        }
                    }
                    Err(error) => {
                        error!(%error, "failed to get scores");
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
