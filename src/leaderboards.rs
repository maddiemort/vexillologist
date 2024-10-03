use std::{collections::HashMap, fmt};

use indoc::{formatdoc, indoc};
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

impl From<DailyQueryRow> for DailyEntry {
    fn from(row: DailyQueryRow) -> Self {
        Self {
            user_id: UserId::new(row.user_id as u64),
            username: row.username,
            correct: row.correct as usize,
            score: row.score,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
struct DailyQueryRow {
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
                        DailyQueryRow::from_row(&row)
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

#[derive(Clone, Debug)]
pub struct AllTime {
    pub medals_listing: Vec<(UserId, MedalsEntry)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MedalsEntry {
    gold: usize,
    silver: usize,
    bronze: usize,
}

impl MedalsEntry {
    fn score(&self) -> usize {
        const GOLD_WEIGHT: usize = 4;
        const SILVER_WEIGHT: usize = 2;
        const BRONZE_WEIGHT: usize = 1;

        self.gold * GOLD_WEIGHT + self.silver * SILVER_WEIGHT + self.bronze * BRONZE_WEIGHT
    }
}

impl fmt::Display for MedalsEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "🥇{} 🥈{} 🥉{} (score: {})",
            self.gold,
            self.silver,
            self.bronze,
            self.score()
        )
    }
}

impl PartialEq for MedalsEntry {
    fn eq(&self, other: &Self) -> bool {
        self.gold == other.gold && self.silver == other.silver && self.bronze == other.bronze
    }
}

impl Eq for MedalsEntry {}

impl PartialOrd for MedalsEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MedalsEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score().cmp(&other.score())
    }
}

#[derive(Clone, Debug, FromRow)]
struct AllTimeQueryRow {
    user_id: i64,
    place: i64,
}

#[derive(Debug, Error)]
pub enum CalculateAllTimeError {
    #[error("failed to extract data from row: {0}")]
    FromRow(#[source] SqlxError),

    #[error("unexpected SQLx error: {0}")]
    Unexpected(SqlxError),

    #[error("unexpectedly received out-of-bounds place value from query: {0}")]
    PlaceOutOfBounds(i64),
}

impl AllTime {
    pub async fn calculate(
        db_pool: &PgPool,
        guild_id: GuildId,
        end_day: usize,
        include_end: bool,
        include_late: bool,
    ) -> Result<Self, CalculateAllTimeError> {
        let board_clause = if include_end {
            "AND s.board <= $2"
        } else {
            "AND s.board < $2"
        };

        let late_clause = if include_late {
            ""
        } else {
            "AND s.board = s.day_added"
        };

        let get_scores_string = formatdoc!(
            "
            WITH cte AS (
                SELECT
                    s.user_id,
                    ROW_NUMBER() OVER (
                        PARTITION BY s.board
                        ORDER BY s.score ASC
                    ) as place
                FROM
                    scores s
                    INNER JOIN users u USING (user_id)
                WHERE
                    s.guild_id = $1
                    {}
                    {}
            )
            SELECT
                user_id,
                place
            FROM cte
            WHERE place <= 3
            ORDER BY place;
            ",
            board_clause,
            late_clause
        );
        let get_scores = sqlx::query(get_scores_string.as_ref());
        let medals = match get_scores
            .bind(guild_id.get() as i64)
            .bind(end_day as i32)
            .fetch_all(db_pool)
            .await
        {
            Ok(rows) => {
                info!(num = %rows.len(), "fetched all scores");

                let rows = rows
                    .into_iter()
                    .map(|row| {
                        AllTimeQueryRow::from_row(&row).map_err(CalculateAllTimeError::FromRow)
                    })
                    .collect::<Result<Vec<_>, CalculateAllTimeError>>()?;

                let mut medals = HashMap::<UserId, MedalsEntry>::default();

                for row in rows {
                    debug!(?row, "got row");

                    let id = UserId::new(row.user_id as u64);
                    let entry = medals.entry(id).or_insert_with(MedalsEntry::default);

                    match row.place {
                        1 => entry.gold += 1,
                        2 => entry.silver += 1,
                        3 => entry.bronze += 1,
                        _ => return Err(CalculateAllTimeError::PlaceOutOfBounds(row.place)),
                    }
                }

                medals
            }
            Err(error) => {
                error!(%error, "failed to fetch all scores");
                return Err(CalculateAllTimeError::Unexpected(error));
            }
        };

        info!(?medals, "medals table");

        let mut medals_listing: Vec<_> = medals.into_iter().collect();
        medals_listing.sort_by_key(|ml| ml.1);
        medals_listing.reverse();

        info!(?medals_listing, "medals listing");

        Ok(AllTime { medals_listing })
    }
}
