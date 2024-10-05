#![allow(async_fn_in_trait)]

use serenity::{
    all::{
        Command, CommandInteraction, CommandOptionType, GuildId, Interaction, ResolvedOption,
        ResolvedValue,
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

use crate::game::{
    flagle::Flagle, foodguessr::FoodGuessr, geogrid::GeoGrid, Game, InsertedScore, Score,
    ScoreInsertionError,
};

pub mod game;
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
                ),
        )
        .await
        {
            Ok(_) => info!("created global /leaderboard command"),
            Err(error) => warn!(%error, "failed to create global /leaderboard command"),
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
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
        async fn process_command(
            command: &CommandInteraction,
            db_pool: &PgPool,
        ) -> CreateInteractionResponseMessage {
            let Some(guild_id) = command.guild_id else {
                warn!("cannot continue processing interaction without guild ID");
                return CreateInteractionResponseMessage::new()
                    .content("This command can only be run in a server!");
            };

            info!(%guild_id, "received command interaction");

            let options = command.data.options();
            let Some(ResolvedOption {
                name,
                value: ResolvedValue::SubCommand(options),
                ..
            }) = options.first()
            else {
                return CreateInteractionResponseMessage::new()
                    .content("An unexpected error occurred");
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
                    "geogrid" => GeoGrid::daily_leaderboard(db_pool, guild_id)
                        .await
                        .map(Into::into),
                    "flagle" => Flagle::daily_leaderboard(db_pool, guild_id)
                        .await
                        .map(Into::into),
                    "foodguessr" => FoodGuessr::daily_leaderboard(db_pool, guild_id)
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
                        CreateInteractionResponseMessage::new()
                            .content("An unexpected error occurred.")
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
                        db_pool,
                        guild_id,
                        include_today,
                        include_late,
                    )
                    .await
                    .map(Into::into),
                    "flagle" => {
                        Flagle::all_time_leaderboard(db_pool, guild_id, include_today, include_late)
                            .await
                            .map(Into::into)
                    }
                    "foodguessr" => FoodGuessr::all_time_leaderboard(
                        db_pool,
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
                        CreateInteractionResponseMessage::new()
                            .content("An unexpected error occurred.")
                    }
                }
            } else {
                CreateInteractionResponseMessage::new().content("An unexpected error occurred.")
            }
        }

        if let Interaction::Command(command) = interaction {
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

impl Bot {
    #[instrument(skip_all, fields(game = %G::description(), %guild_id))]
    async fn process_score<G>(&self, score: G::Score, ctx: Context, msg: Message, guild_id: GuildId)
    where
        G: Game,
    {
        info!(?score, "processing score");

        match score.insert(&self.db_pool, guild_id, &msg.author).await {
            Ok(inserted_score) => {
                match msg.react(&ctx.http, '✅').await {
                    Ok(_) => info!(reaction = %'✅', "reacted to new score"),
                    Err(error) => {
                        error!(%error, reaction = %'✅', "failed to react to new score")
                    }
                }

                if inserted_score.is_best_so_far() && inserted_score.is_on_time() {
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
            Err(ScoreInsertionError::Duplicate) => match msg.react(&ctx.http, '🗞').await {
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
}
