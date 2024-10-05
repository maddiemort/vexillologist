use std::{fmt, str::FromStr};

use serenity::all::{CreateEmbed, GuildId, User};
use sqlx::{Error as SqlxError, PgPool};
use thiserror::Error;

use crate::persist::{GuildUserInsertionError, InsertionTarget};

pub mod flagle;
pub mod geogrid;

pub trait Game {
    type Score: Score<Game = Self>;

    /// A human-readable description of this game, e.g. "Geogrid".
    fn description() -> &'static str;

    async fn daily_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateDailyError>;

    async fn all_time_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
        include_today: bool,
        include_late: bool,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateAllTimeError>;
}

#[derive(Debug, Error)]
pub enum CalculateDailyError {
    #[error("failed to extract data from row: {0}")]
    FromRow(#[source] SqlxError),

    #[error("unexpected SQLx error: {0}")]
    Unexpected(SqlxError),

    #[cfg(debug_assertions)]
    #[error("not implemented yet")]
    Todo,
}

#[derive(Debug, Error)]
pub enum CalculateAllTimeError {
    #[error("failed to extract data from row: {0}")]
    FromRow(#[source] SqlxError),

    #[error("unexpected SQLx error: {0}")]
    Unexpected(SqlxError),

    #[error("unexpectedly received out-of-bounds place value from query: {0}")]
    PlaceOutOfBounds(i64),

    #[cfg(debug_assertions)]
    #[error("not implemented yet")]
    Todo,
}

pub trait Score: FromStr + fmt::Debug {
    type Game: Game;

    async fn insert(
        self,
        db_pool: &PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<impl InsertedScore, ScoreInsertionError>;
}

#[derive(Debug, Error)]
pub enum ScoreInsertionError {
    #[error("score is a duplicate entry for its board number, user and guild")]
    Duplicate,

    #[error("failed to begin transaction: {0}")]
    BeginTxn(#[source] SqlxError),

    #[error("failed to commit transaction: {0}")]
    CommitTxn(#[source] SqlxError),

    #[error("unexpected SQLx error when inserting {target}: {error}")]
    UnexpectedSqlx {
        target: InsertionTarget,
        #[source]
        error: SqlxError,
    },

    #[cfg(debug_assertions)]
    #[error("not implemented yet")]
    Todo,

    #[error(transparent)]
    GuildUserInsertion(#[from] GuildUserInsertionError),
}

pub trait InsertedScore {
    fn is_best_so_far(&self) -> bool;
    fn is_on_time(&self) -> bool;
}
