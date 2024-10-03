use indoc::indoc;
use serenity::all::{GuildId, UserId};
use sqlx::{Error as SqlxError, FromRow, PgPool};
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Clone, Debug)]
pub struct Daily {
    pub entries: Vec<DailyEntry>,
}

#[derive(Clone, Debug)]
pub struct DailyEntry {
    pub user_id: UserId,
    pub username: String,
    pub correct: usize,
    pub score: f32,
}

impl From<DailyEntryRow> for DailyEntry {
    fn from(row: DailyEntryRow) -> Self {
        Self {
            user_id: UserId::new(row.user_id as u64),
            username: row.username,
            correct: row.correct as usize,
            score: row.score,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
struct DailyEntryRow {
    user_id: i64,
    username: String,
    correct: i32,
    score: f32,
}

#[derive(Debug, Error)]
pub enum CalculateDailyError {
    #[error("failed to extract data from row: {0}")]
    FromRow(#[source] SqlxError),

    #[error("unexpected SQLx error: {0}")]
    Unexpected(SqlxError),
}

impl Daily {
    pub async fn calculate_for(
        db_pool: &PgPool,
        guild_id: GuildId,
        day: usize,
    ) -> Result<Self, CalculateDailyError> {
        let get_scores = sqlx::query(indoc! {"
            SELECT
                s.user_id,
                u.username,
                s.correct,
                s.score
            FROM
                scores s
                INNER JOIN users u USING (user_id)
            WHERE
                s.guild_id = $1
                AND s.board = $2
                AND s.board = s.day_added
            ORDER BY score ASC;
        "});
        let entries = match get_scores
            .bind(guild_id.get() as i64)
            .bind(day as i32)
            .fetch_all(db_pool)
            .await
        {
            Ok(rows) => {
                info!("fetched all scores");

                rows.into_iter()
                    .map(|row| {
                        DailyEntryRow::from_row(&row)
                            .map(|row| {
                                #[cfg(debug_assertions)]
                                debug!(?row, "got leaderboard entry");
                                row.into()
                            })
                            .map_err(CalculateDailyError::FromRow)
                    })
                    .collect::<Result<Vec<_>, CalculateDailyError>>()?
            }
            Err(error) => {
                error!(%error, "failed to fetch all scores");
                return Err(CalculateDailyError::Unexpected(error));
            }
        };

        Ok(Daily { entries })
    }
}
