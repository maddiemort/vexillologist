#![allow(async_fn_in_trait)]

use serenity::{
    all::{
        Command, CommandInteraction, CommandOptionType, CreateEmbed, CreateEmbedFooter, GuildId,
        Interaction, ResolvedOption, ResolvedValue,
    },
    async_trait,
    builder::{
        CreateAllowedMentions, CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use sqlx::PgPool;
use tap::Pipe;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    game::{
        flagle::Flagle, foodguessr::FoodGuessr, geogrid::GeoGrid, Game, InsertedScore, Score,
        ScoreInsertionError,
    },
    persist::{is_opted_out, set_opt_out},
};

pub mod game;
pub mod metric;
pub mod persist;

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
                        CommandOptionType::SubCommand,
                        "today",
                        "View the leaderboard for today",
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "game",
                            "The game to view the leaderboard for",
                        )
                        .required(true)
                        .add_string_choice("GeoGrid", "geogrid")
                        .add_string_choice("Flagle", "flagle")
                        .add_string_choice("FoodGuessr", "foodguessr"),
                    ),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "all_time",
                        "View the all-time leaderboard",
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "game",
                            "The game to view the leaderboard for",
                        )
                        .required(true)
                        .add_string_choice("GeoGrid", "geogrid")
                        .add_string_choice("Flagle", "flagle")
                        .add_string_choice("FoodGuessr", "foodguessr"),
                    )
                    .add_sub_option(CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "include_today",
                        "Include today's scores in the all-time leaderboard?",
                    ))
                    .add_sub_option(CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "include_late",
                        "Include score submissions that were entered after the day ended?",
                    )),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "board",
                        "View the leaderboard for a specific board number",
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "game",
                            "The game to view the leaderboard for",
                        )
                        .required(true)
                        .add_string_choice("GeoGrid", "geogrid")
                        .add_string_choice("Flagle", "flagle"),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Integer,
                            "board_number",
                            "The board number to view the leaderboard for",
                        )
                        .required(true),
                    ),
                ),
        )
        .await
        {
            Ok(_) => info!("created global /leaderboard command"),
            Err(error) => warn!(%error, "failed to create global /leaderboard command"),
        }

        match Command::create_global_command(
            &ctx.http,
            CreateCommand::new("opt_out")
                .description("Opt out of score tracking & leaderboards (use again to toggle)"),
        )
        .await
        {
            Ok(_) => info!("created global /opt_out command"),
            Err(error) => warn!(%error, "failed to create global /opt_out command"),
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        metrics::counter!(*metric::MESSAGES_RECEIVED).increment(1);

        let Some(guild_id) = msg.guild_id else {
            warn!("cannot continue processing message without guild ID");
            return;
        };

        match msg.content.parse::<<GeoGrid as Game>::Score>() {
            Ok(score) => {
                self.process_score::<GeoGrid>(score, ctx, msg, guild_id)
                    .await;
                return;
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a Geogrid score");
            }
        }

        match msg.content.parse::<<Flagle as Game>::Score>() {
            Ok(score) => {
                self.process_score::<Flagle>(score, ctx, msg, guild_id)
                    .await;
                return;
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a Flagle score");
            }
        }

        match msg.content.parse::<<FoodGuessr as Game>::Score>() {
            Ok(score) => {
                self.process_score::<FoodGuessr>(score, ctx, msg, guild_id)
                    .await;
                return;
            }
            Err(error) => {
                debug!(reason = %error, "message isn't a FoodGuessr score");
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let response = match (command.guild_id, command.data.name.as_str()) {
                (None, _) => {
                    warn!("cannot continue processing interaction without guild ID");
                    CreateInteractionResponseMessage::new()
                        .content("This command can only be run in a server!")
                }
                (Some(guild_id), "leaderboard") => {
                    self.process_leaderboard(&command, guild_id).await
                }
                (Some(guild_id), "opt_out") => self.toggle_optout(&command, guild_id).await,
                _ => CreateInteractionResponseMessage::new().content("Unrecognised command"),
            }
            .pipe(CreateInteractionResponse::Message);

            match command.create_response(&ctx.http, response).await {
                Ok(_) => info!("responded to command"),
                Err(error) => error!(%error, "failed to respond to command"),
            }
        }
    }
}

impl Bot {
    #[instrument(skip_all, fields(game = %G::description(), %guild_id, %user_id = msg.author.id))]
    async fn process_score<G>(&self, score: G::Score, ctx: Context, msg: Message, guild_id: GuildId)
    where
        G: Game,
    {
        info!(?score, "processing score");
        metrics::counter!(
            *metric::SCORES_RECEIVED,
            "game" => G::description(),
        )
        .increment(1);

        match is_opted_out(&self.db_pool, guild_id, &msg.author).await {
            Ok(false) => info!("user has not opted out in this guild, proceeding to insert score"),
            Ok(true) => {
                info!("user has opted out in this guild, ignoring score");
                return;
            }
            Err(error) => {
                error!(%error, "failed to determine user's opt-out status, ignoring score");
                return;
            }
        }

        match score.insert(&self.db_pool, guild_id, &msg.author).await {
            Ok(inserted_score) => {
                metrics::counter!(
                    *metric::SCORE_REACTIONS,
                    "game" => G::description(),
                    "reaction" => "new",
                )
                .increment(1);

                match msg.react(&ctx.http, '✅').await {
                    Ok(_) => info!(reaction = %'✅', "reacted to new score"),
                    Err(error) => {
                        error!(%error, reaction = %'✅', "failed to react to new score");
                        metrics::counter!(
                            *metric::SCORE_REACTIONS_FAILED,
                            "game" => G::description(),
                            "reaction" => "new",
                        )
                        .increment(1);
                    }
                }

                if inserted_score.is_perfect().unwrap_or_default() {
                    metrics::counter!(
                        *metric::SCORE_REACTIONS,
                        "game" => G::description(),
                        "reaction" => "perfect",
                    )
                    .increment(1);

                    match msg.react(&ctx.http, '👑').await {
                        Ok(_) => info!(reaction = %'✨', "reacted to perfect score"),
                        Err(error) => {
                            error!(
                                %error,
                                reaction = %'👑',
                                "failed to react to perfect score"
                            );
                            metrics::counter!(
                                *metric::SCORE_REACTIONS_FAILED,
                                "game" => G::description(),
                                "reaction" => "perfect",
                            )
                            .increment(1);
                        }
                    }
                }

                if inserted_score.is_best_so_far() && inserted_score.is_on_time() {
                    metrics::counter!(
                        *metric::SCORE_REACTIONS,
                        "game" => G::description(),
                        "reaction" => "best",
                    )
                    .increment(1);

                    match msg.react(&ctx.http, '✨').await {
                        Ok(_) => info!(reaction = %'✨', "reacted to today's best score"),
                        Err(error) => {
                            error!(
                                %error,
                                reaction = %'✨',
                                "failed to react to today's best score"
                            );
                            metrics::counter!(
                                *metric::SCORE_REACTIONS_FAILED,
                                "game" => G::description(),
                                "reaction" => "best",
                            )
                            .increment(1);
                        }
                    }
                }

                metrics::counter!(
                    *metric::SCORES_INSERTED,
                    "game" => G::description(),
                    "perfect" => inserted_score.is_perfect().unwrap_or_default().to_string(),
                    "best" => inserted_score.is_best_so_far().to_string(),
                    "on_time" => inserted_score.is_on_time().to_string(),
                )
                .increment(1);
            }
            Err(ScoreInsertionError::Duplicate) => {
                metrics::counter!(
                    *metric::DUPLICATE_SCORES,
                    "game" => G::description(),
                )
                .increment(1);
                metrics::counter!(
                    *metric::SCORE_REACTIONS,
                    "game" => G::description(),
                    "reaction" => "duplicate",
                )
                .increment(1);

                match msg.react(&ctx.http, '🗞').await {
                    Ok(_) => info!(reaction = %'🗞', "reacted to duplicate score"),
                    Err(error) => {
                        error!(%error, reaction = %'🗞', "failed to react to duplicate score");
                        metrics::counter!(
                            *metric::SCORE_REACTIONS_FAILED,
                            "game" => G::description(),
                            "reaction" => "duplicate",
                        )
                        .increment(1);
                    }
                }
            }
            Err(error) => {
                error!(%error, "failed to insert score");
                metrics::counter!(
                    *metric::SCORE_INSERTIONS_FAILED,
                    "game" => G::description(),
                )
                .increment(1);

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

    async fn process_leaderboard(
        &self,
        command: &CommandInteraction,
        guild_id: GuildId,
    ) -> CreateInteractionResponseMessage {
        info!(%guild_id, "received command interaction");

        let options = command.data.options();
        let Some(ResolvedOption {
            name,
            value: ResolvedValue::SubCommand(options),
            ..
        }) = options.first()
        else {
            return CreateInteractionResponseMessage::new().content("An unexpected error occurred");
        };

        let Some(game) = options.iter().find_map(|opt| {
            if let ResolvedOption {
                name: "game",
                value: ResolvedValue::String(value),
                ..
            } = opt
            {
                Some(*value)
            } else {
                None
            }
        }) else {
            warn!("cannot respond to command without a value for the game parameter");
            return CreateInteractionResponseMessage::new()
                .content("You must specify a game in order to view the leaderboard!");
        };

        if *name == "today" {
            let embed = match game {
                "geogrid" => GeoGrid::daily_leaderboard(&self.db_pool, guild_id)
                    .await
                    .map(Into::into),
                "flagle" => Flagle::daily_leaderboard(&self.db_pool, guild_id)
                    .await
                    .map(Into::into),
                "foodguessr" => FoodGuessr::daily_leaderboard(&self.db_pool, guild_id)
                    .await
                    .map(Into::into),
                _ => {
                    return CreateInteractionResponseMessage::new()
                        .content(format!("Unknown game \"{}\"!", game))
                }
            };

            match embed {
                Ok(embed) => CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new()),
                Err(error) => {
                    error!(%error, "failed to calculate daily leaderboard");
                    CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
                }
            }
        } else if *name == "all_time" {
            let include_today = options
                .iter()
                .find_map(|opt| {
                    if let ResolvedOption {
                        name: "include_today",
                        value: ResolvedValue::Boolean(value),
                        ..
                    } = opt
                    {
                        Some(*value)
                    } else {
                        None
                    }
                })
                .unwrap_or(true);

            let include_late = options
                .iter()
                .find_map(|opt| {
                    if let ResolvedOption {
                        name: "include_late",
                        value: ResolvedValue::Boolean(value),
                        ..
                    } = opt
                    {
                        Some(*value)
                    } else {
                        None
                    }
                })
                .unwrap_or(false);

            let embed = match game {
                "geogrid" => GeoGrid::all_time_leaderboard(
                    &self.db_pool,
                    guild_id,
                    include_today,
                    include_late,
                )
                .await
                .map(Into::into),
                "flagle" => Flagle::all_time_leaderboard(
                    &self.db_pool,
                    guild_id,
                    include_today,
                    include_late,
                )
                .await
                .map(Into::into),
                "foodguessr" => FoodGuessr::all_time_leaderboard(
                    &self.db_pool,
                    guild_id,
                    include_today,
                    include_late,
                )
                .await
                .map(Into::into),
                _ => {
                    return CreateInteractionResponseMessage::new()
                        .content(format!("Unknown game \"{}\"!", game))
                }
            };

            match embed {
                Ok(embed) => CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new()),
                Err(error) => {
                    error!(%error, "failed to calculate all-time leaderboard");
                    CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
                }
            }
        } else if *name == "board" {
            let Some(board) = options.iter().find_map(|opt| {
                if let ResolvedOption {
                    name: "board_number",
                    value: ResolvedValue::Integer(value),
                    ..
                } = opt
                {
                    Some(*value)
                } else {
                    None
                }
            }) else {
                warn!("cannot respond to command without a value for the board number");
                return CreateInteractionResponseMessage::new().content(
                    "You must specify a board number in order to view the leaderboard for a board!",
                );
            };

            let embed = match game {
                "geogrid" => GeoGrid::board_leaderboard(&self.db_pool, guild_id, board as usize)
                    .await
                    .map(Into::into),
                "flagle" => Flagle::board_leaderboard(&self.db_pool, guild_id, board as usize)
                    .await
                    .map(Into::into),
                _ => {
                    return CreateInteractionResponseMessage::new()
                        .content(format!("Unknown game \"{}\"!", game))
                }
            };

            match embed {
                Ok(embed) => CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .allowed_mentions(CreateAllowedMentions::new()),
                Err(error) => {
                    error!(%error, "failed to calculate specific board leaderboard");
                    CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
                }
            }
        } else {
            CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
        }
    }

    #[instrument(skip_all, fields(user_id = %command.user.id, %guild_id))]
    async fn toggle_optout(
        &self,
        command: &CommandInteraction,
        guild_id: GuildId,
    ) -> CreateInteractionResponseMessage {
        info!("toggling opt-out status");

        let is_opted_out = match is_opted_out(&self.db_pool, guild_id, &command.user).await {
            Ok(is_opted_out) => is_opted_out,
            Err(error) => {
                error!(%error, "failed to check opt-out status of user");
                return CreateInteractionResponseMessage::new()
                    .content("An unexpected error occurred");
            }
        };

        let mut txn = match self.db_pool.begin().await {
            Ok(txn) => txn,
            Err(error) => {
                error!(%error, "failed to begin database transaction");
                return CreateInteractionResponseMessage::new()
                    .content("An unexpected error occurred");
            }
        };

        if let Err(error) = set_opt_out(&mut *txn, guild_id, &command.user, !is_opted_out).await {
            error!(%error, "failed to set opt-out status");
            return CreateInteractionResponseMessage::new().content("An unexpected error occurred");
        };

        if let Err(error) = txn.commit().await {
            error!(%error, "failed to commit database transaction");
            return CreateInteractionResponseMessage::new().content("An unexpected error occurred");
        }

        let now_opted_out = !is_opted_out;
        info!(opted_out = %now_opted_out, "user's opt-out status in this guild has been set");

        let mut embed = CreateEmbed::new().footer(CreateEmbedFooter::new(
            "Run the `/opt_out` command again to toggle.",
        ));

        embed = if now_opted_out {
            embed.title("Opted Out").description(
                "You have opted out of participation in score tracking and leaderboards.",
            )
        } else {
            embed.title("Opted In").description(
                "You have opted back in to participation in score tracking and leaderboards. Any \
                 previously recorded scores have been reinstated.",
            )
        };

        CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true)
    }
}
