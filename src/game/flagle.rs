use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use indoc::indoc;
use serenity::{
    all::{CreateEmbed, UserId},
    model::prelude::{GuildId, User},
};
use sqlx::{Error as SqlxError, FromRow, Row as _};
use thiserror::Error;
use tracing::{debug, error, info};

use super::{CalculateAllTimeError, CalculateDailyError, ScoreInsertionError};
use crate::{
    game::flagle::leaderboards::{AllTime, Daily},
    persist::{insert_guild_user, GuildUserRow, InsertionTarget, UserRow},
};

pub mod leaderboards;
pub mod utils;

pub struct Flagle;

impl super::Game for Flagle {
    type Score = Score;

    fn description() -> &'static str {
        "Flagle"
    }

    async fn daily_leaderboard(
        db_pool: &sqlx::PgPool,
        guild_id: GuildId,
    ) -> Result<impl Into<CreateEmbed> + std::fmt::Debug, CalculateDailyError> {
        Daily::calculate_for(db_pool, guild_id, utils::board_now()).await
    }

    async fn all_time_leaderboard(
        db_pool: &sqlx::PgPool,
        guild_id: GuildId,
        include_today: bool,
        include_late: bool,
    ) -> Result<impl Into<CreateEmbed> + std::fmt::Debug, CalculateAllTimeError> {
        AllTime::calculate(
            db_pool,
            guild_id,
            utils::board_now(),
            include_today,
            include_late,
        )
        .await
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub score: usize,
    pub board: usize,
}

impl FromStr for Score {
    type Err = ParseScoreError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let mut lines = raw.trim().lines();

        let details = lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .strip_prefix("#Flagle #")
            .ok_or(ParseScoreError::Missing(Section::Details))?;

        let (board_str, date_guesses) = details
            .split_once(' ')
            .ok_or(ParseScoreError::Missing(Section::BoardNumber))?;

        let board = board_str
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Board))?;

        let (_date, guesses) = date_guesses
            .split_once(") ")
            .ok_or(ParseScoreError::Missing(Section::Guesses))?;

        let (guess_str, _total) = guesses
            .split_once('/')
            .ok_or(ParseScoreError::InvalidFormat(Section::Guesses))?;

        let guess = guess_str
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Guess))
            .or_else(|err| if guess_str == "X" { Ok(7) } else { Err(err) })?;

        let (first, second) = (
            lines.next().ok_or(ParseScoreError::Truncated)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
        );

        let grid_raw = first.trim().to_owned() + second.trim();
        let grid = grid_raw
            .chars()
            .filter_map(|c| match c {
                '🟩' => Some(true),
                '🟥' => Some(false),
                _ => None,
            })
            .collect::<Vec<_>>();

        if grid.is_empty() {
            return Err(ParseScoreError::Missing(Section::Grid));
        } else if grid.len() != 6 {
            return Err(ParseScoreError::InvalidFormat(Section::Grid));
        }

        let score = grid.into_iter().filter(|&v| v).count();

        // The guess number will range between 1 and 7 (X). Each guess will reduce the number of
        // green squares by 1, starting at 6 for 1 guess. If these numbers don't match, the score
        // has been tampered with (badly).
        if 7 - guess != score {
            return Err(ParseScoreError::Inconsistent);
        }

        Ok(Score { score, board })
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseScoreError {
    #[error("string is empty")]
    Empty,

    #[error("string ends prematurely")]
    Truncated,

    #[error("string does not contain a {0}")]
    Missing(Section),

    #[error("{0} was not formatted as expected")]
    InvalidFormat(Section),

    #[error("{0} is not a number")]
    NotANumber(Number),

    #[error("guess number and grid don't match")]
    Inconsistent,
}

#[derive(Copy, Clone, Debug)]
pub enum Section {
    Details,
    BoardNumber,
    Guesses,
    Grid,
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Section::Details => write!(f, "details line"),
            Section::BoardNumber => write!(f, "board number"),
            Section::Guesses => write!(f, "guesses"),
            Section::Grid => write!(f, "grid section"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Number {
    Board,
    Guess,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Board => write!(f, "board number"),
            Number::Guess => write!(f, "guess"),
        }
    }
}

impl super::Score for Score {
    type Game = Flagle;

    async fn insert(
        self,
        db_pool: &sqlx::PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<impl super::InsertedScore, ScoreInsertionError> {
        let mut txn = db_pool
            .begin()
            .await
            .map_err(ScoreInsertionError::BeginTxn)?;

        insert_guild_user(&mut txn, guild_id, user).await?;

        let score_row = ScoreRow::from_score_now(self, guild_id, user.id);

        let get_best_score = sqlx::query(indoc! {"
            SELECT score FROM flagle_scores
            WHERE
                guild_id = $1
                AND user_id != $2
                AND board = $3
                AND board = day_added
            ORDER BY score DESC
            LIMIT 1;
        "});
        let best_so_far = match get_best_score
            .bind(guild_id.get() as i64)
            .bind(user.id.get() as i64)
            .bind(score_row.board)
            .fetch_one(txn.as_mut())
            .await
            .and_then(|row| row.try_get::<i32, _>(0))
        {
            Ok(best_score) => {
                info!(
                    %best_score,
                    board = %score_row.board,
                    "got best existing score for this board"
                );

                score_row.score > best_score
            }
            Err(SqlxError::RowNotFound) => {
                info!(
                    board = %score_row.board,
                    "there are no on-time scores for this board"
                );
                true
            }
            Err(error) => {
                error!(
                    %error,
                    board = %score_row.board,
                    "failed to get current best score for this board"
                );
                true
            }
        };

        let insert_score = sqlx::query(indoc! {"
            INSERT INTO flagle_scores (
                guild_id,
                user_id,
                score,
                board,
                day_added
            )
            VALUES ($1, $2, $3, $4, $5);
        "});
        match insert_score
            .bind(score_row.guild_id)
            .bind(score_row.user_id)
            .bind(score_row.score)
            .bind(score_row.board)
            .bind(score_row.day_added)
            .execute(txn.as_mut())
            .await
        {
            Ok(_) => info!("inserted new score"),
            Err(SqlxError::Database(db_err)) if db_err.is_unique_violation() => {
                info!("score was a duplicate");
                return Err(ScoreInsertionError::Duplicate);
            }
            Err(error) => {
                error!(%error, "failed to insert score");
                return Err(ScoreInsertionError::UnexpectedSqlx {
                    target: InsertionTarget::Score,
                    error,
                });
            }
        }

        txn.commit().await.map_err(ScoreInsertionError::CommitTxn)?;

        #[cfg(debug_assertions)]
        match sqlx::query_as::<_, UserRow>("SELECT user_id FROM users")
            .fetch_all(db_pool)
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
        match sqlx::query_as::<_, GuildUserRow>("SELECT guild_id, user_id FROM guild_users")
            .fetch_all(db_pool)
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
        match sqlx::query_as::<_, ScoreRow>("SELECT * FROM flagle_scores")
            .fetch_all(db_pool)
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

        Ok(InsertedScore {
            best_so_far,
            on_time: score_row.on_time(),
        })
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct ScoreRow {
    pub guild_id: i64,
    pub user_id: i64,
    pub score: i32,
    pub board: i32,
    pub day_added: i32,
}

impl ScoreRow {
    pub fn from_score_now(score: Score, guild_id: GuildId, user_id: UserId) -> Self {
        Self::from_score_at_opt(score, guild_id, user_id, Utc::now())
            .expect("now should always be after day 1")
    }

    pub fn from_score_at_opt(
        score: Score,
        guild_id: GuildId,
        user_id: UserId,
        submitted: DateTime<Utc>,
    ) -> Option<Self> {
        let Score { board, score } = score;

        Some(ScoreRow {
            guild_id: guild_id.get() as i64,
            user_id: user_id.get() as i64,
            score: score as i32,
            board: board as i32,
            day_added: utils::board_on_date(utils::date_from_utc(submitted))? as i32,
        })
    }

    pub fn on_time(&self) -> bool {
        self.day_added == self.board
    }
}

pub struct InsertedScore {
    pub best_so_far: bool,
    pub on_time: bool,
}

impl crate::game::InsertedScore for InsertedScore {
    fn is_best_so_far(&self) -> bool {
        self.best_so_far
    }

    fn is_on_time(&self) -> bool {
        self.on_time
    }
}
