use std::{fmt, str::FromStr};

use serenity::all::{GuildId, User};
use sqlx::{Error as SqlxError, PgPool};
use thiserror::Error;

use crate::persist::InsertionTarget;

pub mod geogrid;

pub trait Game {
    type Score: Score;

    /// A human-readable description of this game, e.g. "Geogrid".
    fn description() -> &'static str;
}

pub trait Score: FromStr + fmt::Debug {
    type Game: Game;

    async fn insert(
        self,
        db_pool: &PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<impl InsertedScore<Game = Self::Game>, ScoreInsertionError>;
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
}

pub trait InsertedScore {
    type Game: Game;

    fn is_best_so_far(&self) -> bool;
    fn is_on_time(&self) -> bool;
}
