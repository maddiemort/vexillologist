use std::{fmt, str::FromStr};

use chrono::{Datelike, Month, NaiveDate, Utc};
use indoc::indoc;
use serenity::{
    all::{CreateEmbed, UserId},
    model::prelude::{GuildId, User},
};
use sqlx::{Error as SqlxError, FromRow, PgPool, Row as _};
use thiserror::Error;
use tracing::{debug, error, info};

use self::leaderboards::{AllTime, Daily};
use super::{CalculateAllTimeError, CalculateDailyError, ScoreInsertionError};
use crate::persist::{insert_guild_user, GuildUserRow, InsertionTarget, UserRow};

pub mod leaderboards;

pub struct FoodGuessr;

impl super::Game for FoodGuessr {
    type Score = Score;

    fn description() -> &'static str {
        "FoodGuessr"
    }

    async fn daily_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateDailyError> {
        Daily::calculate_for(db_pool, guild_id, Utc::now().naive_utc().date()).await
    }

    async fn all_time_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
        include_today: bool,
        include_late: bool,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateAllTimeError> {
        AllTime::calculate(
            db_pool,
            guild_id,
            Utc::now().naive_utc().date(),
            include_today,
            include_late,
        )
        .await
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub date: NaiveDate,
    pub score: usize,
}

impl FromStr for Score {
    type Err = ParseScoreError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let mut lines = raw.trim().lines();

        let date_str = lines
            .next()
            .ok_or(ParseScoreError::Empty)?
            .strip_prefix("FoodGuessr - ")
            .ok_or(ParseScoreError::Missing(Section::Details))?;

        let (day_str, date_remaining) = date_str
            .split_once(' ')
            .ok_or(ParseScoreError::InvalidFormat(Section::Date))?;
        let (month_str, date_remaining) = date_remaining
            .split_once(' ')
            .ok_or(ParseScoreError::InvalidFormat(Section::Date))?;
        let (year_str, _date_remaining) = date_remaining
            .split_once(' ')
            .ok_or(ParseScoreError::InvalidFormat(Section::Date))?;

        let day = day_str
            .parse::<u32>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Day))?;
        let month = month_str
            .parse::<Month>()
            .map_err(|_| ParseScoreError::InvalidMonth)?
            .number_from_month();
        let year = year_str
            .parse::<i32>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Year))?;

        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();

        let (_first, _second, _third) = (
            lines.next().ok_or(ParseScoreError::Truncated)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
        );

        let score_line = lines
            .next()
            .ok_or(ParseScoreError::Missing(Section::Score))?;

        let score_and_total = score_line
            .strip_prefix("Total score: ")
            .ok_or(ParseScoreError::InvalidFormat(Section::Score))?;

        let score_str = score_and_total
            .strip_suffix(" / 15,000")
            .ok_or(ParseScoreError::InvalidFormat(Section::Score))?;

        let score = score_str
            .replace(',', "")
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Score))?;

        Ok(Score { date, score })
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

    #[error("month string does not represent a month")]
    InvalidMonth,
}

#[derive(Copy, Clone, Debug)]
pub enum Section {
    Details,
    Date,
    Round,
    Score,
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Section::Details => write!(f, "details line"),
            Section::Date => write!(f, "date"),
            Section::Round => write!(f, "round"),
            Section::Score => write!(f, "score"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Number {
    Score,
    Year,
    Month,
    Day,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Score => write!(f, "score"),
            Number::Year => write!(f, "year"),
            Number::Month => write!(f, "month"),
            Number::Day => write!(f, "day"),
        }
    }
}

impl super::Score for Score {
    type Game = FoodGuessr;

    async fn insert(
        self,
        db_pool: &PgPool,
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
            SELECT score FROM foodguessr_scores
            WHERE
                guild_id = $1
                AND user_id != $2
                AND year = $3
                AND ordinal = $4
                AND year = year_added
                AND ordinal = ordinal_added
            ORDER BY score DESC
            LIMIT 1;
        "});
        let best_so_far = match get_best_score
            .bind(guild_id.get() as i64)
            .bind(user.id.get() as i64)
            .bind(score_row.year)
            .bind(score_row.ordinal)
            .fetch_one(txn.as_mut())
            .await
            .and_then(|row| row.try_get::<i32, _>(0))
        {
            Ok(best_score) => {
                info!(
                    %best_score,
                    year = %score_row.year,
                    ordinal = %score_row.ordinal,
                    "got best existing score for this board"
                );

                score_row.score > best_score
            }
            Err(SqlxError::RowNotFound) => {
                info!(
                    year = %score_row.year,
                    ordinal = %score_row.ordinal,
                    "there are no on-time scores for this board"
                );
                true
            }
            Err(error) => {
                error!(
                    %error,
                    year = %score_row.year,
                    ordinal = %score_row.ordinal,
                    "failed to get current best score for this board"
                );
                true
            }
        };

        let insert_score = sqlx::query(indoc! {"
            INSERT INTO foodguessr_scores (
                guild_id,
                user_id,
                score,
                year,
                ordinal,
                year_added,
                ordinal_added
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7);
        "});
        match insert_score
            .bind(score_row.guild_id)
            .bind(score_row.user_id)
            .bind(score_row.score)
            .bind(score_row.year)
            .bind(score_row.ordinal)
            .bind(score_row.year_added)
            .bind(score_row.ordinal_added)
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
        match sqlx::query_as::<_, ScoreRow>("SELECT * FROM foodguessr_scores")
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
    pub year: i32,
    pub ordinal: i32,
    pub year_added: i32,
    pub ordinal_added: i32,
}

impl ScoreRow {
    pub fn from_score_now(score: Score, guild_id: GuildId, user_id: UserId) -> Self {
        Self::from_score_at_opt(score, guild_id, user_id, Utc::now().naive_utc().date())
            .expect("now should always be after day 1")
    }

    pub fn from_score_at_opt(
        score: Score,
        guild_id: GuildId,
        user_id: UserId,
        submitted: NaiveDate,
    ) -> Option<Self> {
        let Score { date, score } = score;

        Some(ScoreRow {
            guild_id: guild_id.get() as i64,
            user_id: user_id.get() as i64,
            score: score as i32,
            year: date.year(),
            ordinal: date.ordinal() as i32,
            year_added: submitted.year(),
            ordinal_added: submitted.ordinal() as i32,
        })
    }

    pub fn on_time(&self) -> bool {
        self.year == self.year_added && self.ordinal == self.ordinal_added
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
