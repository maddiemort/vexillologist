use chrono::{DateTime, Utc};
use indoc::indoc;
use serenity::all::{GuildId, User, UserId};
use sqlx::{Error as SqlxError, FromRow, PgPool, Row as _};
use tracing::{debug, error, info};

use crate::{
    game::{
        geogrid::{utils, Score},
        ScoreInsertionError,
    },
    persist::{insert_guild_user, GuildUserRow, InsertionTarget, UserRow},
};

#[derive(Clone, Debug, FromRow)]
pub struct ScoreRow {
    pub guild_id: i64,
    pub user_id: i64,
    pub correct: i32,
    pub board: i32,
    pub score: f32,
    pub rank: i32,
    pub players: i32,
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
        let Score {
            correct,
            board,
            score,
            rank,
            players,
        } = score;

        Some(ScoreRow {
            guild_id: guild_id.get() as i64,
            user_id: user_id.get() as i64,
            correct: correct as i32,
            board: board as i32,
            score,
            rank: rank as i32,
            players: players as i32,
            day_added: utils::board_on_date(utils::date_from_utc(submitted))? as i32,
        })
    }

    pub fn on_time(&self) -> bool {
        self.day_added == self.board
    }
}

impl From<ScoreRow> for Score {
    fn from(score_row: ScoreRow) -> Self {
        Self {
            correct: score_row.correct as usize,
            board: score_row.board as usize,
            score: score_row.score,
            rank: score_row.rank as usize,
            players: score_row.players as usize,
        }
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

pub async fn insert_score(
    db_pool: &PgPool,
    score: Score,
    guild_id: GuildId,
    user: &User,
) -> Result<InsertedScore, ScoreInsertionError> {
    let mut txn = db_pool
        .begin()
        .await
        .map_err(ScoreInsertionError::BeginTxn)?;

    insert_guild_user(&mut txn, guild_id, user).await?;

    let score_row = ScoreRow::from_score_now(score, guild_id, user.id);

    let insert_score = sqlx::query(indoc! {"
        INSERT INTO geogrid_scores (
            guild_id,
            user_id,
            correct,
            board,
            score,
            rank,
            players,
            day_added
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8);
    "});
    match insert_score
        .bind(score_row.guild_id)
        .bind(score_row.user_id)
        .bind(score_row.correct)
        .bind(score_row.board)
        .bind(score_row.score)
        .bind(score_row.rank)
        .bind(score_row.players)
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

    let get_best_score = sqlx::query(indoc! {"
        SELECT user_id FROM geogrid_scores
        WHERE
            guild_id = $1
            AND board = $2
            AND board = day_added
        ORDER BY score ASC
        LIMIT 1;
    "});
    let best_so_far = match get_best_score
        .bind(guild_id.get() as i64)
        .bind(score_row.board)
        .fetch_one(txn.as_mut())
        .await
        .and_then(|row| row.try_get::<i64, _>(0))
    {
        Ok(best_user_id) => {
            info!(
                %best_user_id,
                board = %score_row.board,
                "got best score for this board"
            );

            best_user_id == user.id.get() as i64
        }
        Err(SqlxError::RowNotFound) => {
            info!(
                board = %score_row.board,
                "there are no on-time scores for this board"
            );
            false
        }
        Err(error) => {
            error!(
                %error,
                board = %score_row.board,
                "failed to get current best score for this board"
            );
            false
        }
    };

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
    match sqlx::query_as::<_, ScoreRow>("SELECT * FROM geogrid_scores")
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
